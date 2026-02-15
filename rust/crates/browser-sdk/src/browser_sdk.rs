//! Main BrowserSDK struct — the primary entry point for browser wallet integration.
//!
//! Combines embedded and injected providers behind a unified API,
//! with wallet discovery, debug logging, and auto-confirm support.

use phantom_browser_injected_sdk::auto_confirm::{
    AutoConfirmEnableParams, AutoConfirmResult, AutoConfirmSupportedChainsResult,
};
use phantom_browser_injected_sdk::ExtensionDetector;
use phantom_chain_interfaces::{EthereumChain, SolanaChain};
use phantom_client::constants::AddressFormat;
use phantom_embedded_provider_core::WalletAddress;
use std::sync::Arc;

use crate::debug::{debug, DebugCategory, DebugCallback, DebugLevel};
use crate::provider_manager::{ProviderManager, ProviderPreference, SwitchProviderOptions};
use crate::types::{AuthOptions, AuthProviderType, BrowserSdkConfig, ConnectResult, Provider};
use crate::wallets::{InjectedWalletInfo, InjectedWalletRegistry};

/// All valid provider types for the browser SDK.
const BROWSER_SDK_PROVIDER_TYPES: &[AuthProviderType] = &[
    AuthProviderType::Google,
    AuthProviderType::Apple,
    AuthProviderType::Phantom,
    AuthProviderType::Device,
    AuthProviderType::Injected,
    AuthProviderType::Deeplink,
];

/// Browser SDK for Phantom wallet integration.
///
/// The main entry point for interacting with Phantom wallets in a browser
/// environment. Supports both embedded (OAuth-based) and injected
/// (extension-based) providers.
pub struct BrowserSdk {
    provider_manager: ProviderManager,
    config: BrowserSdkConfig,
    is_loading: bool,
}

impl BrowserSdk {
    /// Create a new BrowserSDK instance.
    pub fn new(config: BrowserSdkConfig) -> Result<Self, String> {
        // Validate providers array
        if config.providers.is_empty() {
            return Err("providers must be a non-empty array of AuthProviderType".to_string());
        }

        // Validate each provider
        for p in &config.providers {
            if !BROWSER_SDK_PROVIDER_TYPES.contains(p) {
                return Err(format!("Invalid provider type: {:?}", p));
            }
        }

        // Check if any embedded providers are included
        let has_embedded = config
            .providers
            .iter()
            .any(|p| !matches!(p, AuthProviderType::Injected | AuthProviderType::Deeplink));

        // Validate appId for embedded providers
        if has_embedded && config.app_id.is_none() {
            return Err(
                "appId is required when using embedded providers (google, apple, phantom, etc.)"
                    .to_string(),
            );
        }

        // Validate embedded wallet type
        let embedded_wallet_type = config
            .embedded_wallet_type
            .as_deref()
            .unwrap_or("user-wallet");
        if !["app-wallet", "user-wallet"].contains(&embedded_wallet_type) {
            return Err(format!(
                "Invalid embeddedWalletType: {}. Must be \"app-wallet\" or \"user-wallet\".",
                embedded_wallet_type
            ));
        }

        let provider_manager = ProviderManager::new(config.clone());

        Ok(Self {
            provider_manager,
            config,
            is_loading: true,
        })
    }

    /// Connect to a wallet.
    pub async fn connect(
        &self,
        options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        debug().info(
            DebugCategory::BROWSER_SDK,
            "Starting connection",
            None,
        );

        match self.provider_manager.connect(options).await {
            Ok(result) => {
                debug().info(
                    DebugCategory::BROWSER_SDK,
                    "Connection successful",
                    None,
                );
                Ok(result)
            }
            Err(err) => {
                debug().error(
                    DebugCategory::BROWSER_SDK,
                    &format!("Connection failed: {}", err),
                    None,
                );
                Err(err)
            }
        }
    }

