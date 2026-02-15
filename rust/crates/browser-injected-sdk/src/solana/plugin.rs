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
use std::sync::RwLock;

/// Phantom Solana chain implementation that implements `SolanaChain`.
///
/// Wraps a `SolanaStrategy` with event listeners and state management.
pub struct Solana {
    strategy: Arc<dyn SolanaStrategy>,
    events: Arc<SolanaEventListeners>,
    /// Stored behind `std::sync::RwLock` so that synchronous trait methods and
    /// `handle_provider_event` can access it without an async runtime.
    public_key: RwLock<Option<String>>,
}

impl Solana {
    /// Create a new Solana instance with the given strategy.
    ///
    /// Internally calls [`bind_provider_events`](Self::bind_provider_events) to
    /// prepare the event bridge (matching the TS constructor behaviour).
    pub fn new(strategy: Arc<dyn SolanaStrategy>) -> Self {
        let instance = Self {
            strategy,
            events: Arc::new(SolanaEventListeners::new()),
            public_key: RwLock::new(None),
        };
        instance.bind_provider_events();
        instance
    }

    /// Get a reference to the event listener registry.
    pub fn events(&self) -> &SolanaEventListeners {
        &self.events
    }

    /// Prepare the native provider event bridge.
    ///
    /// In a browser context the TS `Solana` constructor calls `bindProviderEvents()`
    /// to forward native wallet events (`connect`, `disconnect`, `accountChanged`)
    /// into the SDK event system.  In Rust we cannot register native listeners
    /// directly, so the actual bridging is performed by the platform layer calling
    /// [`handle_provider_event`](Self::handle_provider_event).  This method exists
    /// to mirror the TS constructor flow and can be extended later when a platform
    /// bridge is available.
    fn bind_provider_events(&self) {
        // Platform-specific event registration would go here.
        // For now, the platform layer is expected to call `handle_provider_event`
        // when the native wallet provider emits events.
    }

    /// Handle a native provider event, bridging it to SDK event listeners.
    ///
    /// Platform code should call this when the native wallet provider emits events.
    ///
    /// Supported events:
    /// - `"connect"` — `data` should contain the public key string.
    /// - `"disconnect"` — `data` is ignored.
    /// - `"accountChanged"` — `data` may contain the new public key (or `None`).
    ///   Triggers **both** `accountChanged` and `connect` events (dual-trigger
    ///   behaviour matching the TS implementation).
    pub fn handle_provider_event(&self, event: &str, data: Option<&str>) {
        match event {
            "connect" => {
                if let Some(pk) = data {
                    if let Ok(mut guard) = self.public_key.write() {
                        *guard = Some(pk.to_string());
                    }
                    self.events.trigger_connect(pk);
                }
            }
            "disconnect" => {
                if let Ok(mut guard) = self.public_key.write() {
                    *guard = None;
                }
                self.events.trigger_disconnect();
            }
            "accountChanged" => {
                if let Ok(mut guard) = self.public_key.write() {
                    *guard = data.map(|s| s.to_string());
                }
                // Dual-trigger: accountChanged AND connect (matching TS behavior)
                self.events.trigger_account_changed(data);
                if let Some(pk) = data {
                    self.events.trigger_connect(pk);
                }
            }
            _ => {}
        }
    }

    /// Return an owned copy of the current public key.
    ///
    /// The [`SolanaChain::public_key`] trait method returns `Option<&str>`, which
    /// cannot borrow through the internal lock.  Use this method when you need
    /// the actual runtime value.
    pub fn get_public_key(&self) -> Option<String> {
        self.public_key
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }
}

#[async_trait::async_trait]
impl SolanaChain for Solana {
    fn public_key(&self) -> Option<&str> {
        // The trait requires `Option<&str>`, but we cannot return a reference to
        // data behind a lock guard whose lifetime is shorter than `&self`.
        // Use `Solana::get_public_key()` to obtain an owned `Option<String>`.
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

        if let Ok(mut guard) = self.public_key.write() {
            *guard = Some(address.clone());
        }

        Ok(SolanaConnectResult {
            public_key: address,
        })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        operations::disconnect(self.strategy.as_ref(), &self.events).await?;
        if let Ok(mut guard) = self.public_key.write() {
            *guard = None;
        }
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
            if let Some(pk) = self.public_key.read().ok().and_then(|g| g.clone()) {
                return Ok(SolanaSignMessageResult {
                    signature: result.signature,
                    public_key: pk,
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

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        self.events.add_listener_by_name(event, listener)
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove_listener_by_name(event, listener_id);
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
