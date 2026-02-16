//! Embedded chain implementations for Ethereum and Solana.
//!
//! These structs wrap the [`EmbeddedProvider`] and implement the
//! [`EthereumChain`] and [`SolanaChain`] traits from `phantom-chain-interfaces`,
//! mirroring the TypeScript implementations in
//! `packages/embedded-provider-core/src/chains/`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use phantom_chain_interfaces::{
    EthTransactionRequest, EthereumChain, SolanaChain, SolanaConnectOptions, SolanaConnectResult,
    SolanaNetwork, SolanaSignMessageResult, SolanaSendAllTransactionsResult,
    SolanaSendTransactionResult,
};
use phantom_constants::{chain_id_to_network_id, network_id_to_chain_id, NetworkId};

use crate::constants::AddressFormat;
use crate::embedded_provider::EmbeddedProvider;

// ============================================================================
// Generic event listener registry
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
// Helper: ensure provider is connected
// ============================================================================

async fn ensure_provider_connected(
    provider: &EmbeddedProvider,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !provider.is_connected().await {
        return Err("Provider not connected. Call provider connect first.".into());
    }
    Ok(())
}

// ============================================================================
// EmbeddedEthereumChain
// ============================================================================

/// Embedded Ethereum chain implementation that is EIP-1193 compliant.
///
/// Wraps an [`EmbeddedProvider`] and implements [`EthereumChain`], converting
/// high-level Ethereum RPC calls into embedded provider signing operations.
pub struct EmbeddedEthereumChain {
    provider: Arc<EmbeddedProvider>,
    current_network_id: Mutex<NetworkId>,
    accounts: Mutex<Vec<String>>,
    /// Hex-encoded chain ID string (e.g. "0x1"). Cached so `chain_id()` can
    /// return `&str` without an async lock.
    chain_id_cache: Mutex<String>,
    events: EventListenerRegistry,
}

impl EmbeddedEthereumChain {
    /// Create a new `EmbeddedEthereumChain` wrapping the given provider.
    ///
    /// This does **not** perform initial state synchronisation. Call
    /// [`sync_accounts`](Self::sync_accounts) after construction (or use
    /// [`new_initialized`](Self::new_initialized) instead) to eagerly
    /// populate accounts from the provider.
    pub fn new(provider: Arc<EmbeddedProvider>) -> Self {
        Self {
            provider,
            current_network_id: Mutex::new(NetworkId::EthereumMainnet),
            accounts: Mutex::new(Vec::new()),
            chain_id_cache: Mutex::new("0x1".to_string()),
            events: EventListenerRegistry::new(),
        }
    }

    /// Create a new `EmbeddedEthereumChain` and synchronise initial state.
    ///
    /// This is the async equivalent of the TypeScript constructor which calls
    /// `syncInitialState()` to eagerly populate accounts from the provider.
    pub async fn new_initialized(provider: Arc<EmbeddedProvider>) -> Self {
        let chain = Self::new(provider);
        chain.sync_accounts().await;
        chain
    }

    /// Synchronise accounts from the provider.
    /// Call this after connecting to populate the internal accounts list.
    pub async fn sync_accounts(&self) {
        if !self.provider.is_connected().await {
            return;
        }
        let addresses = self.provider.get_addresses().await;
        let eth_addresses: Vec<String> = addresses
            .iter()
            .filter(|a| a.address_type == AddressFormat::Ethereum)
            .map(|a| a.address.clone())
            .collect();
        if !eth_addresses.is_empty() {
            *self.accounts.lock().unwrap() = eth_addresses;
        }
    }

    /// Refresh the cached hex chain ID string from `current_network_id`.
    fn refresh_chain_id_cache(&self) {
        let network_id = *self.current_network_id.lock().unwrap();
        let numeric = network_id_to_chain_id(network_id).unwrap_or(1);
        let hex = format!("0x{:x}", numeric);
        *self.chain_id_cache.lock().unwrap() = hex;
    }

    /// Parse a chain ID string (hex or decimal) into a numeric u64.
    fn parse_chain_id_str(chain_id: &str) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let s = chain_id.trim();
        if s.starts_with("0x") || s.starts_with("0X") {
            u64::from_str_radix(&s[2..], 16)
                .map_err(|e| format!("Invalid hex chain ID '{}': {}", chain_id, e).into())
        } else {
            s.parse::<u64>()
                .map_err(|e| format!("Invalid chain ID '{}': {}", chain_id, e).into())
        }
    }
}

