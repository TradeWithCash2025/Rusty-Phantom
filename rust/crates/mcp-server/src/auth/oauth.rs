//! OAuth 2.0 authorization flow with PKCE.
//!
//! Orchestrates DCR, browser opening, callback server, and token exchange.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use serde::Deserialize;

use crate::auth::callback_server::{CallbackServer, CallbackServerOptions};
use crate::auth::dcr::DCRClient;
use crate::session::types::{DCRClientConfig, OAuthTokens, StamperKeys};
use crate::utils::logger::Logger;

/// OAuth 2.0 token endpoint response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

/// Result of a successful SSO flow.
pub struct OAuthFlowResult {
    pub wallet_id: String,
    pub organization_id: String,
    pub auth_user_id: String,
    pub client_config: DCRClientConfig,
    pub stamper_keys: StamperKeys,
}

/// Options for configuring the OAuth flow.
pub struct OAuthFlowOptions {
    pub auth_base_url: Option<String>,
    pub connect_base_url: Option<String>,
    pub callback_port: Option<u16>,
    pub callback_path: Option<String>,
    pub app_id: Option<String>,
    pub provider: Option<String>,
}

/// Orchestrates the complete OAuth 2.0 SSO authorization flow.
pub struct OAuthFlow {
    auth_base_url: String,
    connect_base_url: String,
    callback_port: u16,
    callback_path: String,
    app_id: String,
    provider: String,
    logger: Logger,
}

impl OAuthFlow {
    /// Create a new OAuth flow.
    pub fn new(
        auth_base_url: Option<String>,
        connect_base_url: Option<String>,
        callback_port: Option<u16>,
        callback_path: Option<String>,
        app_id: Option<String>,
        provider: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let auth_base_url = auth_base_url
            .or_else(|| std::env::var("PHANTOM_AUTH_BASE_URL").ok())
            .unwrap_or_else(|| "https://auth.phantom.app".to_string());

        let connect_base_url = connect_base_url
            .or_else(|| std::env::var("PHANTOM_CONNECT_BASE_URL").ok())
            .unwrap_or_else(|| "https://connect.phantom.app".to_string());

        let callback_port = if let Some(port) = callback_port {
            if port == 0 {
                return Err(format!("Invalid callbackPort: \"{}\". Must be between 1 and 65535.", port).into());
            }
            port
        } else {
            std::env::var("PHANTOM_CALLBACK_PORT")
                .ok()
                .and_then(|s| s.trim().parse::<u16>().ok())
                .filter(|&p| p > 0)
                .unwrap_or(8080)
        };

        let callback_path = callback_path
            .or_else(|| std::env::var("PHANTOM_CALLBACK_PATH").ok())
            .unwrap_or_else(|| "/callback".to_string());

        let app_id = app_id.unwrap_or_else(|| "phantom-mcp".to_string());

        let provider = provider
            .or_else(|| std::env::var("PHANTOM_SSO_PROVIDER").ok())
            .unwrap_or_else(|| "google".to_string());

        if !["google", "apple", "phantom"].contains(&provider.as_str()) {
            return Err(format!("Unsupported SSO provider: {}", provider).into());
        }

        Ok(Self {
            auth_base_url,
            connect_base_url,
            callback_port,
            callback_path,
            app_id,
            provider,
            logger: Logger::new("OAuthFlow"),
        })
    }

    /// Execute the complete SSO authentication flow.
    pub async fn authenticate(
        &self,
    ) -> Result<OAuthFlowResult, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Starting SSO authentication flow");

        // Start callback server
        let callback_server = CallbackServer::new(CallbackServerOptions {
            port: self.callback_port,
            path: self.callback_path.clone(),
            ..Default::default()
        });
        let redirect_uri = callback_server.get_callback_url();

