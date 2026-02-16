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

/// Solana Sign-In With data (following the SIWS spec).
///
/// All fields are optional per the SIWS specification. The wallet
/// provider fills in defaults (e.g. `domain`, `issued_at`) when omitted.
#[derive(Debug, Clone, Default)]
pub struct SolanaSignInInput {
    pub domain: Option<String>,
    pub address: Option<String>,
    pub statement: Option<String>,
    pub uri: Option<String>,
    pub version: Option<String>,
    pub chain_id: Option<String>,
    pub nonce: Option<String>,
    pub issued_at: Option<String>,
    pub expiration_time: Option<String>,
    pub not_before: Option<String>,
    pub request_id: Option<String>,
    pub resources: Option<Vec<String>>,
}

/// Result of a Solana sign-in operation.
#[derive(Debug, Clone)]
pub struct SolanaSignInOutput {
    /// The base58-encoded address that signed in.
    pub address: String,
    /// The Ed25519 signature bytes.
    pub signature: Vec<u8>,
    /// The signed message bytes (the SIWS message that was signed).
    pub signed_message: Vec<u8>,
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

    /// Get the currently connected account address.
    ///
    /// Default implementation returns the public key if connected.
    async fn get_account(&self) -> Option<String> {
        self.public_key().map(|s| s.to_string())
    }

    /// Sign a message. The message bytes are typically UTF-8 encoded text
    /// or arbitrary binary data.
    ///
    /// See also [`sign_message_str`](Self::sign_message_str) for a convenience
    /// method that accepts `&str` directly.
    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a UTF-8 string message.
    ///
    /// This mirrors the TypeScript `signMessage(message: string | Uint8Array)`
    /// which accepts either strings or byte arrays. In Rust, the trait's
    /// primary method takes `&[u8]`; this convenience method handles the
    /// string-to-bytes conversion (UTF-8 encoding, matching JS `TextEncoder`).
    async fn sign_message_str(
        &self,
        message: &str,
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        self.sign_message(message.as_bytes()).await
    }

    /// Sign in with Solana (SIWS — Sign In With Solana).
    ///
    /// Default implementation returns an error. Override in providers
    /// that support the SIWS protocol (e.g. Phantom injected provider).
    async fn sign_in(
        &self,
        _input: &SolanaSignInInput,
    ) -> Result<SolanaSignInOutput, Box<dyn std::error::Error + Send + Sync>> {
        Err("sign_in is not supported by this provider".into())
    }

    /// Sign a serialized transaction without broadcasting.
    ///
    /// The `transaction` parameter is the wire-format serialized transaction.
    /// Use [`SolanaTransaction`] helpers to convert from base64 or base58.
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

    /// Register an event listener.
    ///
    /// Supported events: "connect", "disconnect", "accountChanged".
    /// Returns a listener ID for removal.
    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64;

    /// Remove an event listener by ID.
    fn off(&self, event: &str, listener_id: u64);
}

// ============================================================================
// Transaction serialization helpers
// ============================================================================

/// Convenience helpers for converting between Solana transaction formats.
///
/// The [`SolanaChain`] trait accepts raw bytes (`&[u8]`) for all transaction
/// methods. These helpers convert to and from the common serialization
/// formats used in Solana tooling:
///
/// ```ignore
/// use phantom_chain_interfaces::SolanaTransaction;
///
/// // Decode a base64 transaction from an RPC response
/// let tx_bytes = SolanaTransaction::from_base64(base64_str)?;
/// let signed = chain.sign_transaction(&tx_bytes).await?;
///
/// // Encode back to base64 for submission
/// let signed_b64 = SolanaTransaction::to_base64(&signed);
/// ```
pub struct SolanaTransaction;

impl SolanaTransaction {
    /// Decode a **standard base64**-encoded serialized transaction to bytes.
    ///
    /// This is the encoding used by Solana RPC methods like
    /// `getTransaction` with `"encoding": "base64"`.
    pub fn from_base64(encoded: &str) -> Result<Vec<u8>, String> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| format!("Invalid base64 transaction: {e}"))
    }

    /// Decode a **base58**-encoded serialized transaction to bytes.
    ///
    /// This is the legacy encoding used by older Solana RPC versions.
    pub fn from_base58(encoded: &str) -> Result<Vec<u8>, String> {
        bs58::decode(encoded)
            .into_vec()
            .map_err(|e| format!("Invalid base58 transaction: {e}"))
    }

    /// Encode transaction bytes as **standard base64**.
    pub fn to_base64(bytes: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// Encode transaction bytes as **base58**.
    pub fn to_base58(bytes: &[u8]) -> String {
        bs58::encode(bytes).into_string()
    }
}
