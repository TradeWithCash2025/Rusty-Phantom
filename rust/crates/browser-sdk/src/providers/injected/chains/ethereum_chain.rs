//! Injected wallet Ethereum chain wrapper.
//!
//! Wraps an external [`EthereumChain`] provider (e.g. an EIP-6963 wallet)
//! and adds debug logging and event forwarding, mirroring the TypeScript
//! `InjectedWalletEthereumChain` in
//! `packages/browser-sdk/src/providers/injected/chains/InjectedWalletEthereumChain.ts`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
// Helper: extract an error code from a boxed error.
//
// Provider errors typically contain a JSON-RPC `code` field.  We try:
//   1. Down-casting to `serde_json::Value` in case the error wraps one.
//   2. Searching the `Display` representation for `"code":4100` or similar.
//
// This is intentionally best-effort -- if the provider uses a different error
// representation we simply will not retry.
// ============================================================================

fn extract_error_code(err: &(dyn std::error::Error + Send + Sync)) -> Option<i64> {
    // Check Display output for a JSON code field.
    let msg = err.to_string();
    // Try simple pattern: "code": 4100 or "code":4100
    if let Some(pos) = msg.find("\"code\"") {
        let after = &msg[pos + 6..];
        // Skip optional whitespace and colon
        let after = after.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
        if let Some(end) = after.find(|c: char| !c.is_ascii_digit() && c != '-') {
            if let Ok(code) = after[..end].parse::<i64>() {
                return Some(code);
            }
        } else if let Ok(code) = after.parse::<i64>() {
            return Some(code);
        }
    }
    // Fallback: look for "4100" anywhere preceded by common patterns.
    // This handles messages like "Error 4100: Unauthorized" or "code 4100".
    for pattern in &["code 4100", "code: 4100", "error 4100", "Error(4100"] {
        if msg.contains(pattern) {
            return Some(4100);
        }
    }
    None
}

/// Methods that require authorisation (the wallet must be connected).
const AUTH_METHODS: &[&str] = &[
    "personal_sign",
    "eth_sign",
    "eth_signTypedData",
    "eth_signTypedData_v4",
    "eth_sendTransaction",
    "eth_signTransaction",
];

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
    /// Whether we believe the wallet is connected.
    connected: AtomicBool,
    /// Cached chain ID so `chain_id()` can return `&str`.
    chain_id_cache: Mutex<String>,
    /// Cached accounts so `accounts()` can return `&[String]`.
    accounts_cache: Mutex<Vec<String>>,
    /// Local event listener registry for this wrapper.
    events: EventListenerRegistry,
}

impl InjectedWalletEthereumChain {
    /// Create a new `InjectedWalletEthereumChain` wrapping the given provider.
    pub fn new(inner: Arc<dyn EthereumChain>, wallet_id: String, wallet_name: String) -> Arc<Self> {
        // Seed the caches from the inner provider.
        let initial_accounts: Vec<String> = inner.accounts().to_vec();
        let connected = AtomicBool::new(!initial_accounts.is_empty());
        let chain_id_cache = Mutex::new(inner.chain_id().to_string());
        let accounts_cache = Mutex::new(initial_accounts);

        let this = Arc::new(Self {
            inner,
            wallet_id,
            wallet_name,
            connected,
            chain_id_cache,
            accounts_cache,
            events: EventListenerRegistry::new(),
        });

        this.setup_event_listeners();
        this
    }

    /// Refresh the chain ID cache from the inner provider.
    fn refresh_chain_id_cache(&self) {
        *self.chain_id_cache.lock().unwrap() = self.inner.chain_id().to_string();
    }

    /// Refresh the accounts cache from the inner provider.
    fn refresh_accounts_cache(&self) {
        *self.accounts_cache.lock().unwrap() = self.inner.accounts().to_vec();
    }