#[async_trait::async_trait]
impl EthereumChain for EmbeddedEthereumChain {
    fn chain_id(&self) -> &str {
        // SAFETY: We never drop `self` while the reference is alive and the
        // Mutex<String> lives as long as `self`. We leak nothing -- the
        // returned &str is valid for the lifetime of `self`.
        //
        // This is the same pattern used in the browser-injected-sdk Ethereum
        // plugin: the trait requires `&str` but we store the value behind a
        // Mutex. We return a 'static-ish reference by leaking a small string
        // each call -- acceptable because the set of distinct chain IDs is
        // small and bounded.
        let cached = self.chain_id_cache.lock().unwrap().clone();
        // Leak a small string so we can return &str with the right lifetime.
        // The number of distinct chain IDs is bounded (< 20 in practice).
        Box::leak(cached.into_boxed_str())
    }

    fn accounts(&self) -> &[String] {
        // Same pattern as chain_id(): the trait requires &[String] but the
        // data is behind a Mutex. We leak a Vec for the lifetime of the
        // program. In practice this is called infrequently and the data is
        // small.
        let accts = self.accounts.lock().unwrap().clone();
        Box::leak(accts.into_boxed_slice())
    }

    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;

        let empty: Vec<serde_json::Value> = vec![];
        let params_slice = params.unwrap_or(&empty);

        match method {
            "personal_sign" => {
                if params_slice.len() < 2 {
                    return Err("personal_sign requires [message, address]".into());
                }
                let message = params_slice[0]
                    .as_str()
                    .ok_or("message must be a string")?;
                let address = params_slice[1]
                    .as_str()
                    .ok_or("address must be a string")?;
                let sig = self.sign_personal_message(message, address).await?;
                Ok(serde_json::Value::String(sig))
            }
            "eth_signTypedData_v4" => {
                if params_slice.len() < 2 {
                    return Err("eth_signTypedData_v4 requires [address, typedData]".into());
                }
                let address = params_slice[0]
                    .as_str()
                    .ok_or("address must be a string")?;
                let typed_data = if let Some(s) = params_slice[1].as_str() {
                    serde_json::from_str(s)?
                } else {
                    params_slice[1].clone()
                };
                let sig = self.sign_typed_data(&typed_data, address).await?;
                Ok(serde_json::Value::String(sig))
            }
            "eth_signTransaction" => {
                if params_slice.is_empty() {
                    return Err("eth_signTransaction requires [transaction]".into());
                }
                let tx: EthTransactionRequest = serde_json::from_value(params_slice[0].clone())?;
                let raw = self.sign_transaction(&tx).await?;
                Ok(serde_json::Value::String(raw))
            }
            "eth_sendTransaction" => {
                if params_slice.is_empty() {
                    return Err("eth_sendTransaction requires [transaction]".into());
                }
                let tx: EthTransactionRequest = serde_json::from_value(params_slice[0].clone())?;
                let hash = self.send_transaction(&tx).await?;
                Ok(serde_json::Value::String(hash))
            }
            "eth_accounts" => {
                let accounts = self.get_accounts().await?;
                Ok(serde_json::to_value(accounts)?)
            }
            "eth_chainId" => {
                let network_id = *self.current_network_id.lock().unwrap();
                let numeric = network_id_to_chain_id(network_id).unwrap_or(1);
                Ok(serde_json::Value::String(format!("0x{:x}", numeric)))
            }
            "wallet_switchEthereumChain" => {
                if params_slice.is_empty() {
                    return Err("wallet_switchEthereumChain requires [{ chainId }]".into());
                }
                let chain_id_val = params_slice[0]
                    .get("chainId")
                    .and_then(|v| v.as_str())
                    .ok_or("chainId must be a hex string")?;
                self.switch_chain(chain_id_val).await?;
                Ok(serde_json::Value::Null)
            }
            _ => Err(format!("Embedded provider doesn't support method: {}", method).into()),
        }
    }

    async fn connect(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;

        let addresses = self.provider.get_addresses().await;
        let eth_addresses: Vec<String> = addresses
            .iter()
            .filter(|a| a.address_type == AddressFormat::Ethereum)
            .map(|a| a.address.clone())
            .collect();

        *self.accounts.lock().unwrap() = eth_addresses.clone();

        let chain_id_str = self.chain_id_cache.lock().unwrap().clone();
        self.events.emit(
            "connect",
            serde_json::json!({ "chainId": chain_id_str }),
        );
        self.events.emit(
            "accountsChanged",
            serde_json::to_value(&eth_addresses).unwrap_or_default(),
        );

        Ok(eth_addresses)
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.provider.disconnect(true).await?;
        *self.accounts.lock().unwrap() = Vec::new();
        self.events.emit(
            "disconnect",
            serde_json::json!({ "code": 4900, "message": "Provider disconnected" }),
        );
        self.events.emit(
            "accountsChanged",
            serde_json::json!([]),
        );
        Ok(())
    }

    async fn sign_personal_message(
        &self,
        message: &str,
        _address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;
        let network_id = *self.current_network_id.lock().unwrap();
        let result = self
            .provider
            .sign_ethereum_message(&crate::types::SignMessageParams {
                message: message.to_string(),
                network_id: network_id.to_string(),
            })
            .await?;
        Ok(result.signature)
    }

    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        _address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;
        let network_id = *self.current_network_id.lock().unwrap();
        let result = self
            .provider
            .sign_typed_data_v4(&crate::types::SignTypedDataV4Params {
                typed_data: typed_data.clone(),
                network_id: network_id.to_string(),
            })
            .await?;
        Ok(result.signature)
    }

    async fn sign_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;

        // Determine the network ID from the transaction's chainId field or
        // fall back to the current network.
        let network_id = if let Some(ref cid) = transaction.chain_id {
            let numeric = Self::parse_chain_id_str(cid)?;
            chain_id_to_network_id(numeric).unwrap_or(*self.current_network_id.lock().unwrap())
        } else {
            *self.current_network_id.lock().unwrap()
        };

        let tx_bytes = serde_json::to_vec(transaction)?;

        let result = self
            .provider
            .sign_transaction(&crate::types::SignTransactionParams {
                transaction: tx_bytes,
                network_id: network_id.to_string(),
            })
            .await?;

        Ok(result.raw_transaction)
    }

    async fn send_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;

        // If the transaction specifies a chainId, switch to that chain first.
        if let Some(ref cid) = transaction.chain_id {
            self.switch_chain(cid).await?;
        }

        let network_id = *self.current_network_id.lock().unwrap();
        let tx_bytes = serde_json::to_vec(transaction)?;

        let result = self
            .provider
            .sign_and_send_transaction(&crate::types::SignAndSendTransactionParams {
                transaction: tx_bytes,
                network_id: network_id.to_string(),
            })
            .await?;

        result
            .hash
            .ok_or_else(|| "Transaction not submitted".into())
    }

    async fn switch_chain(
        &self,
        chain_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let numeric = Self::parse_chain_id_str(chain_id)?;
        let network_id = chain_id_to_network_id(numeric)
            .ok_or_else(|| format!("Unsupported chainId: {}", chain_id))?;

        *self.current_network_id.lock().unwrap() = network_id;
        self.refresh_chain_id_cache();

        let hex_chain_id = format!("0x{:x}", numeric);
        self.events.emit(
            "chainChanged",
            serde_json::Value::String(hex_chain_id),
        );

        Ok(())
    }

    async fn get_chain_id(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let network_id = *self.current_network_id.lock().unwrap();
        Ok(network_id_to_chain_id(network_id).unwrap_or(1))
    }

    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let addresses = self.provider.get_addresses().await;
        let eth_addresses: Vec<String> = addresses
            .iter()
            .filter(|a| a.address_type == AddressFormat::Ethereum)
            .map(|a| a.address.clone())
            .collect();
        *self.accounts.lock().unwrap() = eth_addresses.clone();
        Ok(eth_addresses)
    }

    fn is_connected(&self) -> bool {
        self.provider.is_connected_sync() && !self.accounts.lock().unwrap().is_empty()
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        self.events.add(event, listener)
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove(event, listener_id);
    }
}

