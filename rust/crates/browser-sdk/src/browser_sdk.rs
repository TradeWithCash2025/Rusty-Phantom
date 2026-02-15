//! Main BrowserSDK struct — the primary entry point for browser wallet integration.
//!
//! Combines embedded and injected providers behind a unified API,
//! with wallet discovery, debug logging, and auto-confirm support.

use phantom_browser_injected_sdk::ExtensionDetector;
use phantom_embedded_provider_core::WalletAddress;

use crate::debug::{debug, DebugCategory, DebugCallback, DebugLevel};
use crate::provider_manager::ProviderManager;
use crate::types::{AuthOptions, AuthProviderType, BrowserSdkConfig, ConnectResult};

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
    #[allow(dead_code)]
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

        let result = self.provider_manager.connect(options).await?;

        debug().info(
            DebugCategory::BROWSER_SDK,
            "Connection successful",
            None,
        );

        Ok(result)
    }

    /// Disconnect from the wallet.
    pub async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.provider_manager.disconnect().await
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
