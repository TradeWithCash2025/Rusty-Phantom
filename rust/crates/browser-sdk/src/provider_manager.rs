//! Provider manager that orchestrates between embedded and injected providers.
//!
//! Handles provider switching, auto-connect with fallback strategy,
//! and event forwarding from underlying providers to the SDK.

use phantom_embedded_provider_core::WalletAddress;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::{InjectedProvider, InjectedProviderConfig};
use crate::types::{
    AuthOptions, AuthProviderType, BrowserSdkConfig, ConnectResult, Provider,
};

/// Provider preference information.
#[derive(Debug, Clone)]
pub struct ProviderPreference {
    /// Provider type.
    pub provider_type: String,
    /// Embedded wallet type (if applicable).
    pub embedded_wallet_type: Option<String>,
}

/// Manages multiple providers and routes operations to the active one.
pub struct ProviderManager {
    config: BrowserSdkConfig,
    providers: RwLock<Vec<(String, Arc<dyn Provider>)>>,
    current_provider_key: RwLock<Option<String>>,
}

impl ProviderManager {
    /// Create a new ProviderManager.
    pub fn new(config: BrowserSdkConfig) -> Self {
        let manager = Self {
            config,
            providers: RwLock::new(Vec::new()),
            current_provider_key: RwLock::new(None),
        };

        manager
    }

    /// Initialize providers based on config (must be called after construction).
    pub async fn initialize(&self) {
        let has_injected = self
            .config
            .providers
            .contains(&AuthProviderType::Injected);
        let has_embedded = self
            .config
            .providers
            .iter()
            .any(|p| !matches!(p, AuthProviderType::Injected | AuthProviderType::Deeplink));

        if has_injected {
            let injected = InjectedProvider::new(InjectedProviderConfig {
                address_types: self.config.address_types.clone(),
            });
            let key = "injected".to_string();
            self.providers
                .write()
                .await
                .push((key.clone(), Arc::new(injected)));
        }

        // Set default provider key
        let mut key_guard = self.current_provider_key.write().await;
        if has_embedded {
            let wallet_type = self
                .config
                .embedded_wallet_type
                .clone()
                .unwrap_or_else(|| "user-wallet".to_string());
            *key_guard = Some(format!("embedded-{}", wallet_type));
        } else if has_injected {
            *key_guard = Some("injected".to_string());
        }
    }

    /// Get the current active provider.
    pub async fn get_current_provider(&self) -> Option<Arc<dyn Provider>> {
        let key = self.current_provider_key.read().await;
        let key = key.as_deref()?;
        let providers = self.providers.read().await;
        providers
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, p)| p.clone())
    }

    /// Get current provider info.
    pub async fn get_current_provider_info(&self) -> Option<ProviderPreference> {
        let key = self.current_provider_key.read().await;
        let key = key.as_deref()?;

        if key == "injected" {
            Some(ProviderPreference {
                provider_type: "injected".to_string(),
                embedded_wallet_type: None,
            })
        } else if key.starts_with("embedded-") {
            let wallet_type = key.strip_prefix("embedded-").map(|s| s.to_string());
            Some(ProviderPreference {
                provider_type: "embedded".to_string(),
                embedded_wallet_type: wallet_type,
            })
        } else {
            None
        }
    }

    /// Connect using the current provider.
    pub async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        // Validate provider is allowed
        if !self.config.providers.contains(&auth_options.provider) {
            return Err(format!(
                "Provider {:?} is not in the allowed providers list",
                auth_options.provider
            )
            .into());
        }

        // Auto-switch provider based on auth type
        match auth_options.provider {
            AuthProviderType::Injected => {
                *self.current_provider_key.write().await = Some("injected".to_string());
            }
            AuthProviderType::Deeplink => {
                // Deeplink returns empty result (redirect happens externally)
                return Ok(ConnectResult {
                    addresses: vec![],
                    wallet_id: None,
                    auth_user_id: None,
                    auth_provider: Some(AuthProviderType::Deeplink),
                    status: None,
                    wallet: None,
                });
            }
            _ => {
                let wallet_type = self
                    .config
                    .embedded_wallet_type
                    .clone()
                    .unwrap_or_else(|| "user-wallet".to_string());
                *self.current_provider_key.write().await =
                    Some(format!("embedded-{}", wallet_type));
            }
        }

        let provider = self
            .get_current_provider()
            .await
            .ok_or("No provider selected")?;

        provider.connect(auth_options).await
    }

    /// Disconnect from the current provider.
    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(provider) = self.get_current_provider().await {
            provider.disconnect().await?;
        }
        Ok(())
    }

    /// Get addresses from the current provider.
    pub async fn get_addresses(&self) -> Vec<WalletAddress> {
        if let Some(provider) = self.get_current_provider().await {
            provider.get_addresses()
        } else {
            vec![]
        }
    }

    /// Check if the current provider is connected.
    pub async fn is_connected(&self) -> bool {
        if let Some(provider) = self.get_current_provider().await {
            provider.is_connected()
        } else {
            false
        }
    }

    /// Attempt auto-connect with fallback strategy.
    pub async fn auto_connect(&self) -> bool {
        // Try embedded first, then injected
        let providers = self.providers.read().await;

        for (key, provider) in providers.iter() {
            if let Ok(()) = provider.auto_connect().await {
                if provider.is_connected() {
                    *self.current_provider_key.write().await = Some(key.clone());
                    return true;
                }
            }
        }

        false
    }
}