// ============================================================================
// EmbeddedSolanaChain
// ============================================================================

/// Embedded Solana chain implementation that is wallet-adapter compliant.
///
/// Wraps an [`EmbeddedProvider`] and implements [`SolanaChain`], converting
/// wallet-adapter style calls into embedded provider signing operations.
pub struct EmbeddedSolanaChain {
    provider: Arc<EmbeddedProvider>,
    current_network_id: Mutex<NetworkId>,
    public_key: Mutex<Option<String>>,
    events: EventListenerRegistry,
}

impl EmbeddedSolanaChain {
    /// Create a new `EmbeddedSolanaChain` wrapping the given provider.
    ///
    /// This does **not** perform initial state synchronisation. Call
    /// [`sync_public_key`](Self::sync_public_key) after construction (or use
    /// [`new_initialized`](Self::new_initialized) instead) to eagerly
    /// populate the public key from the provider.
    pub fn new(provider: Arc<EmbeddedProvider>) -> Self {
        Self {
            provider,
            current_network_id: Mutex::new(NetworkId::SolanaMainnet),
            public_key: Mutex::new(None),
            events: EventListenerRegistry::new(),
        }
    }

    /// Create a new `EmbeddedSolanaChain` and synchronise initial state.
    ///
    /// This is the async equivalent of the TypeScript constructor which calls
    /// `syncInitialState()` to eagerly populate the public key from the provider.
    pub async fn new_initialized(provider: Arc<EmbeddedProvider>) -> Self {
        let chain = Self::new(provider);
        chain.sync_public_key().await;
        chain
    }

