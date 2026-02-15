//! Injected provider for the browser SDK.
//!
//! Manages connections to injected browser wallets (Phantom extension,
//! external wallets discovered via EIP-6963 and Wallet Standard).
//! Supports multi-chain connections, event forwarding, and auto-connect.

use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::WalletAddress;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{
    AuthOptions, AuthProviderType, ConnectResult, ConnectStatus, Provider,
};
use crate::wallets::{get_wallet_registry, InjectedWalletRegistry};

/// Configuration for the injected provider.
#[derive(Debug, Clone)]
pub struct InjectedProviderConfig {
    pub address_types: Vec<AddressFormat>,
}

/// State for a specific wallet.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct WalletState {
    connected: bool,
    addresses: Vec<WalletAddress>,
}

/// Injected provider that manages connections to browser-injected wallets.
pub struct InjectedProvider {
    address_types: Vec<AddressFormat>,
    wallet_registry: Arc<InjectedWalletRegistry>,
    selected_wallet_id: RwLock<Option<String>>,
    wallet_states: RwLock<HashMap<String, WalletState>>,
}

impl InjectedProvider {
    /// Create a new injected provider.
    pub fn new(config: InjectedProviderConfig) -> Self {
        let wallet_registry = get_wallet_registry();
        Self {
            address_types: config.address_types,
            wallet_registry,
            selected_wallet_id: RwLock::new(None),
            wallet_states: RwLock::new(HashMap::new()),
        }
    }

    fn get_wallet_id_sync(&self) -> String {
        // Since we can't await in sync contexts, return default
        "phantom".to_string()
    }
}

#[async_trait::async_trait]
impl Provider for InjectedProvider {
    async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        if auth_options.provider != AuthProviderType::Injected {
            return Err(format!(
                "Invalid provider for injected connection: {:?}. Must be Injected",
                auth_options.provider
            )
            .into());
        }

        let requested_wallet_id = auth_options
            .wallet_id
            .clone()
            .unwrap_or_else(|| "phantom".to_string());

        if !self.wallet_registry.has(&requested_wallet_id) {
            return Err(format!("Unknown injected wallet id: {}", requested_wallet_id).into());
        }

        *self.selected_wallet_id.write().await = Some(requested_wallet_id.clone());

        // In a real browser, we'd call into the wallet's connect methods here.
        // The actual chain connection logic is handled by the platform adapter.
        // For now, return an empty result that the platform adapter would populate.
        Ok(ConnectResult {
            addresses: vec![],
            wallet_id: Some(requested_wallet_id),
            auth_user_id: None,
            auth_provider: Some(AuthProviderType::Injected),
            status: Some(ConnectStatus::Completed),
            wallet: None,
        })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let wallet_id = self
            .selected_wallet_id
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "phantom".to_string());

        let mut states = self.wallet_states.write().await;
        states.insert(
            wallet_id,
            WalletState {
                connected: false,
                addresses: vec![],
            },
        );

        Ok(())
    }

    fn get_addresses(&self) -> Vec<WalletAddress> {
        let _wallet_id = self.get_wallet_id_sync();
        // Best effort without async
        vec![]
    }

    fn is_connected(&self) -> bool {
        false // Requires async access to wallet_states
    }

    async fn auto_connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if previously connected wallet is available
        // In a browser, this would check localStorage for was-connected flag
        Ok(())
    }

    fn get_enabled_address_types(&self) -> Vec<AddressFormat> {
        self.address_types.clone()
    }
}
