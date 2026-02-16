//! Browser-specific Phantom app provider implementation.
//!
//! Implements the [`PhantomAppProvider`] trait from `phantom-embedded-provider-core`.
//! In the TypeScript SDK, this communicates with the Phantom browser extension
//! via `window.phantom.solana` to authenticate using the Phantom app (extension
//! or mobile). In native Rust, this reports the Phantom app as unavailable by
//! default, since direct extension communication requires a browser
//! runtime. Platform integrations (e.g., wasm-bindgen) can supply a real
//! implementation via the [`PhantomAppCallback`] hook.

use phantom_embedded_provider_core::{
    AuthResult, PhantomAppAuthOptions, PhantomAppProvider,
};

/// Callback type for custom Phantom app authentication.
///
/// Platform integrations can provide a callback that performs the actual
/// Phantom app authentication (e.g., via browser extension messaging or
/// deep linking). The callback receives the auth options and returns
/// an [`AuthResult`] on success.
pub type PhantomAppCallback = Box<
    dyn Fn(
            PhantomAppAuthOptions,
        )
            -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<AuthResult, Box<dyn std::error::Error + Send + Sync>>,
                        > + Send,
                >,
            > + Send
        + Sync,
>;

/// Browser Phantom app provider.
///
/// Handles authentication through the Phantom browser extension or mobile app.
/// In a native Rust environment without a browser runtime, `is_available()`
/// returns `false` unless a custom callback is provided.
///
/// # Examples
///
/// ```
/// use phantom_browser_sdk::providers::embedded::BrowserPhantomAppProvider;
///
/// // Default: Phantom app not available in native Rust
/// let provider = BrowserPhantomAppProvider::new();
/// assert!(!phantom_embedded_provider_core::PhantomAppProvider::is_available(&provider));
/// ```
pub struct BrowserPhantomAppProvider {
    /// Optional callback for Phantom app authentication.
    callback: Option<PhantomAppCallback>,
    /// Whether the Phantom app is available.
    available: bool,
}

impl BrowserPhantomAppProvider {
    /// Create a new Phantom app provider with default settings.
    ///
    /// The Phantom app is reported as unavailable since there is no browser
    /// extension to communicate with in a native Rust environment.
    pub fn new() -> Self {
        Self {
            callback: None,
            available: false,
        }
    }

    /// Create a Phantom app provider with a custom authentication callback.
    ///
    /// This allows platform integrations (e.g., wasm, mobile bridges) to
    /// supply the actual Phantom app communication logic.
    ///
    /// # Arguments
    /// * `callback` - Async callback that performs Phantom app authentication.
    pub fn with_callback(callback: PhantomAppCallback) -> Self {
        Self {
            callback: Some(callback),
            available: true,
        }
    }

    /// Mark the Phantom app as available or unavailable.
    ///
    /// This can be used after construction to update availability based
    /// on runtime detection (e.g., after checking for the Phantom extension).
    pub fn set_available(&mut self, available: bool) {
        self.available = available;
    }
}

impl Default for BrowserPhantomAppProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PhantomAppProvider for BrowserPhantomAppProvider {
    async fn authenticate(
        &self,
        options: PhantomAppAuthOptions,
    ) -> Result<AuthResult, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref callback) = self.callback {
            return callback(options).await;
        }

        Err(
            "Phantom app authentication is not available in this environment. \
             The Phantom browser extension or mobile app is required."
                .into(),
        )
    }

    fn is_available(&self) -> bool {
        self.available
    }
}
