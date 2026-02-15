//! Wallet discovery and registry for the browser SDK.
//!
//! Manages discovered injected wallets (Phantom, external wallets via EIP-6963
//! and Wallet Standard), custom wallets, and wallet selection.

use phantom_chain_interfaces::{EthereumChain, SolanaChain};
use phantom_client::constants::AddressFormat;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// ID for an injected wallet.
pub type InjectedWalletId = String;

/// Chain providers available on a wallet.
pub struct WalletProviders {
    pub ethereum: Option<Arc<dyn EthereumChain>>,
    pub solana: Option<Arc<dyn SolanaChain>>,
}

/// Information about a discovered injected wallet.
pub struct InjectedWalletInfo {
    pub id: InjectedWalletId,
    pub name: String,
    pub icon: Option<String>,
    pub address_types: Vec<AddressFormat>,
    pub providers: Option<WalletProviders>,
    pub rdns: Option<String>,
    pub discovery: Option<String>,
}

/// Registry of discovered injected wallets.
pub struct InjectedWalletRegistry {
    wallets: Mutex<HashMap<InjectedWalletId, InjectedWalletInfo>>,
}

impl InjectedWalletRegistry {
    /// Create a new empty wallet registry.
    pub fn new() -> Self {
        Self {
            wallets: Mutex::new(HashMap::new()),
        }
    }

    /// Register a wallet.
    pub fn register(&self, info: InjectedWalletInfo) {
        let id = info.id.clone();
        self.wallets.lock().unwrap().insert(id, info);
    }

    /// Unregister a wallet.
    pub fn unregister(&self, id: &str) {
        self.wallets.lock().unwrap().remove(id);
    }

    /// Check if a wallet is registered.
    pub fn has(&self, id: &str) -> bool {
        self.wallets.lock().unwrap().contains_key(id)
    }

    /// Get all registered wallet IDs.
    pub fn get_all_ids(&self) -> Vec<InjectedWalletId> {
        self.wallets.lock().unwrap().keys().cloned().collect()
    }

    /// Get the number of registered wallets.
    pub fn len(&self) -> usize {
        self.wallets.lock().unwrap().len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.wallets.lock().unwrap().is_empty()
    }
}

impl Default for InjectedWalletRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static WALLET_REGISTRY: OnceLock<Arc<InjectedWalletRegistry>> = OnceLock::new();

/// Get the global wallet registry instance.
pub fn get_wallet_registry() -> Arc<InjectedWalletRegistry> {
    WALLET_REGISTRY
        .get_or_init(|| Arc::new(InjectedWalletRegistry::new()))
        .clone()
}

/// Configuration for a custom wallet.
#[derive(Debug, Clone)]
pub struct CustomWalletConfig {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub window_property: String,
    pub address_types: Vec<AddressFormat>,
}
