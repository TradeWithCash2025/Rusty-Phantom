//! Embedded provider for the browser SDK.
//!
//! Wraps the core `EmbeddedProvider` with browser-specific platform adapters
//! (storage, auth, stamper, URL params). In the TypeScript SDK, this uses
//! IndexedDB stamper, localStorage, and window.location for browser integration.

use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::{
    EmbeddedProvider as CoreEmbeddedProvider, EmbeddedProviderConfig, PlatformAdapter, WalletAddress,
};
use std::sync::Arc;

use crate::types::{AuthOptions, ConnectResult, ConnectStatus, Provider};

/// Browser-specific embedded provider.
///
/// Extends the core `EmbeddedProvider` with a browser platform adapter
/// and address type tracking.
pub struct BrowserEmbeddedProvider {
    core: CoreEmbeddedProvider,
    address_types: Vec<AddressFormat>,
}

impl BrowserEmbeddedProvider {
    /// Create a new browser embedded provider.
    pub fn new(
        config: EmbeddedProviderConfig,
        platform: Arc<dyn PlatformAdapter>,
        logger: Arc<dyn phantom_embedded_provider_core::DebugLogger>,
    ) -> Result<Self, String> {
        let address_types = config.address_types.clone();
        let core = CoreEmbeddedProvider::new(config, platform, logger)?;
        Ok(Self {
            core,
            address_types,
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
}
