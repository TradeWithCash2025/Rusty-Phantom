//! Wallet Standard types and adapter for the browser SDK.
//!
//! Defines types for interacting with wallets that implement the Wallet Standard
//! protocol, including type definitions and a `WalletStandardSolanaAdapter`.

use std::sync::Arc;

/// A Wallet Standard account.
#[derive(Debug, Clone)]
pub struct WalletStandardAccount {
    /// The account's address string.
    pub address: String,
    /// The raw public key bytes.
    pub public_key: Vec<u8>,
    /// CAIP-2 chain identifiers this account supports.
    pub chains: Vec<String>,
    /// Feature identifiers this account supports.
    pub features: Vec<String>,
}

/// Properties that can change on a Wallet Standard wallet.
#[derive(Debug, Clone, Default)]
pub struct StandardEventsChangeProperties {
    /// Updated chain identifiers, if changed.
    pub chains: Option<Vec<String>>,
    /// Updated feature identifiers, if changed.
    pub features: Option<Vec<String>>,
    /// Updated accounts, if changed.
    pub accounts: Option<Vec<WalletStandardAccount>>,
}

/// Standard connect feature.
#[async_trait::async_trait]
pub trait StandardConnectFeature: Send + Sync {
    /// Connect to the wallet, returning the available accounts.
    async fn connect(
        &self,
    ) -> Result<Vec<WalletStandardAccount>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Standard disconnect feature.
#[async_trait::async_trait]
pub trait StandardDisconnectFeature: Send + Sync {
    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Standard events feature.
pub trait StandardEventsFeature: Send + Sync {
    /// Register a callback for the given event. Returns a listener ID.
    fn on(
        &self,
        event: &str,
        callback: Box<dyn Fn(StandardEventsChangeProperties) + Send + Sync>,
    ) -> u64;

    /// Remove a listener by its ID.
    fn off(&self, event: &str, listener_id: u64);
}

/// Solana sign message feature.
#[async_trait::async_trait]
pub trait SolanaSignMessageFeature: Send + Sync {
    /// Sign a message with the given account.
    async fn sign_message(
        &self,
        message: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Solana sign transaction feature.
#[async_trait::async_trait]
pub trait SolanaSignTransactionFeature: Send + Sync {
    /// Sign a serialized transaction with the given account.
    async fn sign_transaction(
        &self,
        transaction: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Solana sign and send transaction feature.
#[async_trait::async_trait]
pub trait SolanaSignAndSendTransactionFeature: Send + Sync {
    /// Sign and send a serialized transaction with the given account.
    ///
    /// Returns the transaction signature.
    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
        account: &WalletStandardAccount,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Combined features map for a Wallet Standard wallet.
pub struct WalletStandardFeatures {
    /// Standard connect feature.
    pub connect: Option<Arc<dyn StandardConnectFeature>>,
    /// Standard disconnect feature.
    pub disconnect: Option<Arc<dyn StandardDisconnectFeature>>,
    /// Standard events feature.
    pub events: Option<Arc<dyn StandardEventsFeature>>,
    /// Solana sign message feature.
    pub solana_sign_message: Option<Arc<dyn SolanaSignMessageFeature>>,
    /// Solana sign transaction feature.
    pub solana_sign_transaction: Option<Arc<dyn SolanaSignTransactionFeature>>,
    /// Solana sign and send transaction feature.
    pub solana_sign_and_send_transaction: Option<Arc<dyn SolanaSignAndSendTransactionFeature>>,
}

/// A Wallet Standard wallet.
pub struct WalletStandardWallet {
    /// The wallet's display name.
    pub name: String,
    /// The wallet's icon (typically a data URI).
    pub icon: String,
    /// The wallet's version string.
    pub version: String,
    /// CAIP-2 chain identifiers supported by the wallet.
    pub chains: Vec<String>,
    /// The wallet's features.
    pub features: WalletStandardFeatures,
    /// The wallet's currently connected accounts.
    pub accounts: Vec<WalletStandardAccount>,
}

/// Adapter that bridges a Wallet Standard wallet to the `SolanaChain` interface.
///
/// TODO: Implement browser-specific bindings. This is a stub that provides
/// the structural type for the adapter pattern used in the TypeScript SDK.
pub struct WalletStandardSolanaAdapter {
    /// The underlying Wallet Standard wallet.
    #[allow(dead_code)]
    wallet: Arc<WalletStandardWallet>,
}

impl WalletStandardSolanaAdapter {
    /// Create a new adapter wrapping the given wallet.
    pub fn new(wallet: Arc<WalletStandardWallet>) -> Self {
        Self { wallet }
    }
}