    /// Disconnect from the wallet.
    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.provider_manager.disconnect().await {
            Ok(()) => {
                debug().info(
                    DebugCategory::BROWSER_SDK,
                    "Disconnect successful",
                    None,
                );
                Ok(())
            }
            Err(err) => {
                debug().error(
                    DebugCategory::BROWSER_SDK,
                    &format!("Disconnect failed: {}", err),
                    None,
                );
                Err(err)
            }
        }
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        self.provider_manager.is_connected().await
    }

    /// Get wallet addresses.
    pub async fn get_addresses(&self) -> Vec<WalletAddress> {
        self.provider_manager.get_addresses().await
    }

    /// Attempt auto-connection.
    pub async fn auto_connect(&self) {
        let result = self.provider_manager.auto_connect().await;
        if result {
            debug().info(
                DebugCategory::BROWSER_SDK,
                "Auto-connect successful",
                None,
            );
        }
    }

    /// Whether the SDK is still loading.
    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Enable debug logging.
    pub fn enable_debug(&self) {
        debug().enable();
    }

    /// Disable debug logging.
    pub fn disable_debug(&self) {
        debug().disable();
    }

    /// Set debug level.
    pub fn set_debug_level(&self, level: DebugLevel) {
        debug().set_level(level);
    }

    /// Set debug callback.
    pub fn set_debug_callback(&self, callback: DebugCallback) {
        debug().set_callback(callback);
    }

    /// Configure debug settings.
    pub fn configure_debug(
        &self,
        enabled: Option<bool>,
        level: Option<DebugLevel>,
        callback: Option<DebugCallback>,
    ) {
        if let Some(e) = enabled {
            if e {
                self.enable_debug();
            } else {
                self.disable_debug();
            }
        }
        if let Some(l) = level {
            self.set_debug_level(l);
        }
        if let Some(cb) = callback {
            self.set_debug_callback(cb);
        }
    }

    /// Get the current provider info (type and wallet type).
    pub async fn get_current_provider_info(&self) -> Option<ProviderPreference> {
        self.provider_manager.get_current_provider_info().await
    }

    /// Get enabled address types from the current provider.
    pub async fn get_enabled_address_types(&self) -> Vec<AddressFormat> {
        self.provider_manager.get_enabled_address_types().await
    }

    /// Switch provider type.
    pub async fn switch_provider(
        &self,
        provider_type: &str,
        options: Option<SwitchProviderOptions>,
    ) -> Result<Arc<dyn Provider>, Box<dyn std::error::Error + Send + Sync>> {
        self.provider_manager
            .switch_provider(provider_type, options)
            .await
    }

    /// Register an event listener. Returns a listener ID for removal.
    pub async fn on(
        &self,
        event: &str,
        callback: Arc<dyn Fn(Option<serde_json::Value>) + Send + Sync>,
    ) -> u64 {
        self.provider_manager.on(event, callback).await
    }

    /// Remove an event listener by ID.
    pub async fn off(&self, event: &str, listener_id: u64) {
        self.provider_manager.off(event, listener_id).await;
    }

    /// Get the wallet registry for discovered injected wallets.
    pub fn wallet_registry(&self) -> Arc<InjectedWalletRegistry> {
        crate::wallets::get_wallet_registry()
    }

    /// Register an external provider (e.g., embedded provider from platform adapter).
    pub async fn register_provider(&self, key: &str, provider: Arc<dyn Provider>) {
        self.provider_manager.register_provider(key, provider).await;
    }

    /// Set the loading state.
    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
    }

    // ---------------------------------------------------------------
    // Chain accessors
    // ---------------------------------------------------------------

    /// Get the Solana chain provider from the current injected provider.
    ///
    /// Returns an error if no injected provider is active or the current
    /// provider does not support Solana.
    pub async fn solana(
        &self,
    ) -> Result<Arc<dyn SolanaChain>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(provider) = self.provider_manager.get_injected_provider().await {
            provider.solana().await
        } else {
            Err("No injected provider available for Solana chain access".into())
        }
    }

    /// Get the Ethereum chain provider from the current injected provider.
    ///
    /// Returns an error if no injected provider is active or the current
    /// provider does not support Ethereum.
    pub async fn ethereum(
        &self,
    ) -> Result<Arc<dyn EthereumChain>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(provider) = self.provider_manager.get_injected_provider().await {
            provider.ethereum().await
        } else {
            Err("No injected provider available for Ethereum chain access".into())
        }
    }

    // ---------------------------------------------------------------
    // Wallet discovery
    // ---------------------------------------------------------------

    /// Get discovered wallets filtered by the configured address types.
    ///
    /// Returns wallets from the registry that support at least one of the
    /// address types specified in the SDK configuration.
    pub fn get_discovered_wallets(&self) -> Vec<InjectedWalletInfo> {
        let registry = self.wallet_registry();
        registry.get_by_address_types(&self.config.address_types)
    }

    // ---------------------------------------------------------------
    // Auto-confirm delegation
    // ---------------------------------------------------------------

    /// Enable auto-confirm via the injected provider.
    ///
    /// Delegates to the injected provider's auto-confirm functionality.
    /// Returns an error if no injected provider is available.
    pub async fn enable_auto_confirm(
        &self,
        _params: Option<&AutoConfirmEnableParams>,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        // Auto-confirm requires a browser extension context. The injected
        // provider communicates with the extension via RPC. In a pure-Rust
        // environment we surface an explicit error.
        if self.provider_manager.get_injected_provider().await.is_none() {
            return Err("No injected provider available for auto-confirm".into());
        }
        Err("Auto-confirm is only available in browser extension contexts".into())
    }

    /// Disable auto-confirm via the injected provider.
    ///
    /// Returns an error if no injected provider is available.
    pub async fn disable_auto_confirm(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        if self.provider_manager.get_injected_provider().await.is_none() {
            return Err("No injected provider available for auto-confirm".into());
        }
        Err("Auto-confirm is only available in browser extension contexts".into())
    }

    /// Get auto-confirm status via the injected provider.
    ///
    /// Returns an error if no injected provider is available.
    pub async fn get_auto_confirm_status(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        if self.provider_manager.get_injected_provider().await.is_none() {
            return Err("No injected provider available for auto-confirm".into());
        }
        Err("Auto-confirm is only available in browser extension contexts".into())
    }

    /// Get supported auto-confirm chains via the injected provider.
    ///
    /// Returns an error if no injected provider is available.
    pub async fn get_supported_auto_confirm_chains(
        &self,
    ) -> Result<AutoConfirmSupportedChainsResult, Box<dyn std::error::Error + Send + Sync>> {
        if self.provider_manager.get_injected_provider().await.is_none() {
            return Err("No injected provider available for auto-confirm".into());
        }
        Err("Auto-confirm is only available in browser extension contexts".into())
    }
}

