//! Ethereum-specific types for the browser-injected SDK.

use serde::{Deserialize, Serialize};

/// Ethereum transaction request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EthereumTransaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tx_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
}

/// Ethereum Sign-In With data (EIP-4361 / SIWE).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumSignInData {
    /// Ethereum address (0x-prefixed).
    pub address: String,
    /// EIP-155 chain ID.
    pub chain_id: i64,
    /// RFC 3986 authority domain.
    pub domain: String,
    /// Nonce (at least 8 alphanumeric characters).
    pub nonce: String,
    /// RFC 3986 URI.
    pub uri: String,
    /// SIWE version (must be "1").
    pub version: String,
    /// Optional URL scheme.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// Optional statement (must not contain newlines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
    /// Optional request ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Optional resource URIs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<String>>,
    /// Issued at time (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<String>,
    /// Expiration time (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time: Option<String>,
    /// Not before time (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,
}

/// Ethereum event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EthereumEventType {
    Connect,
    Disconnect,
    AccountsChanged,
    ChainChanged,
}

/// EIP-1193 provider RPC error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Result of an Ethereum sign-in.
#[derive(Debug, Clone)]
pub struct EthereumSignInResult {
    pub address: String,
    pub signature: String,
    pub signed_message: String,
}
