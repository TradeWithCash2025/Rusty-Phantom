//! Browser platform adapter that aggregates all browser-specific implementations.
//!
//! Implements the [`PlatformAdapter`] trait from `phantom-embedded-provider-core`,
//! combining all browser-specific adapters (auth, storage, stamper, URL params,
//! Phantom app, logger) into a single platform interface.

use phantom_embedded_provider_core::{
    AuthProvider, EmbeddedStorage, PlatformAdapter, PhantomAppProvider,
    UrlParamsAccessor,
};
use phantom_sdk_types::StamperWithKeyManagement;
use std::collections::HashMap;
use std::sync::Arc;

use super::auth::{BrowserAuthConfig, BrowserAuthProvider};
use super::phantom_app::BrowserPhantomAppProvider;
use super::storage::BrowserStorage;
use super::url_params::BrowserURLParamsAccessor;

/// Configuration for the browser platform adapter.
///
/// All fields except `stamper` are optional and will use sensible defaults
/// for a native Rust environment.
pub struct BrowserPlatformConfig {
    /// Stamper with key management (required).
    pub stamper: Box<dyn StamperWithKeyManagement>,
    /// Arc-wrapped stamper for passing to `PhantomClient`. If not provided,
    /// `stamper_for_client()` will return `None`.
    pub stamper_arc: Option<Arc<dyn phantom_sdk_types::Stamper>>,
    /// OAuth redirect URL. Used as the default for auth flows.
    pub redirect_url: Option<String>,
    /// Auth server base URL. Used as the default for auth flows.
    pub auth_url: Option<String>,
    /// Auth redirect handler callback. See [`BrowserAuthConfig`] for details.
    pub redirect_handler: Option<super::auth::AuthRedirectHandler>,
    /// Directory for file-based session storage. Defaults to `$HOME/.phantom`.
    pub storage_dir: Option<std::path::PathBuf>,
    /// Pre-populated URL parameters (e.g., from an auth callback).
    pub url_params: Option<HashMap<String, String>>,
    /// Additional analytics headers to include in API requests.
    pub analytics_headers: Option<HashMap<String, String>>,
}

/// Browser platform adapter that combines all platform-specific capabilities.
///
/// This is the top-level adapter that the embedded provider uses to interact
/// with browser-specific APIs. In the TypeScript SDK, this would wire up
/// IndexedDB stamper, localStorage, window.location, etc. In native Rust,
/// it uses file-backed storage, environment-based URL params, and tracing-based
/// logging.
///
/// # Examples
///
/// ```no_run
/// use phantom_browser_sdk::providers::embedded::{BrowserPlatformAdapter, BrowserPlatformConfig};
///
/// // You need a real StamperWithKeyManagement for production use.
/// // let adapter = BrowserPlatformAdapter::new(BrowserPlatformConfig {
/// //     stamper: my_stamper,
/// //     stamper_arc: Some(my_stamper_arc),
/// //     redirect_url: Some("http://localhost:3000/callback".into()),
/// //     auth_url: None,
/// //     redirect_handler: None,
/// //     storage_dir: None,
/// //     url_params: None,
/// //     analytics_headers: None,
/// // });
/// ```
pub struct BrowserPlatformAdapter {
    storage: BrowserStorage,
    auth_provider: BrowserAuthProvider,
    phantom_app_provider: BrowserPhantomAppProvider,
    url_params: Arc<dyn UrlParamsAccessor>,
    stamper: Box<dyn StamperWithKeyManagement>,
    stamper_arc: Option<Arc<dyn phantom_sdk_types::Stamper>>,
    analytics_headers: Option<HashMap<String, String>>,
}

impl BrowserPlatformAdapter {
    /// Create a new browser platform adapter from configuration.
    ///
    /// # Arguments
    /// * `config` - Platform configuration. Only `stamper` is required;
    ///   all other fields have sensible defaults.
    pub fn new(config: BrowserPlatformConfig) -> Self {
        let url_params = match config.url_params {
            Some(params) => BrowserURLParamsAccessor::from_params(params),
            None => BrowserURLParamsAccessor::new(),
        };

        // Wrap in Arc so the auth provider and platform adapter can share it.
        let url_params_arc: Arc<dyn UrlParamsAccessor> = Arc::new(url_params);

        let auth_config = BrowserAuthConfig {
            redirect_handler: config.redirect_handler,
            redirect_url: config.redirect_url,
            auth_url: config.auth_url,
            redirect_result: None,
            url_params: url_params_arc.clone(),
        };

        Self {
            storage: BrowserStorage::new(config.storage_dir),
            auth_provider: BrowserAuthProvider::new(auth_config),
            phantom_app_provider: BrowserPhantomAppProvider::new(),
            url_params: url_params_arc,
            stamper: config.stamper,
            stamper_arc: config.stamper_arc,
            analytics_headers: config.analytics_headers,
        }
    }

    /// Get a reference to the auth provider for direct interaction.
    ///
    /// Useful for calling [`BrowserAuthProvider::set_redirect_result()`]
    /// when resuming from an OAuth callback.
    pub fn auth(&self) -> &BrowserAuthProvider {
        &self.auth_provider
    }

    /// Get a reference to the storage for direct interaction.
    pub fn storage_ref(&self) -> &BrowserStorage {
        &self.storage
    }

    /// Set a shared `Arc<dyn Stamper>` for use with `PhantomClient`.
    pub fn set_stamper_arc(&mut self, stamper: Arc<dyn phantom_sdk_types::Stamper>) {
        self.stamper_arc = Some(stamper);
    }

    /// Set analytics headers.
    pub fn set_analytics_headers(&mut self, headers: HashMap<String, String>) {
        self.analytics_headers = Some(headers);
    }
}

impl PlatformAdapter for BrowserPlatformAdapter {
    fn name(&self) -> &str {
        "web"
    }

    fn storage(&self) -> &dyn EmbeddedStorage {
        &self.storage
    }

    fn auth_provider(&self) -> &dyn AuthProvider {
        &self.auth_provider
    }

    fn phantom_app_provider(&self) -> &dyn PhantomAppProvider {
        &self.phantom_app_provider
    }

    fn url_params_accessor(&self) -> &dyn UrlParamsAccessor {
        self.url_params.as_ref()
    }

    fn stamper(&self) -> &dyn StamperWithKeyManagement {
        self.stamper.as_ref()
    }

    fn stamper_for_client(&self) -> Option<Arc<dyn phantom_sdk_types::Stamper>> {
        self.stamper_arc.clone()
    }

    fn analytics_headers(&self) -> Option<HashMap<String, String>> {
        self.analytics_headers.clone()
    }
}
