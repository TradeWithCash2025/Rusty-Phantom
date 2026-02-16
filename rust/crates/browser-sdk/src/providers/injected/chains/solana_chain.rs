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
    SolanaSendAllTransactionsResult, SolanaSendTransactionResult, SolanaSignInInput,
    SolanaSignInOutput, SolanaSignMessageResult,
};

// ============================================================================
// Event listener registry (same pattern as embedded_chains.rs)
// ============================================================================

/// A generic, thread-safe event listener registry keyed by event name strings.
struct EventListenerRegistry {
    #[allow(clippy::type_complexity)]
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
            .or_default()
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

    /// Emit an event, invoking all registered listeners with the given data.
    fn emit(&self, event: &str, data: serde_json::Value) {
        let map = self.listeners.lock().unwrap();
        if let Some(list) = map.get(event) {
            for (_, cb) in list {
                let cb = cb.clone();
                let data = data.clone();
                if let Err(e) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || cb(data)))
                {
                    tracing::error!("Error in '{}' event listener: {:?}", event, e);
                }
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
    pub fn new(inner: Arc<dyn SolanaChain>, wallet_id: String, wallet_name: String) -> Arc<Self> {
        // Seed the cache from the inner provider.
        let public_key_cache = Mutex::new(inner.public_key().map(|s| s.to_string()));

        let this = Arc::new(Self {
            inner,
            wallet_id,
            wallet_name,
            public_key_cache,
            events: EventListenerRegistry::new(),
        });

        this.setup_event_listeners();
        this
    }

    /// Refresh the public key cache from the inner provider.
    fn refresh_public_key_cache(&self) {
        *self.public_key_cache.lock().unwrap() = self.inner.public_key().map(|s| s.to_string());
    }

    /// Register listeners on the inner provider to update local state and
    /// re-emit events through our own registry. Mirrors `setupEventListeners`
    /// in the TypeScript implementation.
    fn setup_event_listeners(self: &Arc<Self>) {
        // "connect" -- update public key cache.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "connect",
                Box::new(move |value| {
                    // The value may be a string (public key) or an object with a publicKey field.
                    let pk = value.as_str().map(|s| s.to_string()).or_else(|| {
                        value
                            .get("publicKey")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
                    if let Some(ref key) = pk {
                        *this.public_key_cache.lock().unwrap() = Some(key.clone());
                    }
                    this.events.emit("connect", value);
                }),
            );
        }

        // "disconnect" -- clear public key cache.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "disconnect",
                Box::new(move |value| {
                    *this.public_key_cache.lock().unwrap() = None;
                    this.events.emit("disconnect", value);
                }),
            );
        }

        // "accountChanged" -- update public key cache.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "accountChanged",
                Box::new(move |value| {
                    let pk = value.as_str().map(|s| s.to_string()).or_else(|| {
                        value
                            .get("publicKey")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    });
                    // If the value is null or an empty string, clear the cache.
                    let pk = pk.filter(|s| !s.is_empty());
                    *this.public_key_cache.lock().unwrap() = pk;
                    this.events.emit("accountChanged", value);
                }),
            );
        }
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
        // Prefer the wrapped provider's state when available, fallback to our
        // cached key -- mirrors the TS implementation.
        self.inner.is_connected() || self.public_key_cache.lock().unwrap().is_some()
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
                // Post-connect validation: verify the provider reports connected
                // and that we got a non-empty public key.
                if !self.inner.is_connected() {
                    tracing::error!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "Provider not connected after connect() call"
                    );
                    return Err("Provider not connected after connect() call".into());
                }

                if result.public_key.is_empty() {
                    tracing::error!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "Empty publicKey from provider"
                    );
                    return Err("Empty publicKey from provider".into());
                }

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

    async fn get_account(&self) -> Option<String> {
        self.inner.get_account().await
    }

    async fn sign_in(
        &self,
        input: &SolanaSignInInput,
    ) -> Result<SolanaSignInOutput, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            wallet_id = %self.wallet_id,
            wallet_name = %self.wallet_name,
            "External wallet Solana signIn"
        );

        match self.inner.sign_in(input).await {
            Ok(result) => {
                if !result.address.is_empty() {
                    *self.public_key_cache.lock().unwrap() = Some(result.address.clone());
                }
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    address = %result.address,
                    "External wallet Solana signIn success"
                );
                Ok(result)
            }
            Err(e) => {
                tracing::error!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    error = %e,
                    "External wallet Solana signIn failed"
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
                // Normalize: if the result's publicKey is empty, fall back to
                // our cached publicKey (mirrors TS `result.publicKey || this._publicKey || ""`).
                let public_key = if result.public_key.is_empty() {
                    self.public_key_cache
                        .lock()
                        .unwrap()
                        .clone()
                        .unwrap_or_default()
                } else {
                    result.public_key
                };
                Ok(SolanaSignMessageResult {
                    signature: result.signature,
                    public_key,
                })
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

        match self
            .inner
            .sign_and_send_all_transactions(transactions)
            .await
        {
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
        // Register on the local registry only.  Events from the inner provider
        // are forwarded to local listeners via `setup_event_listeners`, so
        // callers receive events regardless of origin.
        self.events.add(event, listener)
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove(event, listener_id);
    }
}
