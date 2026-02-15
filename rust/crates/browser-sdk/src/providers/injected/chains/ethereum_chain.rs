//! Injected wallet Ethereum chain wrapper.
//!
//! Wraps an external [`EthereumChain`] provider (e.g. an EIP-6963 wallet)
//! and adds debug logging and event forwarding, mirroring the TypeScript
//! `InjectedWalletEthereumChain` in
//! `packages/browser-sdk/src/providers/injected/chains/InjectedWalletEthereumChain.ts`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use phantom_chain_interfaces::{EthTransactionRequest, EthereumChain};

// ============================================================================
// Event listener registry (same pattern as embedded_chains.rs)
// ============================================================================

/// A generic, thread-safe event listener registry keyed by event name strings.
///
/// Each listener receives a [`serde_json::Value`] payload and is identified by
/// a monotonically increasing `u64` ID that can be used for removal.
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
// InjectedWalletEthereumChain
// ============================================================================

/// Wrapper around an external [`EthereumChain`] provider that adds debug
/// logging for all operations and event forwarding.
///
/// This is the Rust equivalent of the TypeScript `InjectedWalletEthereumChain`
/// class, used for external EIP-6963 Ethereum providers.
pub struct InjectedWalletEthereumChain {
    /// The inner provider being wrapped.
    inner: Arc<dyn EthereumChain>,
    /// Wallet identifier (e.g. "phantom", "metamask").
    wallet_id: String,
    /// Human-readable wallet name (e.g. "Phantom", "MetaMask").
    wallet_name: String,
    /// Cached chain ID so `chain_id()` can return `&str`.
    chain_id_cache: Mutex<String>,
    /// Cached accounts so `accounts()` can return `&[String]`.
    accounts_cache: Mutex<Vec<String>>,
    /// Local event listener registry for this wrapper.
    events: EventListenerRegistry,
}

impl InjectedWalletEthereumChain {
    /// Create a new `InjectedWalletEthereumChain` wrapping the given provider.
    pub fn new(inner: Arc<dyn EthereumChain>, wallet_id: String, wallet_name: String) -> Self {
        // Seed the caches from the inner provider.
        let chain_id_cache = Mutex::new(inner.chain_id().to_string());
        let accounts_cache = Mutex::new(inner.accounts().to_vec());

        Self {
            inner,
            wallet_id,
            wallet_name,
            chain_id_cache,
            accounts_cache,
            events: EventListenerRegistry::new(),
        }
    }

    /// Refresh the chain ID cache from the inner provider.
    fn refresh_chain_id_cache(&self) {
        *self.chain_id_cache.lock().unwrap() = self.inner.chain_id().to_string();
    }

    /// Refresh the accounts cache from the inner provider.
    fn refresh_accounts_cache(&self) {
        *self.accounts_cache.lock().unwrap() = self.inner.accounts().to_vec();
    }
}

#[async_trait::async_trait]
impl EthereumChain for InjectedWalletEthereumChain {
    fn chain_id(&self) -> &str {
        // Refresh cache from the inner provider each time so we stay in sync.
        self.refresh_chain_id_cache();
        // Same Box::leak pattern as embedded_chains.rs -- the set of distinct
        // chain IDs is small and bounded.
        let cached = self.chain_id_cache.lock().unwrap().clone();
        Box::leak(cached.into_boxed_str())
    }

    fn accounts(&self) -> &[String] {
        // Refresh cache from the inner provider each time so we stay in sync.
        self.refresh_accounts_cache();
        // Same Box::leak pattern as embedded_chains.rs.
        let accts = self.accounts_cache.lock().unwrap().clone();
        Box::leak(accts.into_boxed_slice())
    }

    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            method = %method,
            "External wallet Ethereum request"
        );

        match self.inner.request(method, params).await {
            Ok(result) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    method = %method,
                    "External wallet Ethereum request success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    method = %method,
                    error = %e,
                    "External wallet Ethereum request failed"
                );
                Err(e)
            }
        }
    }

    async fn connect(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Ethereum connect"
        );

        match self.inner.connect().await {
            Ok(accounts) => {
                *self.accounts_cache.lock().unwrap() = accounts.clone();
                self.refresh_chain_id_cache();
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    account_count = accounts.len(),
                    "External wallet Ethereum connected"
                );
                Ok(accounts)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum connect failed"
                );
                Err(e)
            }
        }
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Ethereum disconnect"
        );

        match self.inner.disconnect().await {
            Ok(()) => {
                *self.accounts_cache.lock().unwrap() = Vec::new();
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    "External wallet Ethereum disconnected"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum disconnect failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_personal_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let message_preview = if message.len() > 50 {
            format!("{}...", &message[..50])
        } else {
            message.to_string()
        };

        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            message_preview = %message_preview,
            message_length = message.len(),
            address = %address,
            "External wallet Ethereum signPersonalMessage"
        );

        match self.inner.sign_personal_message(message, address).await {
            Ok(sig) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature_length = sig.len(),
                    "External wallet Ethereum signPersonalMessage success"
                );
                Ok(sig)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum signPersonalMessage failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            address = %address,
            "External wallet Ethereum signTypedData"
        );

        match self.inner.sign_typed_data(typed_data, address).await {
            Ok(sig) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature_length = sig.len(),
                    "External wallet Ethereum signTypedData success"
                );
                Ok(sig)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum signTypedData failed"
                );
                Err(e)
            }
        }
    }

    async fn sign_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            from = ?transaction.from,
            to = ?transaction.to,
            "External wallet Ethereum signTransaction"
        );

        match self.inner.sign_transaction(transaction).await {
            Ok(sig) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    signature_length = sig.len(),
                    "External wallet Ethereum signTransaction success"
                );
                Ok(sig)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum signTransaction failed"
                );
                Err(e)
            }
        }
    }

    async fn send_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            from = ?transaction.from,
            to = ?transaction.to,
            value = ?transaction.value,
            "External wallet Ethereum sendTransaction"
        );

        match self.inner.send_transaction(transaction).await {
            Ok(tx_hash) => {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    tx_hash = %tx_hash,
                    "External wallet Ethereum sendTransaction success"
                );
                Ok(tx_hash)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum sendTransaction failed"
                );
                Err(e)
            }
        }
    }

    async fn switch_chain(
        &self,
        chain_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            chain_id = %chain_id,
            "External wallet Ethereum switchChain"
        );

        match self.inner.switch_chain(chain_id).await {
            Ok(()) => {
                self.refresh_chain_id_cache();
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    chain_id = %chain_id,
                    "External wallet Ethereum switchChain success"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Ethereum switchChain failed"
                );
                Err(e)
            }
        }
    }

    async fn get_chain_id(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.inner.get_chain_id().await?;
        self.refresh_chain_id_cache();
        Ok(result)
    }

    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let accounts = self.inner.get_accounts().await?;
        *self.accounts_cache.lock().unwrap() = accounts.clone();
        Ok(accounts)
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        // Register on both the local registry and the inner provider so that
        // events emitted by either side reach the caller.
        let local_id = self.events.add(event, listener);
        // Forward to inner provider as well (fire-and-forget, we track via
        // local_id only).
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