        // Step 1: Get OAuth client credentials (from env or DCR)
        let env_client_id = std::env::var("PHANTOM_APP_ID")
            .or_else(|_| std::env::var("PHANTOM_CLIENT_ID"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let env_client_secret = std::env::var("PHANTOM_CLIENT_SECRET")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let client_config = if let Some(client_id) = env_client_id {
            self.logger
                .info("Step 1: Using client credentials from environment variables");
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            DCRClientConfig {
                client_id,
                client_secret: env_client_secret.unwrap_or_default(),
                client_id_issued_at: now,
            }
        } else {
            self.logger
                .info("Step 1: Registering OAuth client via DCR");
            self.logger.warn(
                "DCR is not currently supported by auth.phantom.app - \
                 you should provide PHANTOM_APP_ID or PHANTOM_CLIENT_ID",
            );
            let dcr_client = DCRClient::new(&self.auth_base_url, &self.app_id);
            dcr_client.register(&redirect_uri).await?
        };

        // Step 2: Generate stamper keypair
        self.logger.info("Step 2: Generating stamper keypair");
        let stamper_keys = phantom_crypto::generate_key_pair();
        self.logger
            .info(&format!("Stamper public key: {}", stamper_keys.public_key));

        // Step 3: Generate session ID
        self.logger.info("Step 3: Generating session ID");
        let session_id = self.generate_session_id();

        // Step 4: Build SSO authorization URL
        self.logger.info("Step 4: Building SSO authorization URL");
        let auth_url = self.build_authorization_url(
            &client_config.client_id,
            &redirect_uri,
            &stamper_keys.public_key,
            &session_id,
        );

        // Step 5: Start callback server before opening browser
        self.logger.info("Step 5: Starting callback server");
        let callback_future = callback_server.start_and_wait_for_callback(&session_id);

        // Step 6: Open browser
        self.logger
            .info(&format!("Step 6: Opening browser for {} authentication", self.provider));
        // Print the URL so the user can open it manually if needed
        self.logger
            .info("Please open the following URL in your browser to complete authentication:");
        self.logger.info(&auth_url);

        // Step 7: Wait for callback
        self.logger.info("Step 7: Waiting for SSO callback");
        let callback_params = callback_future.await?;
        self.logger.info("Callback received successfully");

        Ok(OAuthFlowResult {
            wallet_id: callback_params.wallet_id,
            organization_id: callback_params.organization_id,
            auth_user_id: callback_params.auth_user_id,
            client_config,
            stamper_keys: StamperKeys {
                public_key: stamper_keys.public_key,
                secret_key: stamper_keys.secret_key,
            },
        })
    }

    /// Refreshes an access token using a refresh token.
    ///
    /// Note: Not used in SSO flow, kept for future OAuth compatibility.
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        client_config: &DCRClientConfig,
    ) -> Result<OAuthTokens, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info("Refreshing access token");

        let token_endpoint = format!("{}/oauth2/token", self.auth_base_url);
        let is_public_client = client_config.client_secret.is_empty();

        // Build request parameters
        let mut params = vec![
            ("grant_type", "refresh_token".to_string()),
            ("refresh_token", refresh_token.to_string()),
        ];

        // For public clients, send client_id in body
        if is_public_client {
            params.push(("client_id", client_config.client_id.clone()));
        }

        // Build request
        let http_client = reqwest::Client::new();
        let mut request_builder = http_client
            .post(&token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .timeout(std::time::Duration::from_secs(30))
            .form(&params);

        // For confidential clients, use HTTP Basic Auth
        if !is_public_client {
            request_builder = request_builder.basic_auth(
                &client_config.client_id,
                Some(&client_config.client_secret),
            );
        }

        let response = request_builder.send().await.map_err(|e| {
            let msg = format!("Token refresh failed: {}", e);
            self.logger.error(&msg);
            msg
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let msg = format!("Token refresh failed ({}): {}", status, body);
            self.logger.error(&msg);
            return Err(msg.into());
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            let msg = format!("Token refresh failed: invalid response: {}", e);
            self.logger.error(&msg);
            msg
        })?;

        self.logger.info("Token refresh successful");

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
        })
    }

    /// Generate a random session ID for SSO flow.
    fn generate_session_id(&self) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Build the SSO authorization URL.
    fn build_authorization_url(
        &self,
        app_id: &str,
        redirect_uri: &str,
        public_key: &str,
        session_id: &str,
    ) -> String {
        let params = [
            ("provider", self.provider.as_str()),
            ("app_id", app_id),
            ("redirect_uri", redirect_uri),
            ("public_key", public_key),
            ("session_id", session_id),
            ("sdk_version", "1.0.0"),
            ("sdk_type", "mcp-server"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}/login?{}", self.connect_base_url, query)
    }
}

/// Simple URL encoding for query parameters.
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
