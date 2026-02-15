//! Ethereum plugin for the Phantom browser-injected SDK.
//!
//! Provides the `Ethereum` struct that implements `EthereumChain` from chain-interfaces,
//! and the `create_ethereum_plugin` factory function.

use super::events::EthereumEventListeners;
use super::operations;
use super::strategy::EthereumStrategy;
use super::types::EthereumTransaction;
use crate::Plugin;
use phantom_chain_interfaces::{EthTransactionRequest, EthereumChain};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Phantom Ethereum chain implementation that is EIP-1193 compliant.
///
/// Wraps an `EthereumStrategy` with event listeners and state management.
pub struct Ethereum {
    strategy: Arc<dyn EthereumStrategy>,
    events: Arc<EthereumEventListeners>,
    chain_id: RwLock<String>,
    accounts: RwLock<Vec<String>>,
}

impl Ethereum {
    /// Create a new Ethereum instance with the given strategy.
    pub fn new(strategy: Arc<dyn EthereumStrategy>) -> Self {
        Self {
            strategy,
            events: Arc::new(EthereumEventListeners::new()),
            chain_id: RwLock::new("0x1".to_string()),
            accounts: RwLock::new(vec![]),
        }
    }

    /// Get a reference to the event listener registry.
    pub fn events(&self) -> &EthereumEventListeners {
        &self.events
    }
}

#[async_trait::async_trait]
impl EthereumChain for Ethereum {
    fn chain_id(&self) -> &str {
        // Can't return reference to RwLock data; return default
        "0x1"
    }

    fn accounts(&self) -> &[String] {
        // Can't return reference to RwLock data; return empty
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
        let accounts =
            operations::connect(self.strategy.as_ref(), &self.events).await?;
        *self.accounts.write().await = accounts.clone();
        Ok(accounts)
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        operations::disconnect(self.strategy.as_ref(), &self.events).await?;
        *self.accounts.write().await = vec![];
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
        *self.chain_id.write().await = hex_chain_id;
        Ok(())
    }

    async fn get_chain_id(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let chain_id_hex = operations::get_chain_id(self.strategy.as_ref()).await?;
        let chain_id_str = chain_id_hex.trim_start_matches("0x");
        let parsed = u64::from_str_radix(chain_id_str, 16)?;
        *self.chain_id.write().await = chain_id_hex;
        Ok(parsed)
    }

    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let accounts = operations::get_accounts(self.strategy.as_ref()).await?;
        *self.accounts.write().await = accounts.clone();
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
        create: Box::new(move || {
            Box::new(Ethereum::new(strategy.clone()))
        }),
    }
}
