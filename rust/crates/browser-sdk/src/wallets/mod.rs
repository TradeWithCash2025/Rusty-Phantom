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

/// How a wallet was discovered.
///
/// Corresponds to the TypeScript `"phantom" | "eip6963" | "standard" | "custom"` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoverySource {
    /// Phantom's own wallet provider.
    Phantom,
    /// Discovered via EIP-6963 (injected provider announcement).
    Eip6963,
    /// Discovered via the Wallet Standard protocol.
    Standard,
    /// A custom wallet detected by window property probing.
    Custom,
}

impl DiscoverySource {
    /// Return the string representation matching the TypeScript values.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiscoverySource::Phantom => "phantom",
            DiscoverySource::Eip6963 => "eip6963",
            DiscoverySource::Standard => "standard",
            DiscoverySource::Custom => "custom",
        }
    }

    /// Parse from a string, returning `None` for unrecognised values.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "phantom" => Some(DiscoverySource::Phantom),
            "eip6963" => Some(DiscoverySource::Eip6963),
            "standard" => Some(DiscoverySource::Standard),
            "custom" => Some(DiscoverySource::Custom),
            _ => None,
        }
    }
}

impl std::fmt::Display for DiscoverySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

impl Clone for WalletProviders {
    fn clone(&self) -> Self {
        Self {
            ethereum: self.ethereum.clone(),
            solana: self.solana.clone(),
        }
    }
}

impl Clone for InjectedWalletInfo {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            icon: self.icon.clone(),
            address_types: self.address_types.clone(),
            providers: self.providers.clone(),
            rdns: self.rdns.clone(),
            discovery: self.discovery.clone(),
        }
    }
}

impl InjectedWalletInfo {
    /// Get the wallet's chain providers.
    ///
    /// Returns `None` if no providers have been set on this wallet.
    pub fn providers(&self) -> Option<&WalletProviders> {
        self.providers.as_ref()
    }

    /// Parse the `discovery` field into a strongly-typed [`DiscoverySource`].
    ///
    /// Returns `None` when `discovery` is `None` or contains an unrecognised value.
    pub fn discovery_source(&self) -> Option<DiscoverySource> {
        self.discovery
            .as_deref()
            .and_then(DiscoverySource::from_str)
    }
}

/// Check whether the given wallet info represents the Phantom wallet.
///
/// Equivalent to the TypeScript `isPhantomWallet` type guard.
pub fn is_phantom_wallet(wallet: &InjectedWalletInfo) -> bool {
    wallet.id == "phantom"
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

    /// Look up a wallet by its ID.
    ///
    /// Returns a cloned [`InjectedWalletInfo`] if a wallet with the given ID is
    /// registered, or `None` otherwise.
    pub fn get_by_id(&self, id: &str) -> Option<InjectedWalletInfo> {
        self.wallets.lock().unwrap().get(id).cloned()
    }

    /// Return all registered wallets.
    ///
    /// The returned `Vec` is a snapshot; subsequent mutations of the registry
    /// will not be reflected.
    pub fn get_all(&self) -> Vec<InjectedWalletInfo> {
        self.wallets.lock().unwrap().values().cloned().collect()
    }

    /// Return wallets that support at least one of the given address types.
    ///
    /// If `address_types` is empty, all wallets are returned (matching the TS
    /// behaviour).
    pub fn get_by_address_types(&self, address_types: &[AddressFormat]) -> Vec<InjectedWalletInfo> {
        if address_types.is_empty() {
            return self.get_all();
        }
        self.wallets
            .lock()
            .unwrap()
            .values()
            .filter(|w| w.address_types.iter().any(|t| address_types.contains(t)))
            .cloned()
            .collect()
    }

    /// Discover wallets from the supplied custom wallet configurations.
    ///
    /// In the Rust SDK there are no browser APIs to call, so discovery simply
    /// registers a wallet entry for each provided [`CustomWalletConfig`].
    /// Wallets whose ID is already registered are skipped so that previously
    /// discovered wallets are not overwritten.
    pub fn discover(&self, configs: &[CustomWalletConfig]) {
        let mut map = self.wallets.lock().unwrap();
        for cfg in configs {
            if map.contains_key(&cfg.id) {
                continue;
            }
            let info = InjectedWalletInfo {
                id: cfg.id.clone(),
                name: cfg.name.clone(),
                icon: Some(cfg.icon.clone()),
                address_types: cfg.address_types.clone(),
                providers: None,
                rdns: None,
                discovery: Some(DiscoverySource::Custom.to_string()),
            };
            map.insert(cfg.id.clone(), info);
        }
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

/// Pre-defined custom wallet configurations.
///
/// These correspond to wallets that require special detection logic beyond
/// EIP-6963 and Wallet Standard. Equivalent to the TypeScript
/// `CUSTOM_WALLET_CONFIGS` array.
pub fn custom_wallet_configs() -> Vec<CustomWalletConfig> {
    vec![CustomWalletConfig {
        id: "coinbase-wallet".to_string(),
        name: "Coinbase Wallet".to_string(),
        icon: "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNTYiIGhlaWdodD0iNTYiIGZpbGw9Im5vbmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PHBhdGggZD0iTTI4IDU2YzE1LjQ2NCAwIDI4LTEyLjUzNiAyOC0yOFM0My40NjQgMCAyOCAwIDAgMTIuNTM2IDAgMjhzMTIuNTM2IDI4IDI4IDI4WiIgZmlsbD0iIzFCNTNFNCIvPjxwYXRoIGZpbGwtcnVsZT0iZXZlbm9kZCIgY2xpcC1ydWxlPSJldmVub2RkIiBkPSJNNyAyOGMwIDExLjU5OCA5LjQwMiAyMSAyMSAyMXMyMS05LjQwMiAyMS0yMVMzOS41OTggNyAyOCA3IDcgMTYuNDAyIDcgMjhabTE3LjIzNC02Ljc2NmEzIDMgMCAwIDAtMyAzdjcuNTMzYTMgMyAwIDAgMCAzIDNoNy41MzNhMyAzIDAgMCAwIDMtM3YtNy41MzNhMyAzIDAgMCAwLTMtM2gtNy41MzNaIiBmaWxsPSIjZmZmIi8+PC9zdmc+".to_string(),
        window_property: "coinbaseSolana".to_string(),
        address_types: vec![AddressFormat::Solana],
    }]
}
