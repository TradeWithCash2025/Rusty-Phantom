//! Core types for the browser SDK.

use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::{
    EmbeddedProviderAuthType, WalletAddress,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::debug::{DebugCallback, DebugLevel};

/// Authentication provider type — extends embedded auth types with "injected" and "deeplink".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthProviderType {
    /// Google OAuth.
    Google,
    /// Apple OAuth.
    Apple,
    /// Phantom app authentication.
    Phantom,
    /// Device-based authentication.
    Device,
    /// Injected browser extension provider.
    Injected,
    /// Mobile deeplink provider.
    Deeplink,
}

impl From<EmbeddedProviderAuthType> for AuthProviderType {
    fn from(val: EmbeddedProviderAuthType) -> Self {
        match val {
            EmbeddedProviderAuthType::Google => AuthProviderType::Google,
            EmbeddedProviderAuthType::Apple => AuthProviderType::Apple,
            EmbeddedProviderAuthType::Phantom => AuthProviderType::Phantom,
            EmbeddedProviderAuthType::Device => AuthProviderType::Device,
        }
    }
}

/// Authentication options for connecting to a wallet.
#[derive(Debug, Clone)]
pub struct AuthOptions {
    /// Authentication provider to use.
    pub provider: AuthProviderType,
    /// Wallet ID for injected wallet selection.
    pub wallet_id: Option<String>,
    /// Custom authentication data.
    pub custom_auth_data: Option<HashMap<String, serde_json::Value>>,
}

/// Debug configuration.
pub struct DebugConfig {
    pub enabled: Option<bool>,
    pub level: Option<DebugLevel>,
    pub callback: Option<DebugCallback>,
}

/// Browser SDK configuration.
#[derive(Debug, Clone)]
pub struct BrowserSdkConfig {
    /// Allowed authentication providers (required).
    pub providers: Vec<AuthProviderType>,
    /// Application ID (required when using embedded providers).
    pub app_id: Option<String>,
    /// API base URL.
    pub api_base_url: Option<String>,
    /// Embedded wallet type.
    pub embedded_wallet_type: Option<String>,
    /// Authentication URL options.
    pub auth_options: Option<AuthUrlOptions>,
    /// Address types to enable.
    pub address_types: Vec<AddressFormat>,
}

/// Authentication URL options.
#[derive(Debug, Clone)]
pub struct AuthUrlOptions {
    pub auth_url: Option<String>,
    pub redirect_url: Option<String>,
}

/// Result of a connection.
#[derive(Debug, Clone)]
pub struct ConnectResult {
    /// Connected wallet addresses.
    pub addresses: Vec<WalletAddress>,
    /// Wallet ID.
    pub wallet_id: Option<String>,
    /// Auth user ID.
    pub auth_user_id: Option<String>,
    /// Authentication provider used.
    pub auth_provider: Option<AuthProviderType>,
    /// Connection status.
    pub status: Option<ConnectStatus>,
    /// Wallet info (only for injected providers).
    pub wallet: Option<ConnectResultWalletInfo>,
}

/// Connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectStatus {
    Pending,
    Completed,
}

/// Wallet info included in connect results.
#[derive(Debug, Clone)]
pub struct ConnectResultWalletInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub address_types: Vec<AddressFormat>,
    pub rdns: Option<String>,
    pub discovery: Option<String>,
}

/// Provider trait — common interface for embedded and injected providers.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Connect with the given auth options.
    async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get all connected wallet addresses.
    fn get_addresses(&self) -> Vec<WalletAddress>;

    /// Whether the provider is connected.
    fn is_connected(&self) -> bool;

    /// Attempt auto-connection.
    async fn auto_connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get enabled address types.
    fn get_enabled_address_types(&self) -> Vec<AddressFormat>;
}
