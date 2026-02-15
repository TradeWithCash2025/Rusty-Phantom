//! Type definitions for the Phantom client.
//!
//! Mirrors the TypeScript types from `packages/client/src/types.ts`.

use serde::{Deserialize, Serialize};

use crate::constants::{AddressFormat, ClientAlgorithm, Curve};

/// Configuration for creating a PhantomClient instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomClientConfig {
    /// Base URL for the Phantom wallet API.
    pub api_base_url: String,
    /// Organization ID (required for most operations).
    pub organization_id: Option<String>,
    /// Additional HTTP headers for analytics.
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Wallet type: "server-wallet" or "user-wallet".
    #[serde(default = "default_wallet_type")]
    pub wallet_type: String,
}

fn default_wallet_type() -> String {
    "user-wallet".to_string()
}

/// Result of creating a new wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWalletResult {
    /// The unique wallet identifier.
    pub wallet_id: String,
    /// List of derived addresses for each chain.
    pub addresses: Vec<WalletAddress>,
}

/// An address entry in a wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    /// The type of address (e.g., "solana", "ethereum").
    pub address_type: String,
    /// The blockchain address.
    pub address: String,
}

/// Transaction is an encoded string:
/// - Solana: base64url encoded
/// - Ethereum: RLP-encoded hex string
/// - Other chains: base64url encoded
pub type Transaction = String;

/// A keypair with public and secret keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypair {
    /// Base64url encoded public key.
    pub public_key: String,
    /// Base64url encoded secret key.
    pub secret_key: String,
}

/// A signed transaction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    /// Base64url encoded signed transaction.
    pub raw_transaction: String,
    /// Optional transaction hash if available.
    pub hash: Option<String>,
}

/// Result of signing a transaction (without sending).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransactionResult {
    /// Base64url encoded signed transaction.
    pub raw_transaction: String,
}

/// Result of listing wallets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWalletsResult {
    /// List of wallets.
    pub wallets: Vec<Wallet>,
    /// Total number of wallets.
    pub total_count: u64,
    /// Page limit.
    pub limit: u64,
    /// Page offset.
    pub offset: u64,
}

/// Basic wallet information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// The unique wallet identifier.
    pub wallet_id: String,
    /// The wallet display name.
    pub wallet_name: String,
}

/// Parameters for signing a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignMessageParams {
    /// Wallet ID to use for signing.
    pub wallet_id: String,
    /// Base64url encoded message.
    pub message: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
}

/// Parameters for signing EIP-712 typed data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTypedDataParams {
    /// Wallet ID to use for signing.
    pub wallet_id: String,
    /// EIP-712 typed data object.
    pub typed_data: serde_json::Value,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
}

/// Parameters for signing a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignTransactionParams {
    /// Wallet ID to use for signing.
    pub wallet_id: String,
    /// Base64url encoded transaction.
    pub transaction: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
    /// Optional specific account address to use.
    pub account: Option<String>,
}

/// Parameters for signing and sending a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignAndSendTransactionParams {
    /// Wallet ID to use for signing.
    pub wallet_id: String,
    /// Base64url encoded transaction.
    pub transaction: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
    /// Optional specific account address to use.
    pub account: Option<String>,
}

/// Parameters for getting a wallet by tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWalletWithTagParams {
    /// Organization ID.
    pub organization_id: String,
    /// Tag to search for.
    pub tag: String,
    /// Derivation paths to include.
    pub derivation_paths: Vec<String>,
}

/// Parameters for creating an authenticator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuthenticatorParams {
    /// Organization ID.
    pub organization_id: String,
    /// Username.
    pub username: String,
    /// Authenticator display name.
    pub authenticator_name: String,
    /// Authenticator configuration.
    pub authenticator: AuthenticatorConfig,
    /// Replace existing expirable authenticator.
    pub replace_expirable: Option<bool>,
}

/// Parameters for deleting an authenticator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAuthenticatorParams {
    /// Organization ID.
    pub organization_id: String,
    /// Username.
    pub username: String,
    /// Authenticator ID.
    pub authenticator_id: String,
}

