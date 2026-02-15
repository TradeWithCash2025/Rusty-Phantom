//! Solana plugin for the Phantom browser-injected SDK.
//!
//! Provides the `Solana` struct that implements `SolanaChain` from chain-interfaces,
//! and the `create_solana_plugin` factory function.

use super::events::SolanaEventListeners;
use super::operations;
use super::strategy::SolanaStrategy;
use crate::Plugin;
use phantom_chain_interfaces::{
    SolanaChain, SolanaConnectOptions, SolanaConnectResult, SolanaNetwork,
    SolanaSignMessageResult, SolanaSendTransactionResult, SolanaSendAllTransactionsResult,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Phantom Solana chain implementation that implements `SolanaChain`.
///
/// Wraps a `SolanaStrategy` with event listeners and state management.
pub struct Solana {
    strategy: Arc<dyn SolanaStrategy>,
    events: Arc<SolanaEventListeners>,
    public_key: RwLock<Option<String>>,
}

impl Solana {
    /// Create a new Solana instance with the given strategy.
    pub fn new(strategy: Arc<dyn SolanaStrategy>) -> Self {
        Self {
            strategy,
            events: Arc::new(SolanaEventListeners::new()),
            public_key: RwLock::new(None),
        }
    }

    /// Get a reference to the event listener registry.
    pub fn events(&self) -> &SolanaEventListeners {
        &self.events
    }
}

#[async_trait::async_trait]
impl SolanaChain for Solana {
    fn public_key(&self) -> Option<&str> {
        // We can't return a reference to data behind RwLock.
        // The trait requires Option<&str>, so we need a workaround.
        // Since the trait is from chain-interfaces, we return None here
        // and provide a separate async method for getting the public key.
        None
    }

    fn is_connected(&self) -> bool {
        self.strategy.is_connected()
    }

    async fn connect(
        &self,
        options: Option<SolanaConnectOptions>,
    ) -> Result<SolanaConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        let only_if_trusted = options
            .as_ref()
            .and_then(|o| o.only_if_trusted)
            .unwrap_or(false);

        let address =
            operations::connect(self.strategy.as_ref(), &self.events, only_if_trusted).await?;

        *self.public_key.write().await = Some(address.clone());

        Ok(SolanaConnectResult {
            public_key: address,
        })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        operations::disconnect(self.strategy.as_ref(), &self.events).await?;
        *self.public_key.write().await = None;
        Ok(())
    }

    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        let result =
            operations::sign_message(self.strategy.as_ref(), message, None).await?;

        let public_key = result.address.clone();
        if public_key.is_empty() {
            if let Some(pk) = self.public_key.read().await.as_ref() {
                return Ok(SolanaSignMessageResult {
                    signature: result.signature,
                    public_key: pk.clone(),
                });
            }
        }

        Ok(SolanaSignMessageResult {
            signature: result.signature,
            public_key,
        })
    }

    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        operations::sign_transaction(self.strategy.as_ref(), transaction).await
    }

    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SolanaSendTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        let result =
            operations::sign_and_send_transaction(self.strategy.as_ref(), transaction).await?;
        Ok(SolanaSendTransactionResult {
            signature: result.signature,
        })
    }

    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        operations::sign_all_transactions(self.strategy.as_ref(), transactions).await
    }

    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SolanaSendAllTransactionsResult, Box<dyn std::error::Error + Send + Sync>> {
        let result =
            operations::sign_and_send_all_transactions(self.strategy.as_ref(), transactions)
                .await?;
        Ok(SolanaSendAllTransactionsResult {
            signatures: result.signatures,
        })
    }

    async fn switch_network(
        &self,
        _network: SolanaNetwork,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Solana network switching is typically handled by the provider.
        // This is a no-op for the browser-injected SDK.
        Ok(())
    }
}

/// Create a Solana plugin for the Phantom instance.
pub fn create_solana_plugin(strategy: Arc<dyn SolanaStrategy>) -> Plugin {
    Plugin {
        name: "solana".to_string(),
        create: Box::new(move || {
            Box::new(Solana::new(strategy.clone()))
        }),
    }
}
