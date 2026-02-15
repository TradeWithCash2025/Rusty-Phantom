//! Core types for the embedded provider.

use phantom_parsers::{ParsedSignatureResult, ParsedTransactionResult};
use serde::{Deserialize, Serialize};

use crate::constants::AddressFormat;

/// Wallet address with chain type and address string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    /// Address type (e.g., "solana", "ethereum").
    pub address_type: AddressFormat,
    /// The blockchain address string.
    pub address: String,
}

/// Result of a successful connection.
#[derive(Debug, Clone)]
pub struct ConnectResult {
    /// Wallet ID (only for embedded).
    pub wallet_id: Option<String>,
    /// List of addresses for the connected wallet.
    pub addresses: Vec<WalletAddress>,
    /// Session status: pending (redirect in progress) or completed.
    pub status: Option<ConnectStatus>,
    /// Phantom user ID from auth flow (for embedded user-wallets).
    pub auth_user_id: Option<String>,
    /// Auth provider used for authentication.
    pub auth_provider: EmbeddedProviderAuthType,
}

/// Connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectStatus {
    /// Redirect in progress.
    Pending,
    /// Wallet is ready.
    Completed,
}

/// Parameters for signing a message.
#[derive(Debug, Clone)]
pub struct SignMessageParams {
    /// Message to sign.
    pub message: String,
    /// Network to sign on.
    pub network_id: String,
}

/// Parameters for signing EIP-712 typed data (v4).
#[derive(Debug, Clone)]
pub struct SignTypedDataV4Params {
    /// EIP-712 typed data object.
    pub typed_data: serde_json::Value,
    /// Network to sign on.
    pub network_id: String,
}

/// Result of signing a message (alias for ParsedSignatureResult).
pub type SignMessageResult = ParsedSignatureResult;

/// Parameters for signing a transaction.
#[derive(Debug, Clone)]
pub struct SignTransactionParams {
    /// Transaction data (serialized bytes).
    pub transaction: Vec<u8>,
    /// Network to sign on.
    pub network_id: String,
}

/// Parameters for signing and sending a transaction.
#[derive(Debug, Clone)]
pub struct SignAndSendTransactionParams {
    /// Transaction data (serialized bytes).
    pub transaction: Vec<u8>,
    /// Network to sign on.
    pub network_id: String,
}

/// Result of a signed transaction (alias for ParsedTransactionResult).
pub type SignedTransaction = ParsedTransactionResult;

/// Event data emitted when connection starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectStartEventData {
    pub provider: EmbeddedProviderAuthType,
}

/// Event data emitted on successful connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectEventData {
    pub wallet_id: String,
    pub addresses: Vec<WalletAddress>,
    pub auth_provider: EmbeddedProviderAuthType,
    pub auth_user_id: Option<String>,
}

/// Event data emitted on connection error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectErrorEventData {
    pub error: String,
    pub provider: Option<EmbeddedProviderAuthType>,
}

/// Event data emitted on disconnect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectEventData {
    pub wallet_id: Option<String>,
}

/// Authentication provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedProviderAuthType {
    /// Google OAuth.
    Google,
    /// Apple OAuth.
    Apple,
    /// Phantom app authentication.
    Phantom,
    /// Device-based authentication (for app-wallets).
    Device,
}

/// Authentication options.
#[derive(Debug, Clone)]
pub struct AuthOptions {
    /// Authentication provider to use.
    pub provider: EmbeddedProviderAuthType,
    /// Custom authentication data.
    pub custom_auth_data: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Configuration for the embedded provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedProviderConfig {
    /// Base URL for the Phantom wallet API.
    pub api_base_url: String,
    /// Application ID.
    pub app_id: String,
    /// Authentication options.
    pub auth_options: AuthUrlOptions,
    /// Wallet type.
    pub embedded_wallet_type: String,
    /// Enabled address types.
    pub address_types: Vec<AddressFormat>,
}

/// Authentication URL configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUrlOptions {
    /// Authentication server URL.
    pub auth_url: String,
    /// Redirect URL after authentication.
    pub redirect_url: String,
}
