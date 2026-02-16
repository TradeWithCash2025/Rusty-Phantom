//! Embedded provider for the browser SDK.
//!
//! Wraps the core `EmbeddedProvider` with browser-specific platform adapters
//! (storage, auth, stamper, URL params). In the TypeScript SDK, this uses
//! IndexedDB stamper, localStorage, and window.location for browser integration.
//!
//! This module also provides the browser-specific platform adapter implementations:
//! - [`BrowserAuthProvider`] — OAuth redirect-based auth (implements [`AuthProvider`])
//! - [`BrowserLogger`] — Console/tracing-based debug logging (implements [`DebugLogger`])
//! - [`BrowserStorage`] — File-backed session storage (implements [`EmbeddedStorage`])
//! - [`BrowserURLParamsAccessor`] — URL parameter access (implements [`UrlParamsAccessor`])
//! - [`BrowserPhantomAppProvider`] — Phantom app integration (implements [`PhantomAppProvider`])
//! - [`BrowserPlatformAdapter`] — Aggregates all adapters (implements [`PlatformAdapter`])

mod auth;
mod logger;
mod phantom_app;
mod platform;
mod storage;
mod url_params;

use phantom_chain_interfaces::{EthereumChain, SolanaChain};
use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::{
    EmbeddedEthereumChain, EmbeddedProvider as CoreEmbeddedProvider, EmbeddedProviderConfig,
    EmbeddedSolanaChain, PlatformAdapter, WalletAddress,
};
use std::sync::Arc;

use crate::types::{AuthOptions, ConnectResult, ConnectStatus, Provider};

// Re-export browser adapter types.
pub use auth::{BrowserAuthConfig, BrowserAuthProvider};
pub use logger::BrowserLogger;
pub use phantom_app::BrowserPhantomAppProvider;
pub use platform::{BrowserPlatformAdapter, BrowserPlatformConfig};
pub use storage::BrowserStorage;
pub use url_params::BrowserURLParamsAccessor;

/// Browser-specific embedded provider.
///
/// Extends the core `EmbeddedProvider` with a browser platform adapter
/// and address type tracking. The core provider is held in an `Arc` so
/// that chain wrappers (`EmbeddedSolanaChain`, `EmbeddedEthereumChain`)
/// can share it.
pub struct BrowserEmbeddedProvider {
    core: Arc<CoreEmbeddedProvider>,
    address_types: Vec<AddressFormat>,
    solana_chain: Option<Arc<EmbeddedSolanaChain>>,
    ethereum_chain: Option<Arc<EmbeddedEthereumChain>>,
}

impl BrowserEmbeddedProvider {
    /// Create a new browser embedded provider.
    pub fn new(
        config: EmbeddedProviderConfig,
        platform: Arc<dyn PlatformAdapter>,
        logger: Arc<dyn phantom_embedded_provider_core::DebugLogger>,
    ) -> Result<Self, String> {
        let address_types = config.address_types.clone();
        let core = Arc::new(CoreEmbeddedProvider::new(config, platform, logger)?);

        // Create chain wrappers based on configured address types.
        let solana_chain = if address_types.contains(&AddressFormat::Solana) {
            Some(Arc::new(EmbeddedSolanaChain::new(core.clone())))
        } else {
            None
        };
        let ethereum_chain = if address_types.contains(&AddressFormat::Ethereum) {
            Some(Arc::new(EmbeddedEthereumChain::new(core.clone())))
        } else {
            None
        };

        Ok(Self {
            core,
            address_types,
            solana_chain,
            ethereum_chain,
        })
    }
}

#[async_trait::async_trait]
impl Provider for BrowserEmbeddedProvider {
    async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        let core_auth = phantom_embedded_provider_core::AuthOptions {
            provider: match auth_options.provider {
                crate::types::AuthProviderType::Google => {
                    phantom_embedded_provider_core::EmbeddedProviderAuthType::Google
                }
                crate::types::AuthProviderType::Apple => {
                    phantom_embedded_provider_core::EmbeddedProviderAuthType::Apple
                }
                crate::types::AuthProviderType::Phantom => {
                    phantom_embedded_provider_core::EmbeddedProviderAuthType::Phantom
                }
                crate::types::AuthProviderType::Device => {
                    phantom_embedded_provider_core::EmbeddedProviderAuthType::Device
                }
                _ => {
                    return Err("Invalid auth provider for embedded wallet".into());
                }
            },
            custom_auth_data: auth_options.custom_auth_data.clone(),
        };

        let result = self.core.connect(core_auth).await?;

        Ok(ConnectResult {
            addresses: result.addresses,
            wallet_id: result.wallet_id,
            auth_user_id: result.auth_user_id,
            auth_provider: Some(auth_options.provider.clone()),
            status: result.status.map(|s| match s {
                phantom_embedded_provider_core::ConnectStatus::Pending => ConnectStatus::Pending,
                phantom_embedded_provider_core::ConnectStatus::Completed => {
                    ConnectStatus::Completed
                }
            }),
            wallet: None,
        })
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.core.disconnect(true).await
    }

    fn get_addresses(&self) -> Vec<WalletAddress> {
        // get_addresses is async on core, but we need sync here.
        // Return empty and let callers use async version when needed.
        vec![]
    }

    fn is_connected(&self) -> bool {
        // is_connected is async on core, same approach.
        false
    }

    async fn auto_connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.core.auto_connect().await
    }

    fn get_enabled_address_types(&self) -> Vec<AddressFormat> {
        self.address_types.clone()
    }

    async fn solana(
        &self,
    ) -> Result<Arc<dyn SolanaChain>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.solana_chain {
            Some(chain) => Ok(chain.clone() as Arc<dyn SolanaChain>),
            None => Err("Solana not enabled for this embedded provider".into()),
        }
    }

    async fn ethereum(
        &self,
    ) -> Result<Arc<dyn EthereumChain>, Box<dyn std::error::Error + Send + Sync>> {
        match &self.ethereum_chain {
            Some(chain) => Ok(chain.clone() as Arc<dyn EthereumChain>),
            None => Err("Ethereum not enabled for this embedded provider".into()),
        }
    }
}
