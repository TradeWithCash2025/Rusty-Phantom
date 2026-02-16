//! Ethereum provider strategy traits and implementations.

use super::siwe::create_siwe_message;
use super::types::*;
use crate::types::ProviderStrategy;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Provider trait (represents window.phantom.ethereum in TS)
// ============================================================================

/// Trait representing a Phantom Ethereum provider.
///
/// In the TypeScript SDK, this is the `PhantomEthereumProvider` interface
/// which maps to `window.phantom.ethereum`. In Rust, platform adapters
/// implement this trait.
#[async_trait::async_trait]
pub trait PhantomEthereumProvider: Send + Sync {
    /// Whether the provider is Phantom.
    fn is_phantom(&self) -> bool;

    /// The currently selected address, if any.
    fn selected_address(&self) -> Option<String>;

    /// The current chain ID (hex-encoded).
    fn chain_id(&self) -> String;

    /// Whether the provider is connected.
    fn is_connected(&self) -> bool;

    /// Send a JSON-RPC request.
    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// Strategy trait
// ============================================================================

/// Trait representing an Ethereum strategy.
#[async_trait::async_trait]
pub trait EthereumStrategy: Send + Sync {
    /// The strategy type.
    fn strategy_type(&self) -> ProviderStrategy;

    /// Whether the provider is currently connected.
    fn is_connected(&self) -> bool;

    /// Get the underlying provider.
    fn get_provider(&self) -> Option<Arc<dyn PhantomEthereumProvider>>;

    /// Connect to the wallet.
    async fn connect(
        &self,
        only_if_trusted: bool,
    ) -> Result<Option<Vec<String>>, Box<dyn std::error::Error + Send + Sync>>;

    /// Disconnect from the wallet.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get connected accounts.
    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a message (eth_sign).
    async fn sign_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a personal message (personal_sign).
    async fn sign_personal_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign EIP-712 typed data.
    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign in with SIWE.
    async fn sign_in(
        &self,
        sign_in_data: &EthereumSignInData,
    ) -> Result<EthereumSignInResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Send a transaction.
    async fn send_transaction(
        &self,
        transaction: &EthereumTransaction,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Sign a transaction without sending.
    async fn sign_transaction(
        &self,
        transaction: &EthereumTransaction,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the current chain ID.
    async fn get_chain_id(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Switch to a different chain.
    async fn switch_chain(
        &self,
        chain_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Send a raw JSON-RPC request.
    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// InjectedEthereumStrategy
// ============================================================================

const MAX_RETRIES: u32 = 4;
const BASE_DELAY_MS: u64 = 100;

/// Factory trait for obtaining an injected Ethereum provider.
#[async_trait::async_trait]
pub trait EthereumProviderFactory: Send + Sync {
    /// Attempt to get the injected Phantom Ethereum provider.
    fn get_provider(&self) -> Option<Arc<dyn PhantomEthereumProvider>>;
}

/// Injected Ethereum strategy — wraps a provider obtained from the platform.
pub struct InjectedEthereumStrategy {
    factory: Arc<dyn EthereumProviderFactory>,
    provider: Option<Arc<dyn PhantomEthereumProvider>>,
}

impl InjectedEthereumStrategy {
    /// Create a new injected strategy with the given factory.
    pub fn new(factory: Arc<dyn EthereumProviderFactory>) -> Self {
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
    ) -> Result<&Arc<dyn PhantomEthereumProvider>, Box<dyn std::error::Error + Send + Sync>> {
        self.provider
            .as_ref()
            .ok_or_else(|| "Provider not found.".into())
    }
}

#[async_trait::async_trait]
impl EthereumStrategy for InjectedEthereumStrategy {
    fn strategy_type(&self) -> ProviderStrategy {
        ProviderStrategy::Injected
    }

    fn is_connected(&self) -> bool {
        self.provider
            .as_ref()
            .map(|p| p.is_connected() && p.selected_address().is_some())
            .unwrap_or(false)
    }

    fn get_provider(&self) -> Option<Arc<dyn PhantomEthereumProvider>> {
        self.provider.clone()
    }

    async fn connect(
        &self,
        only_if_trusted: bool,
    ) -> Result<Option<Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;

        if provider.is_connected() && provider.selected_address().is_some() {
            return Ok(Some(self.get_accounts().await?));
        }

        let method = if only_if_trusted {
            "eth_accounts"
        } else {
            "eth_requestAccounts"
        };

        match provider.request(method, None).await {
            Ok(val) => {
                let accounts: Vec<String> = serde_json::from_value(val).unwrap_or_default();
                Ok(Some(accounts))
            }
            Err(_) => Ok(None),
        }
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ethereum providers don't typically have a disconnect method.
        Ok(())
    }

    async fn get_accounts(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = match self.provider.as_ref() {
            Some(p) => p,
            None => return Ok(vec![]),
        };

        match provider.request("eth_accounts", None).await {
            Ok(val) => Ok(serde_json::from_value(val).unwrap_or_default()),
            Err(_) => Ok(vec![]),
        }
    }

    async fn sign_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }

        let params = vec![
            serde_json::Value::String(address.to_string()),
            serde_json::Value::String(message.to_string()),
        ];
        let result = provider.request("eth_sign", Some(&params)).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn sign_personal_message(
        &self,
        message: &str,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }

        let params = vec![
            serde_json::Value::String(message.to_string()),
            serde_json::Value::String(address.to_string()),
        ];
        let result = provider.request("personal_sign", Some(&params)).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn sign_typed_data(
        &self,
        typed_data: &serde_json::Value,
        address: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }

        let params = vec![
            serde_json::Value::String(address.to_string()),
            serde_json::Value::String(serde_json::to_string(typed_data)?),
        ];
        let result = provider
            .request("eth_signTypedData_v4", Some(&params))
            .await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn sign_in(
        &self,
        sign_in_data: &EthereumSignInData,
    ) -> Result<EthereumSignInResult, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;

        let message = create_siwe_message(sign_in_data)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

        let address = provider.selected_address().ok_or("No address available.")?;

        let signature = self.sign_personal_message(&message, &address).await?;

        Ok(EthereumSignInResult {
            address,
            signature,
            signed_message: message,
        })
    }

    async fn send_transaction(
        &self,
        transaction: &EthereumTransaction,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }

        let params = vec![serde_json::to_value(transaction)?];
        let result = provider
            .request("eth_sendTransaction", Some(&params))
            .await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn sign_transaction(
        &self,
        transaction: &EthereumTransaction,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        if !provider.is_connected() {
            return Err("Provider is not connected.".into());
        }

        let params = vec![serde_json::to_value(transaction)?];
        let result = provider
            .request("eth_signTransaction", Some(&params))
            .await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn get_chain_id(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        let result = provider.request("eth_chainId", None).await?;
        Ok(result.as_str().unwrap_or_default().to_string())
    }

    async fn switch_chain(
        &self,
        chain_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        let params = vec![serde_json::json!({ "chainId": chain_id })];
        provider
            .request("wallet_switchEthereumChain", Some(&params))
            .await?;
        Ok(())
    }

    async fn request(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let provider = self.require_provider()?;
        provider.request(method, params).await
    }
}
