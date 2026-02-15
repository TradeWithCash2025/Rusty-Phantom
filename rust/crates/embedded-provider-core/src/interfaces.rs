//! Platform abstraction interfaces for the embedded provider.
//!
//! These traits define the platform-specific capabilities that must be
//! implemented for each target environment (web, iOS, Android, React Native).

use crate::types::EmbeddedProviderAuthType;
use phantom_sdk_types::{Algorithm, StamperWithKeyManagement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Auth interfaces
// ============================================================================

/// Result of an authentication operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// Wallet ID from the auth flow.
    pub wallet_id: String,
    /// Organization ID from the auth flow.
    pub organization_id: String,
    /// Auth provider used.
    pub provider: EmbeddedProviderAuthType,
    /// Account derivation index from the auth response.
    pub account_derivation_index: u32,
    /// Authenticator expiration time in milliseconds.
    pub expires_in_ms: u64,
    /// User ID from the auth flow (optional, for user-wallets).
    pub auth_user_id: Option<String>,
}

/// Options for Phantom Connect authentication flow.
#[derive(Debug, Clone)]
pub struct PhantomConnectOptions {
    /// Base64url encoded public key.
    pub public_key: String,
    /// Application ID.
    pub app_id: String,
    /// Auth provider.
    pub provider: EmbeddedProviderAuthType,
    /// Redirect URL after auth.
    pub redirect_url: Option<String>,
    /// Auth server URL.
    pub auth_url: Option<String>,
    /// Session ID.
    pub session_id: String,
    /// Whether to clear previous OAuth session.
    pub clear_previous_session: Option<bool>,
    /// Whether to allow OAuth session refresh.
    pub allow_refresh: Option<bool>,
    /// Signing algorithm.
    pub algorithm: Option<Algorithm>,
}

/// Auth provider trait for handling authentication flows.
#[async_trait::async_trait]
pub trait AuthProvider: Send + Sync {
    /// Authenticate using the given options.
    /// Returns None if a redirect is in progress.
    async fn authenticate(&self, options: PhantomConnectOptions)
        -> Result<Option<AuthResult>, Box<dyn std::error::Error + Send + Sync>>;

    /// Resume authentication from a redirect (if applicable).
    fn resume_auth_from_redirect(
        &self,
        provider: EmbeddedProviderAuthType,
    ) -> Option<AuthResult>;
}

/// Options for Phantom app authentication.
#[derive(Debug, Clone)]
pub struct PhantomAppAuthOptions {
    /// Public key.
    pub public_key: String,
    /// Application ID.
    pub app_id: String,
    /// Session ID.
    pub session_id: String,
}

/// Phantom app provider trait (browser extension or mobile app).
#[async_trait::async_trait]
pub trait PhantomAppProvider: Send + Sync {
    /// Authenticate using the Phantom app.
    async fn authenticate(
        &self,
        options: PhantomAppAuthOptions,
    ) -> Result<AuthResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Check if the Phantom app is available.
    fn is_available(&self) -> bool;
}

// ============================================================================
// Storage interfaces
// ============================================================================

/// Keypair with public and secret keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypair {
    /// Base58 encoded public key.
    pub public_key: String,
    /// Base58 encoded secret key.
    pub secret_key: String,
}

/// Stamper identification information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StamperInfo {
    /// Key identifier.
    pub key_id: String,
    /// Base58 encoded public key.
    pub public_key: String,
    /// Optional timestamp when key was created.
    pub created_at: Option<u64>,
    /// Optional authenticator ID from server.
    pub authenticator_id: Option<String>,
}

/// Session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Authentication in progress (redirect flow).
    Pending,
    /// Session is fully established.
    Completed,
    /// Session creation failed.
    Failed,
}

/// Complete session object with authenticator lifecycle tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub session_id: String,
    /// Wallet ID.
    pub wallet_id: String,
    /// Organization ID from auth flow.
    pub organization_id: String,
    /// Application ID.
    pub app_id: String,
    /// Stamper key information.
    pub stamper_info: StamperInfo,
    /// Keypair (for backward compatibility).
    pub keypair: Option<Keypair>,
    /// Auth provider used for authentication.
    pub auth_provider: EmbeddedProviderAuthType,
    /// Session status.
    pub status: SessionStatus,
    /// Creation timestamp (ms).
    pub created_at: u64,
    /// Last used timestamp (ms).
    pub last_used: u64,
    /// When the current authenticator was created (ms).
    pub authenticator_created_at: u64,
    /// When the authenticator expires (ms).
    pub authenticator_expires_at: u64,
    /// Last time we attempted renewal (ms).
    pub last_renewal_attempt: Option<u64>,
    /// Account derivation index from auth flow.
    pub account_derivation_index: Option<u32>,
    /// User ID from auth flow (for user-wallets).
    pub auth_user_id: Option<String>,
}

/// Storage interface for session management.
#[async_trait::async_trait]
pub trait EmbeddedStorage: Send + Sync {
    /// Get the current session.
    async fn get_session(&self) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>>;
    /// Save a session.
    async fn save_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    /// Clear the current session.
    async fn clear_session(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get whether to clear previous OAuth session.
    async fn get_should_clear_previous_session(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    /// Set whether to clear previous OAuth session.
    async fn set_should_clear_previous_session(
        &self,
        should: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// URL params interface
// ============================================================================

/// URL parameter accessor abstraction.
///
/// Abstracts URL parameter access for different environments:
/// - Browser: window.location.search via URLSearchParams
/// - React Native: deep links or webview callback URLs
pub trait UrlParamsAccessor: Send + Sync {
    /// Get a URL parameter value by key.
    fn get_param(&self, key: &str) -> Option<String>;
}

// ============================================================================
// Platform adapter interface
// ============================================================================

/// Platform adapter combining all platform-specific capabilities.
pub trait PlatformAdapter: Send + Sync {
    /// Platform identifier (e.g., "web", "ios", "android", "react-native").
    fn name(&self) -> &str;

    /// Storage implementation.
    fn storage(&self) -> &dyn EmbeddedStorage;

    /// Authentication provider.
    fn auth_provider(&self) -> &dyn AuthProvider;

    /// Phantom app provider.
    fn phantom_app_provider(&self) -> &dyn PhantomAppProvider;

    /// URL parameters accessor.
    fn url_params_accessor(&self) -> &dyn UrlParamsAccessor;

    /// Stamper with key management.
    fn stamper(&self) -> &dyn StamperWithKeyManagement;

    /// Optional analytics headers.
    fn analytics_headers(&self) -> Option<HashMap<String, String>> {
        None
    }
}

// ============================================================================
// Debug logger interface
// ============================================================================

/// Debug logging interface.
pub trait DebugLogger: Send + Sync {
    /// Log an informational message.
    fn info(&self, category: &str, message: &str, data: Option<&serde_json::Value>);
    /// Log a warning message.
    fn warn(&self, category: &str, message: &str, data: Option<&serde_json::Value>);
    /// Log an error message.
    fn error(&self, category: &str, message: &str, data: Option<&serde_json::Value>);
    /// Log a debug message.
    fn log(&self, category: &str, message: &str, data: Option<&serde_json::Value>);
}
