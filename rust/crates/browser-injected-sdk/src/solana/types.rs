//! Solana-specific types for the browser-injected SDK.

use serde::{Deserialize, Serialize};

/// Options for sending transactions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SendOptions {
    /// Skip preflight transaction checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_preflight: Option<bool>,
    /// Preflight commitment level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight_commitment: Option<String>,
    /// Maximum number of retries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    /// Minimum context slot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_context_slot: Option<u64>,
}

/// Solana Sign-In With data (following the SIWS spec).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolanaSignInData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<String>>,
}

/// Display encoding for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayEncoding {
    Utf8,
    Hex,
}

/// Phantom Solana event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PhantomEventType {
    Connect,
    Disconnect,
    AccountChanged,
}

/// Result of signing a message.
#[derive(Debug, Clone)]
pub struct SignMessageResult {
    /// The signature bytes.
    pub signature: Vec<u8>,
    /// The address (public key) that signed.
    pub address: String,
}

/// Result of signing and sending a transaction.
#[derive(Debug, Clone)]
pub struct SignAndSendResult {
    /// The transaction signature.
    pub signature: String,
    /// The address that signed (optional).
    pub address: Option<String>,
}

/// Result of signing and sending all transactions.
#[derive(Debug, Clone)]
pub struct SignAndSendAllResult {
    /// The transaction signatures.
    pub signatures: Vec<String>,
    /// The address that signed (optional).
    pub address: Option<String>,
}

/// Result of a sign-in operation.
#[derive(Debug, Clone)]
pub struct SignInResult {
    /// The address that signed in.
    pub address: String,
    /// The signature bytes.
    pub signature: Vec<u8>,
    /// The signed message bytes.
    pub signed_message: Vec<u8>,
}

/// Connect options.
#[derive(Debug, Clone, Default)]
pub struct ConnectOptions {
    /// Only connect if trusted (previously approved).
    pub only_if_trusted: bool,
}
