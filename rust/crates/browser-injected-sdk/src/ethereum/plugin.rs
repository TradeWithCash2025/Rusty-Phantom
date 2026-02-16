//! Ethereum plugin for the Phantom browser-injected SDK.
//!
//! Provides the `Ethereum` struct that implements `EthereumChain` from chain-interfaces,
//! and the `create_ethereum_plugin` factory function.

use super::events::EthereumEventListeners;
use super::operations;
use super::strategy::EthereumStrategy;
use super::types::{EthereumEventType, EthereumTransaction};
use crate::Plugin;
use phantom_chain_interfaces::{EthTransactionRequest, EthereumChain};
use std::sync::Arc;
use std::sync::RwLock;

/// Phantom Ethereum chain implementation that is EIP-1193 compliant.
///
/// Wraps an `EthereumStrategy` with event listeners and state management.
pub struct Ethereum {
    strategy: Arc<dyn EthereumStrategy>,
    events: Arc<EthereumEventListeners>,
    /// Stored behind `std::sync::RwLock` so that synchronous trait methods and
    /// `handle_provider_event` can access it without an async runtime.
    chain_id: RwLock<String>,
    /// Stored behind `std::sync::RwLock` for the same reason as `chain_id`.
    accounts: RwLock<Vec<String>>,
}

impl Ethereum {
    /// Create a new Ethereum instance with the given strategy.
    ///
    /// Internally calls [`bind_provider_events`](Self::bind_provider_events) to
    /// prepare the event bridge (matching the TS constructor behaviour).
    pub fn new(strategy: Arc<dyn EthereumStrategy>) -> Self {
        let instance = Self {
            strategy,
            events: Arc::new(EthereumEventListeners::new()),
            chain_id: RwLock::new("0x1".to_string()),
            accounts: RwLock::new(vec![]),
        };
        instance.bind_provider_events();
        instance
    }

    /// Get a reference to the event listener registry.
    pub fn events(&self) -> &EthereumEventListeners {
        &self.events
    }

    /// Prepare the native provider event bridge.
    ///
    /// In a browser context the TS `Ethereum` constructor calls `bindProviderEvents()`
    /// to forward native wallet events (`connect`, `disconnect`, `accountsChanged`,
    /// `chainChanged`) into the SDK event system.  In Rust we cannot register
    /// native listeners directly, so the actual bridging is performed by the
    /// platform layer calling [`handle_provider_event`](Self::handle_provider_event).
    /// This method exists to mirror the TS constructor flow and can be extended
    /// later when a platform bridge is available.
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
    /// - `"connect"` -- triggers the connect event with the provided data.
    /// - `"disconnect"` -- clears accounts, triggers disconnect with
    ///   `ProviderRpcError { code: 4900, message: "Provider disconnected" }`.
    /// - `"accountsChanged"` -- `data` should be a JSON array of account strings.
    ///   Updates internal accounts and triggers `accountsChanged`. If the new
    ///   account list is non-empty, also triggers `connect` (dual-trigger
    ///   behaviour matching the TS implementation).
    /// - `"chainChanged"` -- `data` should contain the new chain ID string.
    ///   Updates internal chain ID and triggers `chainChanged`.
    pub fn handle_provider_event(&self, event: &str, data: serde_json::Value) {
        match event {
            "connect" => {
                // In TS: fetches accounts and updates state.
                // Here we just trigger the event; account fetching should be done
                // by the caller.
                self.events.trigger_event(EthereumEventType::Connect, data);
            }
            "disconnect" => {
                if let Ok(mut guard) = self.accounts.write() {
                    guard.clear();
                }
                // TS creates ProviderRpcError { code: 4900, message: "Provider disconnected" }
                let error_data = serde_json::json!({
                    "code": 4900,
                    "message": "Provider disconnected"
                });
                self.events
                    .trigger_event(EthereumEventType::Disconnect, error_data);
            }
            "accountsChanged" => {
                if let Ok(accounts) = serde_json::from_value::<Vec<String>>(data.clone()) {
                    let has_accounts = !accounts.is_empty();
                    if let Ok(mut guard) = self.accounts.write() {
                        *guard = accounts;
                    }
                    self.events
                        .trigger_event(EthereumEventType::AccountsChanged, data.clone());
                    // Dual-trigger: if accounts exist, also trigger connect
                    if has_accounts {
                        self.events.trigger_event(EthereumEventType::Connect, data);
                    }
                }
            }
            "chainChanged" => {
                if let Some(chain_id) = data.as_str() {
                    if let Ok(mut guard) = self.chain_id.write() {
                        *guard = chain_id.to_string();
                    }
                }
                self.events
                    .trigger_event(EthereumEventType::ChainChanged, data);
            }
            _ => {}
        }
    }