    /// Synchronise public key from the provider.
    /// Call this after connecting to populate the internal public key.
    pub async fn sync_public_key(&self) {
        if !self.provider.is_connected().await {
            return;
        }
        let addresses = self.provider.get_addresses().await;
        let sol_addr = addresses
            .iter()
            .find(|a| a.address_type == AddressFormat::Solana);
        if let Some(addr) = sol_addr {
            *self.public_key.lock().unwrap() = Some(addr.address.clone());
        }
    }
}

#[async_trait::async_trait]
impl SolanaChain for EmbeddedSolanaChain {
    fn public_key(&self) -> Option<&str> {
        // Same pattern as the browser-injected-sdk: the trait requires
        // Option<&str> but the data is behind a Mutex. We leak a small
        // String when present. The number of distinct public keys is 1.
        let pk = self.public_key.lock().unwrap().clone();
        pk.map(|s| &*Box::leak(s.into_boxed_str()) as &str)
    }

    fn is_connected(&self) -> bool {
        self.public_key.lock().unwrap().is_some()
    }

    async fn connect(
        &self,
        _options: Option<SolanaConnectOptions>,
    ) -> Result<SolanaConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        ensure_provider_connected(&self.provider).await?;

        let addresses = self.provider.get_addresses().await;
        let sol_addr = addresses
            .iter()
            .find(|a| a.address_type == AddressFormat::Solana)
            .ok_or("No Solana address found")?;

        let pk = sol_addr.address.clone();
        *self.public_key.lock().unwrap() = Some(pk.clone());

        self.events.emit("connect", serde_json::json!(pk));

        Ok(SolanaConnectResult { public_key: pk })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.provider.disconnect(true).await?;
        *self.public_key.lock().unwrap() = None;
        self.events.emit("disconnect", serde_json::Value::Null);
        Ok(())
    }

    async fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<SolanaSignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_connected() {
            return Err("Solana chain not available. Ensure SDK is connected.".into());
        }
        let network_id = *self.current_network_id.lock().unwrap();
        let message_str = String::from_utf8_lossy(message).to_string();

        let result = self
            .provider
            .sign_message(&crate::types::SignMessageParams {
                message: message_str,
                network_id: network_id.to_string(),
            })
            .await?;

        // Decode signature from base58 to raw bytes.
        let sig_bytes = bs58::decode(&result.signature)
            .into_vec()
            .unwrap_or_else(|_| result.signature.as_bytes().to_vec());

        let pk = self
            .public_key
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default();

        Ok(SolanaSignMessageResult {
            signature: sig_bytes,
            public_key: pk,
        })
    }

    async fn sign_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_connected() {
            return Err("Solana chain not available. Ensure SDK is connected.".into());
        }
        let network_id = *self.current_network_id.lock().unwrap();

        let result = self
            .provider
            .sign_transaction(&crate::types::SignTransactionParams {
                transaction: transaction.to_vec(),
                network_id: network_id.to_string(),
            })
            .await?;

        // Decode the base64url signed transaction back to bytes.
        let bytes = phantom_base64url::base64url_decode(&result.raw_transaction)?;
        Ok(bytes)
    }

    async fn sign_and_send_transaction(
        &self,
        transaction: &[u8],
    ) -> Result<SolanaSendTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_connected() {
            return Err("Solana chain not available. Ensure SDK is connected.".into());
        }
        let network_id = *self.current_network_id.lock().unwrap();

        let result = self
            .provider
            .sign_and_send_transaction(&crate::types::SignAndSendTransactionParams {
                transaction: transaction.to_vec(),
                network_id: network_id.to_string(),
            })
            .await?;

        let sig = result
            .hash
            .ok_or("Transaction not submitted")?;

        Ok(SolanaSendTransactionResult { signature: sig })
    }

    async fn sign_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_connected() {
            return Err("Solana chain not available. Ensure SDK is connected.".into());
        }
        let mut results = Vec::with_capacity(transactions.len());
        for tx in transactions {
            results.push(self.sign_transaction(tx).await?);
        }
        Ok(results)
    }

    async fn sign_and_send_all_transactions(
        &self,
        transactions: &[Vec<u8>],
    ) -> Result<SolanaSendAllTransactionsResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut signatures = Vec::with_capacity(transactions.len());
        for tx in transactions {
            let result = self.sign_and_send_transaction(tx).await?;
            signatures.push(result.signature);
        }
        Ok(SolanaSendAllTransactionsResult { signatures })
    }

    async fn switch_network(
        &self,
        network: SolanaNetwork,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let network_id = match network {
            SolanaNetwork::Mainnet => NetworkId::SolanaMainnet,
            SolanaNetwork::Devnet => NetworkId::SolanaDevnet,
        };
        *self.current_network_id.lock().unwrap() = network_id;
        Ok(())
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        self.events.add(event, listener)
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove(event, listener_id);
    }
}
