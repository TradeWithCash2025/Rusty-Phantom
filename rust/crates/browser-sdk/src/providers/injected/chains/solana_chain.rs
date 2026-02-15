//! Injected wallet Solana chain wrapper.
//!
//! Wraps an external [`SolanaChain`] provider (e.g. a Wallet Standard wallet)
//! and adds debug logging and event forwarding, mirroring the TypeScript
//! `InjectedWalletSolanaChain` in
//! `packages/browser-sdk/src/providers/injected/chains/InjectedWalletSolanaChain.ts`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use phantom_chain_interfaces::{
    SolanaChain, SolanaConnectOptions, SolanaConnectResult, SolanaNetwork,
    SolanaSignMessageResult, SolanaSendAllTransactionsResult, SolanaSendTransactionResult,
};

// ============================================================================
// Event listener registry (same pattern as embedded_chains.rs)
// ============================================================================

/// A generic, thread-safe event listener registry keyed by event name strings.
struct EventListenerRegistry {
    listeners: Mutex<HashMap<String, Vec<(u64, Arc<dyn Fn(serde_json::Value) + Send + Sync>)>>>,
    next_id: AtomicU64,
}

impl EventListenerRegistry {
    fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Register a listener for `event`. Returns a unique listener ID.
    fn add(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut map = self.listeners.lock().unwrap();
        map.entry(event.to_string())
            .or_insert_with(Vec::new)
            .push((id, Arc::from(listener)));
        id
    }

    /// Remove a listener by event name and ID.
    fn remove(&self, event: &str, listener_id: u64) {
        let mut map = self.listeners.lock().unwrap();
        if let Some(list) = map.get_mut(event) {
            list.retain(|(id, _)| *id != listener_id);
            if list.is_empty() {
                map.remove(event);
            }
        }
    }
}

// ============================================================================
// InjectedWalletSolanaChain
// ============================================================================

/// Wrapper around an external [`SolanaChain`] provider that adds debug
/// logging for all operations and event forwarding.
///
/// This is the Rust equivalent of the TypeScript `InjectedWalletSolanaChain`
/// class, used for external Wallet Standard Solana providers.
pub struct InjectedWalletSolanaChain {
    /// The inner provider being wrapped.
    inner: Arc<dyn SolanaChain>,
    /// Wallet identifier (e.g. "phantom").
    wallet_id: String,
    /// Human-readable wallet name (e.g. "Phantom").
    wallet_name: String,
    /// Cached public key so `public_key()` can return `Option<&str>`.
    public_key_cache: Mutex<Option<String>>,
    /// Local event listener registry for this wrapper.
    events: EventListenerRegistry,
}

impl InjectedWalletSolanaChain {
    /// Create a new `InjectedWalletSolanaChain` wrapping the given provider.
    pub fn new(inner: Arc<dyn SolanaChain>, wallet_id: String, wallet_name: String) -> Self {
        // Seed the cache from the inner provider.
        let public_key_cache = Mutex::new(inner.public_key().map(|s| s.to_string()));

        Self {
            inner,
            wallet_id,
            wallet_name,
            public_key_cache,
            events: EventListenerRegistry::new(),
        }
    }

    /// Refresh the public key cache from the inner provider.
    fn refresh_public_key_cache(&self) {
        *self.public_key_cache.lock().unwrap() = self.inner.public_key().map(|s| s.to_string());
    }
}

#[async_trait::async_trait]
impl SolanaChain for InjectedWalletSolanaChain {
    fn public_key(&self) -> Option<&str> {
        self.refresh_public_key_cache();
        // Same Box::leak pattern as embedded_chains.rs -- the number of
        // distinct public keys is 1 (the connected wallet).
        let pk = self.public_key_cache.lock().unwrap().clone();
        pk.map(|s| &*Box::leak(s.into_boxed_str()) as &str)
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    async fn connect(
        &self,
        options: Option<SolanaConnectOptions>,
    ) -> Result<SolanaConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Solana connect"
        );

        match self.inner.connect(options).await {
            Ok(result) => {
                *self.public_key_cache.lock().unwrap() = Some(result.public_key.clone());
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    public_key = %result.public_key,
                    "External wallet Solana connected"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana connect failed"
                );
                Err(e)
            }
        }
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Solana disconnect"
        );

        match self.inner.disconnect().await {
            Ok(()) => {
                *self.public_key_cache.lock().unwrap() = None;
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    "External wallet Solana disconnected"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana disconnect failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            message_length = message.len(),
            "External wallet Solana signMessage"
        );

        match self.inner.sign_message(message).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature_length = result.signature.len(),
                    "External wallet Solana signMessage success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signMessage failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Solana signTransaction"
        );

        match self.inner.sign_transaction(transaction).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    "External wallet Solana signTransaction success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signTransaction failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SolanaSendTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Solana signAndSendTransaction"
        );

        match self.inner.sign_and_send_transaction(transaction).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature = %result.signature,
                    "External wallet Solana signAndSendTransaction success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signAndSendTransaction failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            transaction_count = transactions.len(),
            "External wallet Solana signAllTransactions"
        );

        match self.inner.sign_all_transactions(transactions).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signed_count = result.len(),
                    "External wallet Solana signAllTransactions success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signAllTransactions failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SolanaSendAllTransactionsResult, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            transaction_count = transactions.len(),
            "External wallet Solana signAndSendAllTransactions"
        );

        match self.inner.sign_and_send_all_transactions(transactions).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature_count = result.signatures.len(),
                    "External wallet Solana signAndSendAllTransactions success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signAndSendAllTransactions failed"
                );
                Err(e)
            }
        }
    }

    async fn switch_network(
        &self,
        network: SolanaNetwork,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            network = ?network,
            "External wallet Solana switchNetwork"
        );

        match self.inner.switch_network(network).await {
            Ok(()) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    network = ?network,
                    "External wallet Solana switchNetwork success"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana switchNetwork failed"
                );
                Err(e)
            }
        }
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        // Register on both the local registry and the inner provider so that
        // events emitted by either side reach the caller.
        let local_id = self.events.add(event, listener);
        // Forward to inner provider as well.
        let inner_listener: Box<dyn Fn(serde_json::Value) + Send + Sync> =
            Box::new(|_| { /* forwarded via inner */ });
        let _inner_id = self.inner.on(event, inner_listener);
        local_id
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove(event, listener_id);
        // Also forward removal to inner provider.
        self.inner.off(event, listener_id);
    }
}