/// Authenticator configuration supporting multiple kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "authenticatorKind")]
pub enum AuthenticatorConfig {
    /// Keypair-based authenticator.
    #[serde(rename = "keypair")]
    Keypair {
        /// Authenticator display name.
        authenticator_name: String,
        /// Base64url encoded public key.
        public_key: String,
        /// Cryptographic algorithm.
        algorithm: ClientAlgorithm,
        /// Optional expiration in milliseconds.
        expires_in_ms: Option<u64>,
    },
    /// Passkey-based authenticator.
    #[serde(rename = "passkey")]
    Passkey {
        /// Authenticator display name.
        authenticator_name: String,
        /// Base64url encoded public key.
        public_key: String,
        /// Cryptographic algorithm.
        algorithm: ClientAlgorithm,
        /// Optional expiration in milliseconds.
        expires_in_ms: Option<u64>,
    },
    /// OIDC-based authenticator.
    #[serde(rename = "oidc")]
    Oidc {
        /// Authenticator display name.
        authenticator_name: String,
        /// JWKS URL for token verification.
        jwks_url: String,
        /// ID token claims.
        id_token_claims: IdTokenClaims,
        /// Optional expiration in milliseconds.
        expires_in_ms: Option<u64>,
    },
}

/// OIDC ID token claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
    /// Subject claim.
    pub sub: String,
    /// Issuer claim.
    pub iss: String,
}

/// User configuration for organization creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// Username.
    pub username: String,
    /// User role: "ADMIN" or "USER" (defaults to "ADMIN").
    pub role: Option<String>,
    /// User's authenticators.
    pub authenticators: Vec<AuthenticatorConfig>,
}

/// Spending limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingLimitConfig {
    /// Daily spending limit in USD cents.
    pub usd_cents_limit_per_day: u64,
}

/// Response from the prepare endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareResponse {
    /// The prepared transaction.
    pub transaction: String,
    /// Optional simulation result.
    pub simulation_result: Option<serde_json::Value>,
    /// Memory config used for spending limits.
    pub memory_config_used: Option<SpendingLimitConfig>,
}

/// Wallet service error types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletServiceErrorType {
    /// Spending limit has been exceeded.
    #[serde(rename = "spending-limit-exceeded")]
    SpendingLimitExceeded,
    /// Transaction was blocked by security policy.
    #[serde(rename = "transaction-blocked")]
    TransactionBlocked,
}

/// Wallet service error data from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletServiceErrorData {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: WalletServiceErrorType,
    /// Error title.
    pub title: String,
    /// Error detail message.
    pub detail: String,
    /// Request ID.
    #[serde(rename = "requestId")]
    pub request_id: String,
}

/// Extended error response from the prepare endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareErrorResponse {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: Option<WalletServiceErrorType>,
    /// Error title.
    pub title: Option<String>,
    /// Error detail message.
    pub detail: Option<String>,
    /// Request ID.
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    /// General error message.
    pub message: Option<String>,

    // Spending limit specific fields
    /// Previous spend in cents.
    #[serde(rename = "previousSpendCents")]
    pub previous_spend_cents: Option<u64>,
    /// Transaction spend in cents.
    #[serde(rename = "transactionSpendCents")]
    pub transaction_spend_cents: Option<u64>,
    /// Total spend in cents.
    #[serde(rename = "totalSpendCents")]
    pub total_spend_cents: Option<u64>,
    /// Limit in cents.
    #[serde(rename = "limitCents")]
    pub limit_cents: Option<u64>,

    // Transaction blocked specific fields
    /// Full simulation result when transaction is blocked.
    #[serde(rename = "scannerResult")]
    pub scanner_result: Option<serde_json::Value>,
}

/// Derivation information for a wallet account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationInfo {
    /// BIP-44/84 derivation path.
    #[serde(rename = "derivationPath")]
    pub derivation_path: String,
    /// Elliptic curve.
    pub curve: Curve,
    /// Address format.
    #[serde(rename = "addressFormat")]
    pub address_format: AddressFormat,
}

/// Submission configuration for transaction submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionConfig {
    /// Chain name (e.g., "solana", "ethereum", "polygon").
    pub chain: String,
    /// Network name (e.g., "mainnet", "devnet", "sepolia").
    pub network: String,
}

/// Simulation configuration for transaction simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// The address/account that is signing the transaction.
    pub account: String,
}
