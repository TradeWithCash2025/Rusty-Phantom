//! Provider manager that orchestrates between embedded and injected providers.
//!
//! Handles provider switching, auto-connect with fallback strategy,
//! event forwarding from underlying providers to the SDK, and
//! provider preference persistence.

use phantom_embedded_provider_core::WalletAddress;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::debug::{debug, DebugCategory};
use crate::providers::{InjectedProvider, InjectedProviderConfig};
use crate::types::{
    AuthOptions, AuthProviderType, BrowserSdkConfig, ConnectResult, Provider,
};

/// Embedded provider auth types (subset that triggers embedded provider).
const EMBEDDED_PROVIDER_AUTH_TYPES: &[AuthProviderType] = &[
    AuthProviderType::Google,
    AuthProviderType::Apple,
    AuthProviderType::Phantom,
    AuthProviderType::Device,
];

/// Provider preference information.
#[derive(Debug, Clone)]
pub struct ProviderPreference {
    /// Provider type: "injected" or "embedded".
    pub provider_type: String,
    /// Embedded wallet type (if applicable): "app-wallet" or "user-wallet".
    pub embedded_wallet_type: Option<String>,
}

/// Options for switching providers.
#[derive(Debug, Clone, Default)]
pub struct SwitchProviderOptions {
    /// Embedded wallet type to use.
    pub embedded_wallet_type: Option<String>,
}

/// Manages multiple providers and routes operations to the active one.
///
/// Supports embedded and injected providers with event forwarding,
/// auto-connect fallback strategy, and provider preference persistence.
pub struct ProviderManager {
    config: BrowserSdkConfig,
    providers: RwLock<HashMap<String, Arc<dyn Provider>>>,
    current_provider_key: RwLock<Option<String>>,
    /// Concrete reference to the injected provider for chain-level access.
    injected_provider: RwLock<Option<Arc<InjectedProvider>>>,
    event_listeners: RwLock<HashMap<String, Vec<(u64, Arc<dyn Fn(Option<serde_json::Value>) + Send + Sync>)>>>,
    next_listener_id: RwLock<u64>,
}

impl ProviderManager {
    /// Create a new ProviderManager.
    pub fn new(config: BrowserSdkConfig) -> Self {
        debug().log(
            DebugCategory::PROVIDER_MANAGER,
            "Initializing ProviderManager",
            None,
        );

        Self {
            config,
            providers: RwLock::new(HashMap::new()),
            current_provider_key: RwLock::new(None),
            injected_provider: RwLock::new(None),
            event_listeners: RwLock::new(HashMap::new()),
            next_listener_id: RwLock::new(1),
        }
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

        let default_embedded_type = self
            .config
            .embedded_wallet_type
            .clone()
            .unwrap_or_else(|| "user-wallet".to_string());

        // Create injected provider if allowed
        if has_injected {
            debug().log(
                DebugCategory::PROVIDER_MANAGER,
                "Creating injected provider",
                None,
            );
            let injected = Arc::new(InjectedProvider::new(InjectedProviderConfig {
                address_types: self.config.address_types.clone(),
            }));
            // Store the concrete Arc for chain-level access.
            *self.injected_provider.write().await = Some(injected.clone());
            self.providers
                .write()
                .await
                .insert("injected".to_string(), injected);
        }

        // Set default provider key: prefer embedded if available, otherwise injected
        let mut key_guard = self.current_provider_key.write().await;
        if has_embedded {
            *key_guard = Some(format!("embedded-{}", default_embedded_type));
        } else if has_injected {
            *key_guard = Some("injected".to_string());
        }

        debug().info(
            DebugCategory::PROVIDER_MANAGER,
            "ProviderManager initialized",
            None,
        );
    }

    /// Switch to a different provider type.
    ///
    /// Creates the provider if it doesn't exist yet. Returns the new active provider.
    pub async fn switch_provider(
        &self,
        provider_type: &str,
        options: Option<SwitchProviderOptions>,
    ) -> Result<Arc<dyn Provider>, Box<dyn std::error::Error + Send + Sync>> {
        let embedded_wallet_type = options
            .as_ref()
            .and_then(|o| o.embedded_wallet_type.clone());

        // Validate embedded wallet type if provided
        if let Some(ref ewt) = embedded_wallet_type {
            if !["app-wallet", "user-wallet"].contains(&ewt.as_str()) {
                return Err(format!(
                    "Invalid embeddedWalletType: {}. Must be \"app-wallet\" or \"user-wallet\".",
                    ewt
                )
                .into());
            }
        }

        let key = get_provider_key(provider_type, embedded_wallet_type.as_deref());

        // Create injected provider on-demand if needed
        if provider_type == "injected" && !self.providers.read().await.contains_key(&key) {
            let injected = Arc::new(InjectedProvider::new(InjectedProviderConfig {
                address_types: self.config.address_types.clone(),
            }));
            *self.injected_provider.write().await = Some(injected.clone());
            self.providers
                .write()
                .await
                .insert(key.clone(), injected);
        }

        let providers = self.providers.read().await;
        let provider = providers
            .get(&key)
            .ok_or_else(|| format!("Provider not found: {}", key))?
            .clone();

        *self.current_provider_key.write().await = Some(key);

        Ok(provider)
    }

