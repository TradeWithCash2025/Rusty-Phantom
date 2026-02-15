//! Session type definitions for OAuth callbacks, tokens, DCR config, and session data.

use serde::{Deserialize, Serialize};

/// SSO callback parameters received from connect.phantom.app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCallbackParams {
    pub session_id: String,
    pub wallet_id: String,
    pub organization_id: String,
    pub auth_user_id: String,
}

/// OAuth tokens (not used in SSO flow, kept for compatibility).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

/// Dynamic Client Registration (DCR) client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DCRClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub client_id_issued_at: u64,
}

/// Complete session data stored on disk.
///
/// Note: SSO flow uses stamper keys for API authentication, not OAuth tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    #[serde(rename = "walletId")]
    pub wallet_id: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(rename = "authUserId")]
    pub auth_user_id: String,
    #[serde(rename = "stamperKeys")]
    pub stamper_keys: StamperKeys,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

/// Stamper key pair for API authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StamperKeys {
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "secretKey")]
    pub secret_key: String,
}
