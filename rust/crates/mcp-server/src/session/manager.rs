//! SessionManager orchestrates the complete session lifecycle:
//! - Loads existing sessions from storage
//! - Handles authentication when needed
//! - Creates and manages PhantomClient instances
//! - Provides session data access

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_client::{PhantomClient, PhantomClientConfig};
use std::sync::Arc;

use super::storage::SessionStorage;
use super::types::SessionData;
use crate::auth::oauth::OAuthFlow;
use crate::utils::logger::Logger;

/// Configuration options for SessionManager.
pub struct SessionManagerOptions {
    /// Base URL for OAuth authorization server.
    pub auth_base_url: Option<String>,
    /// Base URL for Phantom Connect.
    pub connect_base_url: Option<String>,
    /// Base URL for Phantom API.
    pub api_base_url: Option<String>,
    /// Port for local OAuth callback server.
    pub callback_port: Option<u16>,
    /// Path for OAuth callback.
    pub callback_path: Option<String>,
    /// Application identifier prefix.
    pub app_id: Option<String>,
    /// Directory to store session data.
    pub session_dir: Option<String>,
}

impl Default for SessionManagerOptions {
    fn default() -> Self {
        Self {
            auth_base_url: None,
            connect_base_url: None,
            api_base_url: None,
            callback_port: None,
            callback_path: None,
            app_id: None,
            session_dir: None,
        }
    }
}

/// SessionManager handles session lifecycle, auto-authentication, and PhantomClient creation.
pub struct SessionManager {
    auth_base_url: String,
    connect_base_url: Option<String>,
    api_base_url: String,
    callback_port: u16,
    callback_path: String,
    app_id: String,
    storage: SessionStorage,
    logger: Logger,
    session: Option<SessionData>,
    client: Option<PhantomClient>,
}

impl SessionManager {
    /// Create a new SessionManager.
    pub fn new(options: SessionManagerOptions) -> Self {
        let logger = Logger::new("SessionManager");

        let auth_base_url = options
            .auth_base_url
            .or_else(|| std::env::var("PHANTOM_AUTH_BASE_URL").ok())
            .unwrap_or_else(|| "https://auth.phantom.app".to_string());

        let connect_base_url = options.connect_base_url;

        let api_base_url = options
            .api_base_url
            .or_else(|| std::env::var("PHANTOM_API_BASE_URL").ok())
            .unwrap_or_else(|| "https://api.phantom.app/v1/wallets".to_string());

        let callback_port = if let Some(port) = options.callback_port {
            port
        } else {
            std::env::var("PHANTOM_CALLBACK_PORT")
                .ok()
                .and_then(|s| s.trim().parse::<u16>().ok())
                .filter(|&p| p > 0)
                .unwrap_or(8080)
        };

        let callback_path = options
            .callback_path
            .or_else(|| std::env::var("PHANTOM_CALLBACK_PATH").ok())
            .unwrap_or_else(|| "/callback".to_string());

        let app_id = options
            .app_id
            .unwrap_or_else(|| "phantom-mcp".to_string());

        let storage = SessionStorage::new(options.session_dir.as_deref());

        Self {
            auth_base_url,
            connect_base_url,
            api_base_url,
            callback_port,
            callback_path,
            app_id,
            storage,
            logger,
            session: None,
            client: None,
        }
    }

    /// Initialize the session manager.
    /// Loads existing session or authenticates if needed.
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Initializing session manager");

        // Step 1: Try to load existing session
        let existing_session = self.storage.load();

        // Step 2: Check if session is valid
        if let Some(ref session) = existing_session {
            if !self.storage.is_expired(session) {
                self.logger.info("Loaded valid session from storage");
                self.session = existing_session;
                self.create_client()?;
                return Ok(());
            }
            self.logger.info("Session expired, re-authenticating");
        } else {
            self.logger.info("No session found, authenticating");
        }

        // Step 3: Authenticate
        self.authenticate().await
    }

    /// Returns the initialized PhantomClient.
    pub fn get_client(&self) -> &PhantomClient {
        self.client
            .as_ref()
            .expect("SessionManager not initialized. Call initialize() first.")
    }

    /// Returns the current session data.
    pub fn get_session(&self) -> &SessionData {
        self.session
            .as_ref()
            .expect("SessionManager not initialized. Call initialize() first.")
    }

    /// Resets the session by clearing stored data and re-authenticating.
    pub async fn reset_session(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Resetting session");
        self.storage.delete();
        self.session = None;
        self.client = None;
        self.authenticate().await
    }

    /// Execute the SSO flow and create a new session.
    async fn authenticate(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting authentication");

        let oauth_flow = OAuthFlow::new(
            Some(self.auth_base_url.clone()),
            self.connect_base_url.clone(),
            Some(self.callback_port),
            Some(self.callback_path.clone()),
            Some(self.app_id.clone()),
            None,
        )?;

        let oauth_result = oauth_flow.authenticate().await?;
        self.logger.info("SSO flow completed successfully");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.session = Some(SessionData {
            wallet_id: oauth_result.wallet_id,
            organization_id: oauth_result.organization_id,
            auth_user_id: oauth_result.auth_user_id,
            stamper_keys: oauth_result.stamper_keys,
            created_at: now,
            updated_at: now,
        });

        self.storage
            .save(self.session.as_ref().unwrap())
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to save session: {}", e).into()
            })?;
        self.logger.info("Session saved to storage");

        self.create_client()?;
        Ok(())
    }

    /// Create a PhantomClient instance from the current session.
    fn create_client(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .session
            .as_ref()
            .ok_or("Cannot create client without session")?;

        self.logger.info("Creating PhantomClient");

        let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
            api_secret_key: session.stamper_keys.secret_key.clone(),
        })?;

        let app_id = std::env::var("PHANTOM_APP_ID")
            .or_else(|_| std::env::var("PHANTOM_CLIENT_ID"))
            .unwrap_or_else(|_| self.app_id.clone());

        let mut headers = std::collections::HashMap::new();
        headers.insert("X-App-Id".to_string(), app_id);

        let config = PhantomClientConfig {
            api_base_url: self.api_base_url.clone(),
            organization_id: Some(session.organization_id.clone()),
            wallet_type: "user-wallet".to_string(),
            headers: Some(headers),
        };

        self.client = Some(PhantomClient::new(config, Some(Arc::new(stamper))));

        self.logger.info("PhantomClient created successfully");
        Ok(())
    }
}