    /// Get the current active provider.
    pub async fn get_current_provider(&self) -> Option<Arc<dyn Provider>> {
        let key = self.current_provider_key.read().await;
        let key = key.as_deref()?;
        let providers = self.providers.read().await;
        providers.get(key).cloned()
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

    /// Check if a provider is allowed by the config.
    pub fn is_provider_allowed(&self, provider: &AuthProviderType) -> bool {
        self.config.providers.contains(provider)
    }

    /// Register an external provider (e.g., embedded provider created by platform adapter).
    pub async fn register_provider(
        &self,
        key: &str,
        provider: Arc<dyn Provider>,
    ) {
        self.providers.write().await.insert(key.to_string(), provider);
    }

    /// Connect using the current provider.
    ///
    /// Automatically switches provider based on authOptions.provider.
    pub async fn connect(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        debug().info(
            DebugCategory::PROVIDER_MANAGER,
            "Starting connection",
            None,
        );

        // Validate that the requested provider is allowed
        if !self.config.providers.contains(&auth_options.provider) {
            let error = format!(
                "Provider {:?} is not in the allowed providers list",
                auth_options.provider
            );
            debug().error(DebugCategory::PROVIDER_MANAGER, &error, None);
            return Err(error.into());
        }

        // Auto-switch provider based on auth type
        match auth_options.provider {
            AuthProviderType::Injected => {
                *self.current_provider_key.write().await = Some("injected".to_string());
            }
            AuthProviderType::Deeplink => {
                debug().log(
                    DebugCategory::PROVIDER_MANAGER,
                    "Deeplink provider: returning redirect result",
                    None,
                );
                return Ok(ConnectResult {
                    addresses: vec![],
                    wallet_id: None,
                    auth_user_id: None,
                    auth_provider: Some(AuthProviderType::Deeplink),
                    status: None,
                    wallet: None,
                });
            }
            ref p if EMBEDDED_PROVIDER_AUTH_TYPES.contains(p) => {
                let wallet_type = self
                    .config
                    .embedded_wallet_type
                    .clone()
                    .unwrap_or_else(|| "user-wallet".to_string());
                *self.current_provider_key.write().await =
                    Some(format!("embedded-{}", wallet_type));
            }
            _ => {}
        }

        let provider = self
            .get_current_provider()
            .await
            .ok_or("No provider selected")?;

        let result = provider.connect(auth_options).await?;

        // Save provider preference after successful connection
        self.save_provider_preference().await;

        // Emit connect event
        self.emit("connect", Some(serde_json::json!({
            "addresses": result.addresses.len(),
            "provider": format!("{:?}", auth_options.provider),
        }))).await;

        debug().info(
            DebugCategory::PROVIDER_MANAGER,
            "Connect completed",
            None,
        );

        Ok(result)
    }

    /// Disconnect from the current provider.
    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(provider) = self.get_current_provider().await {
            provider.disconnect().await?;
            self.emit("disconnect", None).await;
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

    /// Get enabled address types from the current provider.
    pub async fn get_enabled_address_types(
        &self,
    ) -> Vec<phantom_client::constants::AddressFormat> {
        if let Some(provider) = self.get_current_provider().await {
            provider.get_enabled_address_types()
        } else {
            self.config.address_types.clone()
        }
    }

    /// Attempt auto-connect with fallback strategy.
    ///
    /// Tries embedded provider first if it exists and is allowed, then injected.
    pub async fn auto_connect(&self) -> bool {
        debug().log(
            DebugCategory::PROVIDER_MANAGER,
            "Starting auto-connect with fallback strategy",
            None,
        );

        let embedded_wallet_type = self
            .config
            .embedded_wallet_type
            .clone()
            .unwrap_or_else(|| "user-wallet".to_string());
        let embedded_key = format!("embedded-{}", embedded_wallet_type);

        // Check if embedded providers are allowed
        let embedded_allowed = self
            .config
            .providers
            .iter()
            .any(|p| !matches!(p, AuthProviderType::Injected | AuthProviderType::Deeplink));

        // Try embedded provider first if it exists and is allowed
        if embedded_allowed {
            let provider = self.providers.read().await.get(&embedded_key).cloned();
            if let Some(embedded_provider) = provider {
                debug().log(
                    DebugCategory::PROVIDER_MANAGER,
                    "Trying auto-connect with existing embedded provider",
                    None,
                );

                if let Ok(()) = embedded_provider.auto_connect().await {
                    if embedded_provider.is_connected() {
                        *self.current_provider_key.write().await =
                            Some(embedded_key.clone());
                        debug().info(
                            DebugCategory::PROVIDER_MANAGER,
                            "Embedded auto-connect successful",
                            None,
                        );
                        self.save_provider_preference().await;
                        return true;
                    }
                }
            }
        }

        // Check if injected provider is allowed
        let injected_allowed = self
            .config
            .providers
            .contains(&AuthProviderType::Injected);

        if injected_allowed {
            let provider = self.providers.read().await.get("injected").cloned();
            if let Some(injected_provider) = provider {
                debug().log(
                    DebugCategory::PROVIDER_MANAGER,
                    "Trying auto-connect with existing injected provider",
                    None,
                );

                if let Ok(()) = injected_provider.auto_connect().await {
                    if injected_provider.is_connected() {
                        *self.current_provider_key.write().await =
                            Some("injected".to_string());
                        debug().info(
                            DebugCategory::PROVIDER_MANAGER,
                            "Injected auto-connect successful",
                            None,
                        );
                        self.save_provider_preference().await;
                        return true;
                    }
                }
            }
        }

        debug().log(
            DebugCategory::PROVIDER_MANAGER,
            "Auto-connect failed for all allowed providers",
            None,
        );
        false
    }

    /// Add event listener. Returns a listener ID for removal.
    pub async fn on(
        &self,
        event: &str,
        callback: Arc<dyn Fn(Option<serde_json::Value>) + Send + Sync>,
    ) -> u64 {
        let mut next_id = self.next_listener_id.write().await;
        let id = *next_id;
        *next_id += 1;

        let mut listeners = self.event_listeners.write().await;
        listeners
            .entry(event.to_string())
            .or_default()
            .push((id, callback));

        id
    }

    /// Remove event listener by ID.
    pub async fn off(&self, event: &str, listener_id: u64) {
        let mut listeners = self.event_listeners.write().await;
        if let Some(list) = listeners.get_mut(event) {
            list.retain(|(id, _)| *id != listener_id);
            if list.is_empty() {
                listeners.remove(event);
            }
        }
    }

    /// Emit event to all registered callbacks.
    pub async fn emit(&self, event: &str, data: Option<serde_json::Value>) {
        let listeners = self.event_listeners.read().await;
        if let Some(list) = listeners.get(event) {
            for (_, callback) in list {
                let cb = callback.clone();
                let data = data.clone();
                if let Err(e) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        cb(data);
                    }))
                {
                    debug().error(
                        DebugCategory::PROVIDER_MANAGER,
                        &format!("Event callback error for '{}': {:?}", event, e),
                        None,
                    );
                }
            }
        }
    }

    /// Save provider preference (platform-specific persistence).
    async fn save_provider_preference(&self) {
        if let Some(pref) = self.get_current_provider_info().await {
            debug().log(
                DebugCategory::PROVIDER_MANAGER,
                &format!(
                    "Provider preference: type={}, walletType={:?}",
                    pref.provider_type, pref.embedded_wallet_type
                ),
                None,
            );
            // In a browser environment, this would save to localStorage.
            // In Rust CLI/server contexts, this is a no-op.
        }
    }

    /// Get a reference to the concrete injected provider, if available.
    ///
    /// This provides direct access to chain-specific methods (e.g., `solana()`,
    /// `ethereum()`) that are not part of the generic `Provider` trait.
    pub async fn get_injected_provider(&self) -> Option<Arc<InjectedProvider>> {
        self.injected_provider.read().await.clone()
    }
}

/// Generate a unique key for provider instances.
fn get_provider_key(provider_type: &str, embedded_wallet_type: Option<&str>) -> String {
    match provider_type {
        "injected" => "injected".to_string(),
        "embedded" => format!(
            "embedded-{}",
            embedded_wallet_type.unwrap_or("app-wallet")
        ),
        _ => provider_type.to_string(),
    }
}