    /// Register listeners on the inner provider to update local state and
    /// re-emit events through our own registry. Mirrors `setupEventListeners`
    /// in the TypeScript implementation.
    fn setup_event_listeners(self: &Arc<Self>) {
        // "connect" -- update connected flag and chain ID cache.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "connect",
                Box::new(move |value| {
                    this.connected.store(true, Ordering::Relaxed);
                    // The value should contain a `chainId` field.
                    if let Some(chain_id) = value.get("chainId").and_then(|v| v.as_str()) {
                        *this.chain_id_cache.lock().unwrap() = chain_id.to_string();
                    }
                    this.events.emit("connect", value);
                }),
            );
        }

        // "disconnect" -- clear connected flag and accounts.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "disconnect",
                Box::new(move |value| {
                    this.connected.store(false, Ordering::Relaxed);
                    *this.accounts_cache.lock().unwrap() = Vec::new();
                    this.events.emit("disconnect", value);
                    this.events
                        .emit("accountsChanged", serde_json::Value::Array(vec![]));
                }),
            );
        }

        // "accountsChanged" -- update accounts cache and connected flag.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "accountsChanged",
                Box::new(move |value| {
                    if let Some(arr) = value.as_array() {
                        let accounts: Vec<String> = arr
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        this.connected
                            .store(!accounts.is_empty(), Ordering::Relaxed);
                        *this.accounts_cache.lock().unwrap() = accounts;
                    }
                    this.events.emit("accountsChanged", value);
                }),
            );
        }

        // "chainChanged" -- update chain ID cache.
        {
            let this = Arc::clone(self);
            self.inner.on(
                "chainChanged",
                Box::new(move |value| {
                    if let Some(chain_id) = value.as_str() {
                        *this.chain_id_cache.lock().unwrap() = chain_id.to_string();
                    }
                    this.events.emit("chainChanged", value);
                }),
            );
        }
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

        // For methods that require authorization, ensure we're connected first.
        if AUTH_METHODS.contains(&method) {
            let needs_connect = !self.connected.load(Ordering::Relaxed)
                || self.accounts_cache.lock().unwrap().is_empty();
            if needs_connect {
                tracing::info!(
                    wallet_id = %self.wallet_id,
                    wallet_name = %self.wallet_name,
                    method = %method,
                    "Method requires authorization, ensuring connection"
                );
                self.connect().await?;
            }
        }

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
                // If we get 4100 (Unauthorized), try to re-authorize and retry once.
                if extract_error_code(e.as_ref()) == Some(4100) {
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        method = %method,
                        "Got 4100 Unauthorized, attempting to re-authorize"
                    );
                    let reauth_params: &[serde_json::Value] = &[];
                    match self
                        .inner
                        .request("eth_requestAccounts", Some(reauth_params))
                        .await
                    {
                        Ok(_) => {
                            // Retry the original request.
                            match self.inner.request(method, params).await {
                                Ok(result) => {
                                    tracing::info!(
                                        wallet_id = %self.wallet_id,
                                        wallet_name = %self.wallet_name,
                                        method = %method,
                                        "External wallet Ethereum request success (after re-auth)"
                                    );
                                    return Ok(result);
                                }
                                Err(retry_err) => {
                                    tracing::error!(
                                        wallet_id = %self.wallet_id,
                                        wallet_name = %self.wallet_name,
                                        method = %method,
                                        error = %retry_err,
                                        "Failed after re-authorization"
                                    );
                                    return Err(retry_err);
                                }
                            }
                        }
                        Err(reauth_err) => {
                            tracing::error!(
                                wallet_id = %self.wallet_id,
                                wallet_name = %self.wallet_name,
                                method = %method,
                                error = %reauth_err,
                                "Re-authorization request failed"
                            );
                            return Err(reauth_err);
                        }
                    }
                }

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
                self.connected
                    .store(!accounts.is_empty(), Ordering::Relaxed);
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
                self.connected.store(false, Ordering::Relaxed);
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

        // Try the direct trait method first; fall back to request("personal_sign")
        // if the inner provider does not support it.
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
                // Check if the error indicates the method is unsupported.
                let msg = e.to_string().to_lowercase();
                if msg.contains("unsupported")
                    || msg.contains("not implemented")
                    || msg.contains("not supported")
                {
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "signPersonalMessage unsupported, falling back to request(\"personal_sign\")"
                    );
                    let params = vec![
                        serde_json::Value::String(message.to_string()),
                        serde_json::Value::String(address.to_string()),
                    ];
                    let result = self.request("personal_sign", Some(&params)).await?;
                    let sig = result
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| result.to_string());
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        signature_length = sig.len(),
                        "External wallet Ethereum signPersonalMessage success (via fallback)"
                    );
                    Ok(sig)
                } else {
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

        // Try the direct trait method first; fall back to request("eth_signTypedData_v4").
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
                let msg = e.to_string().to_lowercase();
                if msg.contains("unsupported")
                    || msg.contains("not implemented")
                    || msg.contains("not supported")
                {
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "signTypedData unsupported, falling back to request(\"eth_signTypedData_v4\")"
                    );
                    let params = vec![
                        serde_json::Value::String(address.to_string()),
                        typed_data.clone(),
                    ];
                    let result = self.request("eth_signTypedData_v4", Some(&params)).await?;
                    let sig = result
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| result.to_string());
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        signature_length = sig.len(),
                        "External wallet Ethereum signTypedData success (via fallback)"
                    );
                    Ok(sig)
                } else {
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

        // Try the direct trait method first; fall back to request("eth_signTransaction").
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
                let msg = e.to_string().to_lowercase();
                if msg.contains("unsupported")
                    || msg.contains("not implemented")
                    || msg.contains("not supported")
                {
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "signTransaction unsupported, falling back to request(\"eth_signTransaction\")"
                    );
                    let tx_value = serde_json::to_value(transaction)?;
                    let params = vec![tx_value];
                    let result = self.request("eth_signTransaction", Some(&params)).await?;
                    let sig = result
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| result.to_string());
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        signature_length = sig.len(),
                        "External wallet Ethereum signTransaction success (via fallback)"
                    );
                    Ok(sig)
                } else {
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

        // Try the direct trait method first; fall back to request("eth_sendTransaction").
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
                let msg = e.to_string().to_lowercase();
                if msg.contains("unsupported")
                    || msg.contains("not implemented")
                    || msg.contains("not supported")
                {
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        "sendTransaction unsupported, falling back to request(\"eth_sendTransaction\")"
                    );
                    let tx_value = serde_json::to_value(transaction)?;
                    let params = vec![tx_value];
                    let result = self.request("eth_sendTransaction", Some(&params)).await?;
                    let tx_hash = result
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| result.to_string());
                    tracing::info!(
                        wallet_id = %self.wallet_id,
                        wallet_name = %self.wallet_name,
                        tx_hash = %tx_hash,
                        "External wallet Ethereum sendTransaction success (via fallback)"
                    );
                    Ok(tx_hash)
                } else {
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
        self.connected.load(Ordering::Relaxed)
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
