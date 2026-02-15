//! Browser SDK for Phantom wallet integration.
//!
//! Provides a unified interface for interacting with Phantom wallets
//! in browser environments. Supports both embedded (OAuth) and injected
//! (extension) providers, wallet discovery (EIP-6963, Wallet Standard),
//! debug logging, auto-confirm, and deeplink generation.

pub mod browser_sdk;
pub mod debug;
pub mod provider_manager;
pub mod providers;
pub mod types;
pub mod utils;
pub mod wallets;

// Re-export main types
pub use browser_sdk::BrowserSdk;
pub use debug::{debug, DebugCategory, DebugLevel};
pub use provider_manager::ProviderPreference;
pub use types::{
    AuthOptions, AuthProviderType, BrowserSdkConfig, ConnectResult, ConnectStatus, Provider,
};
pub use phantom_embedded_provider_core::WalletAddress;
pub use utils::{
    get_browser_display_name, get_deeplink_to_phantom, get_platform_name,
    is_mobile_user_agent, parse_browser_from_user_agent, BrowserInfo,
};
pub use wallets::{get_wallet_registry, InjectedWalletId, InjectedWalletInfo, InjectedWalletRegistry};

// Re-export from dependencies
pub use phantom_chain_interfaces::{EthereumChain, SolanaChain};
pub use phantom_constants::NetworkId;
pub use phantom_client::constants::AddressFormat;
