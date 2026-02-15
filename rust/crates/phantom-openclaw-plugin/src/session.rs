//! Session management for the Phantom OpenClaw plugin.
//!
//! Wraps the SessionManager from phantom-mcp-server.

use phantom_client::PhantomClient;
use phantom_mcp_server::session::manager::{SessionManager, SessionManagerOptions};
use phantom_mcp_server::session::types::SessionData;
use tokio::sync::Mutex;

/// Configuration options for PluginSession.
pub struct PluginSessionOptions {
    /// Application identifier from Phantom Portal.
    pub app_id: Option<String>,
    /// OAuth callback port (default: 8080).
    pub callback_port: Option<u16>,
    /// Directory to store session data.
    pub session_dir: Option<String>,
}

impl Default for PluginSessionOptions {
    fn default() -> Self {
        Self {
            app_id: None,
            callback_port: None,
            session_dir: None,
        }
    }
}

/// Plugin session manager.
///
/// Handles authentication and provides access to PhantomClient.
pub struct PluginSession {
    session_manager: Mutex<SessionManager>,
    initialized: std::sync::atomic::AtomicBool,
    init_lock: Mutex<()>,
}

impl PluginSession {
    /// Create a new plugin session.
    pub fn new(options: PluginSessionOptions) -> Self {
        let session_manager = SessionManager::new(SessionManagerOptions {
            app_id: Some(options.app_id.unwrap_or_else(|| "phantom-openclaw".to_string())),
            callback_port: options.callback_port,
            session_dir: options.session_dir,
            ..Default::default()
        });

        Self {
            session_manager: Mutex::new(session_manager),
            initialized: std::sync::atomic::AtomicBool::new(false),
            init_lock: Mutex::new(()),
        }
    }

    /// Initialize the session (authenticate if needed).
    /// Thread-safe: concurrent calls will await the same initialization.
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        // Acquire init lock to prevent concurrent initialization
        let _guard = self.init_lock.lock().await;

        // Double-check after acquiring lock
        if self.initialized.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }

        let mut manager = self.session_manager.lock().await;
        manager.initialize().await?;
        self.initialized
            .store(true, std::sync::atomic::Ordering::Release);

        Ok(())
    }

    /// Get the authenticated PhantomClient (cloned).
    pub async fn get_client(&self) -> PhantomClient {
        assert!(
            self.initialized.load(std::sync::atomic::Ordering::Acquire),
            "Session not initialized. Call initialize() first."
        );
        let manager = self.session_manager.lock().await;
        manager.get_client().clone()
    }

    /// Get the current session data (cloned).
    pub async fn get_session(&self) -> SessionData {
        assert!(
            self.initialized.load(std::sync::atomic::Ordering::Acquire),
            "Session not initialized. Call initialize() first."
        );
        let manager = self.session_manager.lock().await;
        manager.get_session().clone()
    }
}