/// Wait for Phantom extension to be available with retry logic.
///
/// Polls for the Phantom extension every 100ms until it is detected or
/// the timeout is reached.
///
/// # Arguments
/// * `detector` - Extension detector implementation (platform-provided).
/// * `timeout_ms` - Maximum time to wait in milliseconds (default: 3000).
///
/// # Returns
/// `true` if the Phantom extension is available, `false` if the timeout is reached.
pub async fn wait_for_phantom_extension(
    detector: &dyn ExtensionDetector,
    timeout_ms: u64,
) -> bool {
    let start = std::time::Instant::now();
    let check_interval = std::time::Duration::from_millis(100);
    let timeout = std::time::Duration::from_millis(timeout_ms);

    loop {
        if detector.is_installed() {
            return true;
        }

        if start.elapsed() >= timeout {
            return false;
        }

        tokio::time::sleep(check_interval).await;
    }
}

/// Features response from the Phantom extension.
pub trait PhantomFeaturesProvider: Send + Sync {
    /// Query available features from the Phantom extension.
    ///
    /// Returns a list of feature identifiers (e.g., `["phantom_login"]`).
    fn features(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>> + Send + '_>,
    >;
}

/// Check if Phantom Login is available.
///
/// This function checks if:
/// 1. The Phantom extension is installed (via detector)
/// 2. The extension supports the `phantom_login` feature (via features provider)
///
/// # Arguments
/// * `detector` - Extension detector implementation.
/// * `features_provider` - Provider for querying extension features.
/// * `timeout_ms` - Maximum time to wait for extension in milliseconds (default: 3000).
///
/// # Returns
/// `true` if Phantom Login is available, `false` otherwise.
pub async fn is_phantom_login_available(
    detector: &dyn ExtensionDetector,
    features_provider: &dyn PhantomFeaturesProvider,
    timeout_ms: u64,
) -> bool {
    // First, wait for the extension to be installed
    let extension_installed = wait_for_phantom_extension(detector, timeout_ms).await;
    if !extension_installed {
        return false;
    }

    // Check if the features API returns phantom_login
    match features_provider.features().await {
        Ok(features) => features.iter().any(|f| f == "phantom_login"),
        Err(_) => false,
    }
}