    /// Return an owned copy of the current chain ID.
    ///
    /// The [`EthereumChain::chain_id`] trait method returns `&str`, which cannot
    /// borrow through the internal lock.  Use this method when you need the
    /// actual runtime value.
    pub fn get_chain_id_owned(&self) -> String {
        self.chain_id
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| "0x1".to_string())
    }

    /// Return an owned copy of the current accounts list.
    ///
    /// The [`EthereumChain::accounts`] trait method returns `&[String]`, which
    /// cannot borrow through the internal lock.  Use this method when you need
    /// the actual runtime value.
    pub fn get_accounts_owned(&self) -> Vec<String> {
        self.accounts
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl EthereumChain for Ethereum {
    fn chain_id(&self) -> &str {
        // The trait requires `&str`, but we cannot return a reference to data
        // behind a lock guard whose lifetime is shorter than `&self`.
        // Use `Ethereum::get_chain_id_owned()` to obtain an owned `String`.
        "0x1"
    }

    fn accounts(&self) -> &[String] {
        // The trait requires `&[String]`, but we cannot return a reference to
        // data behind a lock guard whose lifetime is shorter than `&self`.
        // Use `Ethereum::get_accounts_owned()` to obtain an owned `Vec<String>`.
        &[]
    }

    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        self.strategy.request(method, params).await
    }

    async fn connect(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let accounts = operations::connect(self.strategy.as_ref(), &self.events, false).await?;
        if let Ok(mut guard) = self.accounts.write() {
            *guard = accounts.clone();
        }
        Ok(accounts)
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        operations::disconnect(self.strategy.as_ref(), &self.events).await?;
        if let Ok(mut guard) = self.accounts.write() {
            *guard = vec![];
        }
        Ok(())
    }

    async fn sign_personal_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        operations::sign_personal_message(self.strategy.as_ref(), message, address).await
    }

    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        operations::sign_typed_data(self.strategy.as_ref(), typed_data, address).await
    }

    async fn sign_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Convert chain-interfaces EthTransactionRequest to our local EthereumTransaction
        let local_tx = EthereumTransaction {
            to: transaction.to.clone(),
            from: transaction.from.clone(),
            value: transaction.value.clone(),
            gas: transaction.gas.clone(),
            gas_price: transaction.gas_price.clone(),
            max_fee_per_gas: transaction.max_fee_per_gas.clone(),
            max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.clone(),
            data: transaction.data.clone(),
            nonce: transaction.nonce.clone(),
            tx_type: transaction.tx_type.clone(),
            chain_id: transaction.chain_id.clone(),
        };
        operations::sign_transaction(self.strategy.as_ref(), &local_tx).await
    }

    async fn send_transaction(
        &self,
        transaction: &EthTransactionRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let local_tx = EthereumTransaction {
            to: transaction.to.clone(),
            from: transaction.from.clone(),
            value: transaction.value.clone(),
            gas: transaction.gas.clone(),
            gas_price: transaction.gas_price.clone(),
            max_fee_per_gas: transaction.max_fee_per_gas.clone(),
            max_priority_fee_per_gas: transaction.max_priority_fee_per_gas.clone(),
            data: transaction.data.clone(),
            nonce: transaction.nonce.clone(),
            tx_type: transaction.tx_type.clone(),
            chain_id: transaction.chain_id.clone(),
        };
        operations::send_transaction(self.strategy.as_ref(), &local_tx).await
    }

    async fn switch_chain(
        &self,
        chain_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Normalize to hex: if it's a decimal string or already hex, handle both.
        let hex_chain_id = if chain_id.starts_with("0x") || chain_id.starts_with("0X") {
            chain_id.to_lowercase()
        } else if let Ok(num) = chain_id.parse::<u64>() {
            format!("0x{:x}", num)
        } else {
            chain_id.to_string()
        };
        operations::switch_chain(self.strategy.as_ref(), &hex_chain_id).await?;
        if let Ok(mut guard) = self.chain_id.write() {
            *guard = hex_chain_id;
        }
        Ok(())
    }

    async fn get_chain_id(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let chain_id_hex = operations::get_chain_id(self.strategy.as_ref()).await?;
        let chain_id_str = chain_id_hex.trim_start_matches("0x");
        let parsed = u64::from_str_radix(chain_id_str, 16)?;
        if let Ok(mut guard) = self.chain_id.write() {
            *guard = chain_id_hex;
        }
        Ok(parsed)
    }

    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let accounts = operations::get_accounts(self.strategy.as_ref()).await?;
        if let Ok(mut guard) = self.accounts.write() {
            *guard = accounts.clone();
        }
        Ok(accounts)
    }

    fn is_connected(&self) -> bool {
        self.strategy.is_connected()
    }

    fn on(&self, event: &str, listener: Box<dyn Fn(serde_json::Value) + Send + Sync>) -> u64 {
        self.events.add_listener_by_name(event, listener)
    }

    fn off(&self, event: &str, listener_id: u64) {
        self.events.remove_listener_by_name(event, listener_id);
    }
}

/// Create an Ethereum plugin for the Phantom instance.
pub fn create_ethereum_plugin(strategy: Arc<dyn EthereumStrategy>) -> Plugin {
    Plugin {
        name: "ethereum".to_string(),
        create: Box::new(move || Box::new(Ethereum::new(strategy.clone()))),
    }
}
