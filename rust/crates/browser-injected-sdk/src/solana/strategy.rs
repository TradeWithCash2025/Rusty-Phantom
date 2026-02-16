//! Solana provider strategy traits and implementations.

use super::types::*;
use crate::types::ProviderStrategy;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Provider trait (represents window.phantom.solana in TS)
// ============================================================================

/// Trait representing a Phantom Solana provider.
///
/// In the TypeScript SDK, this is the `PhantomSolanaProvider` interface
/// which maps to `window.phantom.solana`. In Rust, platform adapters
/// implement this trait to provide the actual wallet interaction.
#[async_trait::async_trait]
pub trait PhantomSolanaProvider: Send + Sync {
    /// Whether the provider is Phantom.
    fn is_phantom(&self) -> bool;

    /// The connected public key, if any.
    fn public_key(&self) -> Option<String>;

    /// Whether the provider is connected.
    fn is_connected(&self) -> bool;

    /// Connect to the wallet.
    async fn connect(
        &self,
        only_if_trusted: bool,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a message.
    async fn sign_message(
        &self,
        message: &[u8],
        display: Option<DisplayEncoding>,
    ) -> Result<SignMessageResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign in with Solana.
    async fn sign_in(
        &self,
        sign_in_data: &SolanaSignInData,
    ) -> Result<SignInResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send a transaction.
    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
        options: Option<&SendOptions>,
    ) -> Result<SignAndSendResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send all transactions.
    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
        options: Option<&SendOptions>,
    ) -> Result<SignAndSendAllResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign all transactions.
    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a single transaction.
    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// Strategy trait (wraps a provider with higher-level operations)
// ============================================================================

/// Trait representing a Solana strategy.
///
/// Strategies wrap a `PhantomSolanaProvider` and provide higher-level
/// operations. The only current strategy is `Injected` (window.phantom.solana).
#[async_trait::async_trait]
pub trait SolanaStrategy: Send + Sync {
    /// The strategy type.
    fn strategy_type(&self) -> ProviderStrategy;

    /// Whether the provider is currently connected.
    fn is_connected(&self) -> bool;

    /// Get the underlying provider.
    fn get_provider(&self) -> Option<Arc<dyn PhantomSolanaProvider>>;

    /// Connect to the wallet.
    async fn connect(
        &self,
        options: ConnectOptions,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get the currently connected account address.
    async fn get_account(&self) -> Option<String>;

    /// Sign a message.
    async fn sign_message(
        &self,
        message: &[u8],
        display: Option<DisplayEncoding>,
    ) -> Result<SignMessageResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign in with Solana.
    async fn sign_in(
        &self,
        sign_in_data: &SolanaSignInData,
    ) -> Result<SignInResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send a transaction.
    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SignAndSendResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign and send all transactions.
    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SignAndSendAllResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a single transaction.
    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign all transactions.
    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// InjectedSolanaStrategy
// ============================================================================

const MAX_RETRIES: u32 = 4;
const BASE_DELAY_MS: u64 = 100;

/// Factory trait for obtaining an injected Solana provider.
///
/// In the TypeScript SDK, this is handled by accessing `window.phantom.solana`.
/// In Rust, the platform must provide a factory that returns the provider.
#[async_trait::async_trait]
pub trait SolanaProviderFactory: Send + Sync {
    /// Attempt to get the injected Phantom Solana provider.
    fn get_provider(&self) -> Option<Arc<dyn PhantomSolanaProvider>>;
}

/// Injected Solana strategy — wraps a provider obtained from the platform.
pub struct InjectedSolanaStrategy {
    factory: Arc<dyn SolanaProviderFactory>,
    provider: Option<Arc<dyn PhantomSolanaProvider>>,
}

impl InjectedSolanaStrategy {
    /// Create a new injected strategy with the given factory.
    pub fn new(factory: Arc<dyn SolanaProviderFactory>) -> Self {
        Self {
            factory,
            provider: None,
        }
    }

    /// Load the provider with exponential backoff retries.
    pub async fn load(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for retry in 0..MAX_RETRIES {
            let delay = Duration::from_millis(BASE_DELAY_MS * 2u64.pow(retry.min(5)));
            sleep(delay).await;

            if let Some(provider) = self.factory.get_provider() {
                self.provider = Some(provider);
                return Ok(());
            }
        }
        Err("Provider not found.".into())
    }

    fn require_provider(
        &self,
    ) -> Result<&Arc<dyn PhantomSolanaProvider>, Box<dyn std::error::Error + Send + Sync>> {
        self.provider
            .as_ref()
            .ok_or_else(|| "Provider not found.".into())
    }
}

#[async_trait::async_trait]
impl SolanaStrategy for InjectedSolanaStrategy {
    fn strategy_type(&self) -> ProviderStrategy {
        ProviderStrategy::Injected
    }

    fn is_connected(&self) -> bool {
        self.provider
            .as_ref()
            .map(|p| p.is_connected() && p.public_key().is_some())
            .unwrap_or(false)
    }

    fn get_provider(&self) -> Option<Arc<dyn PhantomSolanaProvider>> {
        self.provider.clone()
    }

    async fn connect(
        &self,
        options: ConnectOptions,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;

        if provider.is_connected() && provider.public_key().is_some() {
            return Ok(self.get_account().await);
        }

        match provider.connect(options.only_if_trusted).await {
            Ok(public_key) => Ok(Some(public_key)),
            Err(_) => Ok(None),
        }
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        provider.disconnect().await
    }

    async fn get_account(&self) -> Option<String> {
        let provider = self.provider.as_ref()?;
        if provider.is_connected() {
            provider.public_key()
        } else {
            None
        }
    }

    async fn sign_message(
        &self,
        message: &[u8],
        display: Option<DisplayEncoding>,
    ) -> Result<SignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }
        provider.sign_message(message, display).await
    }

    async fn sign_in(
        &self,
        sign_in_data: &SolanaSignInData,
    ) -> Result<SignInResult, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        provider.sign_in(sign_in_data).await
    }

    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SignAndSendResult, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }
        provider.sign_and_send_transaction(transaction, None).await
    }

    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SignAndSendAllResult, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }
        provider
            .sign_and_send_all_transactions(transactions, None)
            .await
    }

    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }
        provider.sign_transaction(transaction).await
    }

    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }
        provider.sign_all_transactions(transactions).await
    }
}
