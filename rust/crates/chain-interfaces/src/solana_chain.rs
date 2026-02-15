//! Solana chain interface.

/// Options for connecting to a Solana wallet.
#[derive(Debug, Clone, Default)]
pub struct SolanaConnectOptions {
    /// Only connect if the user has previously approved the connection.
    pub only_if_trusted: Option<bool>,
}

/// Result of a successful Solana connection.
#[derive(Debug, Clone)]
pub struct SolanaConnectResult {
    /// Base58-encoded public key of the connected wallet.
    pub public_key: String,
}

/// Result of signing a message on Solana.
#[derive(Debug, Clone)]
pub struct SolanaSignMessageResult {
    /// The detached Ed25519 signature.
    pub signature: Vec<u8>,
    /// Base58-encoded public key that produced the signature.
    pub public_key: String,
}

/// Result of signing and sending a transaction on Solana.
#[derive(Debug, Clone)]
pub struct SolanaSendTransactionResult {
    /// Base58-encoded transaction signature.
    pub signature: String,
}

/// Result of signing and sending all transactions on Solana.
#[derive(Debug, Clone)]
pub struct SolanaSendAllTransactionsResult {
    /// Base58-encoded transaction signatures.
    pub signatures: Vec<String>,
}

/// Solana network identifier for switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolanaNetwork {
    /// Solana mainnet-beta.
    Mainnet,
    /// Solana devnet.
    Devnet,
}

/// Trait for interacting with a Solana chain.
///
/// Provides methods for connecting, signing messages and transactions,
/// and managing wallet state on the Solana blockchain.
#[async_trait::async_trait]
pub trait SolanaChain: Send + Sync {
    /// The base58-encoded public key of the connected wallet, if any.
    fn public_key(&self) -> Option<&str>;

    /// Whether the wallet is currently connected.
    fn is_connected(&self) -> bool;

    /// Connect to the wallet.
    async fn connect(
        &self,
        options: Option<SolanaConnectOptions>,
    ) -> Result<SolanaConnectResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a message. The message can be a UTF-8 string or raw bytes.
    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a serialized transaction without broadcasting.
    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send a transaction, returning the signature.
    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SolanaSendTransactionResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign multiple transactions without broadcasting.
    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send multiple transactions.
    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SolanaSendAllTransactionsResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Switch to a different Solana network (NOOP in most wallets except embedded providers).
    async fn switch_network(
        &self,
        network: SolanaNetwork,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
