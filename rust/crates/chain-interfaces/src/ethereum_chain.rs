//! Ethereum/EVM chain interface.

use serde::{Deserialize, Serialize};

/// Ethereum transaction request parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EthTransactionRequest {
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Value in wei (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Gas limit (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<String>,
    /// Gas price (hex-encoded, legacy transactions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    /// Max fee per gas (hex-encoded, EIP-1559).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<String>,
    /// Max priority fee per gas (hex-encoded, EIP-1559).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<String>,
    /// Transaction data (hex-encoded calldata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Transaction nonce (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Transaction type (hex-encoded).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tx_type: Option<String>,
    /// Chain ID (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
}

/// Trait for interacting with an Ethereum/EVM chain.
///
/// Provides methods for connecting, signing messages and transactions,
/// sending transactions, and managing chain/account state.
#[async_trait::async_trait]
pub trait EthereumChain: Send + Sync {
    /// The current EIP-155 chain ID (hex-encoded).
    fn chain_id(&self) -> &str;

    /// The current connected accounts.
    fn accounts(&self) -> &[String];

    /// Send a JSON-RPC request to the provider.
    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;

    /// Connect to the wallet and return the list of accounts.
    async fn connect(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a personal message (EIP-191).
    async fn sign_personal_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign EIP-712 typed data.
    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a transaction without broadcasting.
    async fn sign_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send a transaction, returning the transaction hash.
    async fn send_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Switch to a different EVM chain.
    async fn switch_chain(
        &self,
        chain_id: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the current chain ID as a number.
    async fn get_chain_id(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the list of connected accounts.
    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;

    /// Check if the wallet is connected.
    fn is_connected(&self) -> bool;
}
