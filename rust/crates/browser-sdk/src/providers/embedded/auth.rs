//! Browser-specific authentication provider implementation.
//!
//! Implements the [`AuthProvider`] trait from `phantom-embedded-provider-core`.
//! In the TypeScript SDK, this handles OAuth redirect flows by:
//! 1. Building a Phantom Connect auth URL with the appropriate parameters
//! 2. Redirecting the browser to the auth URL (via `window.location.href`)
//! 3. Resuming authentication from the redirect callback URL parameters
//!
//! In native Rust, the redirect is replaced by either:
//! - An [`AuthRedirectHandler`] callback that the caller can use to open a
//!   browser or handle the URL however is appropriate for the platform.
//! - A direct auth result callback for environments that can complete auth
//!   in-band (e.g., tests, server-side flows).

use phantom_embedded_provider_core::{
    AuthProvider, AuthResult, EmbeddedProviderAuthType, PhantomConnectOptions,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// Callback invoked when an OAuth redirect URL is constructed.
///
/// The implementation should present this URL to the user (e.g., open a browser
/// window, display a QR code, etc.). The callback returns `Ok(())` if the
/// redirect was initiated successfully.
pub type AuthRedirectHandler = Box<
    dyn Fn(String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync,
>;

/// Configuration for the browser auth provider.
pub struct BrowserAuthConfig {
    /// Handler called when the user needs to be redirected to an auth URL.
    /// If `None`, `authenticate()` will still return `Ok(None)` for redirect
    /// flows (the caller logs the URL), but no automatic redirect occurs.
    pub redirect_handler: Option<AuthRedirectHandler>,

    /// Default redirect URL for OAuth callbacks. Can be overridden per
    /// request via [`PhantomConnectOptions::redirect_url`].
    pub redirect_url: Option<String>,

    /// Default auth server base URL. Can be overridden per request via
    /// [`PhantomConnectOptions::auth_url`].
    pub auth_url: Option<String>,

    /// Pre-populated redirect result for resuming auth from a callback URL.
    /// This is set from URL parameters when the application loads after an
    /// OAuth redirect.
    pub redirect_result: Option<AuthResult>,
}

impl Default for BrowserAuthConfig {
    fn default() -> Self {
        Self {
            redirect_handler: None,
            redirect_url: None,
            auth_url: None,
            redirect_result: None,
        }
    }
}

/// Browser authentication provider.
///
/// Handles OAuth-based authentication flows (Google, Apple) by constructing
/// the appropriate Phantom Connect auth URL and managing the redirect lifecycle.
///
/// # Auth Flow
///
/// 1. **`authenticate()`** is called with [`PhantomConnectOptions`] specifying
///    the auth provider, public key, and redirect URL.
/// 2. The provider builds an auth URL with all required parameters.
/// 3. If configured, the [`AuthRedirectHandler`] is invoked to navigate the user.
/// 4. `authenticate()` returns `Ok(None)` indicating a redirect is in progress.
/// 5. After the OAuth provider redirects back, the application calls
///    [`set_redirect_result()`] with the parsed auth result, then
///    [`resume_auth_from_redirect()`] to retrieve it.
///
/// # Examples
///
/// ```
/// use phantom_browser_sdk::providers::embedded::{BrowserAuthProvider, BrowserAuthConfig};
///
/// // For testing: create with no redirect handler
/// let provider = BrowserAuthProvider::new(BrowserAuthConfig::default());
///
/// // With a redirect handler that opens a browser
/// let provider = BrowserAuthProvider::new(BrowserAuthConfig {
///     redirect_handler: Some(Box::new(|url| {
///         println!("Open this URL to authenticate: {}", url);
///         Ok(())
///     })),
///     redirect_url: Some("http://localhost:3000/callback".to_string()),
///     auth_url: None,
///     redirect_result: None,
/// });
/// ```
pub struct BrowserAuthProvider {
    /// Default redirect URL for OAuth callbacks.
    redirect_url: Option<String>,
    /// Default auth server base URL.
    auth_url: Option<String>,
    /// Handler for redirecting to auth URLs.
    redirect_handler: Option<AuthRedirectHandler>,
    /// Stored redirect result for resumption.
    redirect_result: Mutex<Option<AuthResult>>,
    /// Pending auth state keyed by session ID, used for redirect resumption.
    pending_auth: Mutex<HashMap<String, PendingAuthState>>,
}

/// Internal state saved before a redirect, used to complete auth on return.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingAuthState {
    /// The auth provider type (Google, Apple, etc.).
    provider: EmbeddedProviderAuthType,
    /// Session ID that ties the request to the response.
    session_id: String,
}

impl BrowserAuthProvider {
    /// Create a new browser auth provider with the given configuration.
    pub fn new(config: BrowserAuthConfig) -> Self {
        Self {
            redirect_url: config.redirect_url,
            auth_url: config.auth_url,
            redirect_handler: config.redirect_handler,
            redirect_result: Mutex::new(config.redirect_result),
            pending_auth: Mutex::new(HashMap::new()),
        }
    }

    /// Set a pre-parsed redirect result for auth resumption.
    ///
    /// Call this when the application loads and URL parameters indicate an
    /// auth callback (i.e., `session_id` and `wallet_id` or `response_type`
    /// are present in the URL).
    ///
    /// # Arguments
    /// * `result` - The parsed auth result from the redirect URL parameters.
    pub fn set_redirect_result(&self, result: AuthResult) {
        let mut guard = self.redirect_result.lock().unwrap();
        *guard = Some(result);
    }

    /// Build the full auth URL for a given set of connect options.
    ///
    /// Constructs a URL like:
    /// ```text
    /// {auth_url}/connect?provider={provider}&publicKey={key}&appId={id}
    ///     &sessionId={session}&redirectUrl={url}&clearPreviousSession={bool}
    ///     &allowRefresh={bool}&algorithm={algo}
    /// ```
    pub fn build_auth_url(&self, options: &PhantomConnectOptions) -> String {
        let base = options
            .auth_url
            .as_deref()
            .or(self.auth_url.as_deref())
            .unwrap_or("https://auth.phantom.app");

        let redirect = options
            .redirect_url
            .as_deref()
            .or(self.redirect_url.as_deref())
            .unwrap_or("");

        let provider_str = match options.provider {
            EmbeddedProviderAuthType::Google => "google",
            EmbeddedProviderAuthType::Apple => "apple",
            EmbeddedProviderAuthType::Phantom => "phantom",
            EmbeddedProviderAuthType::Device => "device",
        };

        let mut url = format!(
            "{}/connect?provider={}&publicKey={}&appId={}&sessionId={}&redirectUrl={}",
            base,
            provider_str,
            percent_encode(&options.public_key),
            percent_encode(&options.app_id),
            percent_encode(&options.session_id),
            percent_encode(redirect),
        );

        if let Some(clear) = options.clear_previous_session {
            url.push_str(&format!("&clearPreviousSession={}", clear));
        }

        if let Some(allow) = options.allow_refresh {
            url.push_str(&format!("&allowRefresh={}", allow));
        }

        if let Some(ref algorithm) = options.algorithm {
            url.push_str(&format!("&algorithm={:?}", algorithm));
        }

        url
    }
}

/// Simple percent-encoding for URL parameters.
fn percent_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[async_trait::async_trait]
impl AuthProvider for BrowserAuthProvider {
    /// Initiate an OAuth authentication flow.
    ///
    /// For redirect-based flows (Google, Apple), this builds the auth URL
    /// and invokes the redirect handler if configured. Returns `Ok(None)`
    /// to indicate a redirect is in progress and [`resume_auth_from_redirect()`]
    /// should be called after the redirect completes.
    ///
    /// If a redirect result has already been set (e.g., the app loaded from
    /// an auth callback URL), returns that result directly.
    ///
    /// For device auth, returns `Ok(None)` since the device flow is handled
    /// separately by the embedded provider core.
    async fn authenticate(
        &self,
        options: PhantomConnectOptions,
    ) -> Result<Option<AuthResult>, Box<dyn std::error::Error + Send + Sync>> {
        // Check if we already have a redirect result (resuming from callback).
        {
            let mut result_guard = self.redirect_result.lock().unwrap();
            if let Some(result) = result_guard.take() {
                return Ok(Some(result));
            }
        }

        // For device auth, the result comes back immediately (no redirect).
        // The embedded provider handles device auth flow separately.
        if matches!(options.provider, EmbeddedProviderAuthType::Device) {
            return Ok(None);
        }

        // Save pending state for redirect resumption.
        {
            let mut pending = self.pending_auth.lock().unwrap();
            pending.insert(
                options.session_id.clone(),
                PendingAuthState {
                    provider: options.provider,
                    session_id: options.session_id.clone(),
                },
            );
        }

        // Build the auth URL.
        let auth_url = self.build_auth_url(&options);

        // Invoke the redirect handler if available.
        if let Some(ref handler) = self.redirect_handler {
            handler(auth_url)?;
        } else {
            tracing::info!(
                url = auth_url.as_str(),
                "Auth redirect URL generated (no redirect handler configured)"
            );
        }

        // Redirect initiated (or URL logged); auth will complete on return.
        Ok(None)
    }

    /// Resume authentication from a redirect callback.
    ///
    /// Called after the OAuth provider redirects back to the application.
    /// Checks for a stored redirect result matching the given auth provider.
    ///
    /// # Arguments
    /// * `provider` - The expected auth provider type to match.
    ///
    /// # Returns
    /// `Some(AuthResult)` if a matching redirect result is available,
    /// `None` otherwise.
    fn resume_auth_from_redirect(
        &self,
        provider: EmbeddedProviderAuthType,
    ) -> Option<AuthResult> {
        let mut guard = self.redirect_result.lock().unwrap();
        if let Some(ref result) = *guard {
            if result.provider == provider {
                return guard.take();
            }
        }
        None
    }
}
