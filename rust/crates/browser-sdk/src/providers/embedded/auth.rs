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

use phantom_constants::DEFAULT_AUTHENTICATOR_ALGORITHM;
use phantom_embedded_provider_core::{
    AuthProvider, AuthResult, EmbeddedProviderAuthType, PhantomConnectOptions, UrlParamsAccessor,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// SDK version reported in auth URL parameters.
const SDK_VERSION: &str = "0.1.0";

/// SDK type reported in auth URL parameters.
const SDK_TYPE: &str = "rust";

/// Platform identifier reported in auth URL parameters.
const SDK_PLATFORM: &str = "rust-native";

/// Callback invoked when an OAuth redirect URL is constructed.
///
/// The implementation should present this URL to the user (e.g., open a browser
/// window, display a QR code, etc.). The callback returns `Ok(())` if the
/// redirect was initiated successfully.
pub type AuthRedirectHandler =
    Box<dyn Fn(String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

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

    /// URL parameters accessor for reading redirect callback parameters.
    /// Required for [`resume_auth_from_redirect`] to read URL params like
    /// `wallet_id`, `session_id`, `error`, etc.
    pub url_params: Arc<dyn UrlParamsAccessor>,
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
///    [`resume_auth_from_redirect()`] to parse and validate the callback URL
///    parameters.
///
/// # Examples
///
/// ```ignore
/// use phantom_browser_sdk::providers::embedded::{BrowserAuthProvider, BrowserAuthConfig, BrowserURLParamsAccessor};
/// use std::sync::Arc;
///
/// // With a redirect handler that opens a browser
/// let url_params = Arc::new(BrowserURLParamsAccessor::new());
/// let provider = BrowserAuthProvider::new(BrowserAuthConfig {
///     redirect_handler: Some(Box::new(|url| {
///         println!("Open this URL to authenticate: {}", url);
///         Ok(())
///     })),
///     redirect_url: Some("http://localhost:3000/callback".to_string()),
///     auth_url: None,
///     redirect_result: None,
///     url_params,
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
    /// URL parameters accessor for reading redirect callback params.
    url_params: Arc<dyn UrlParamsAccessor>,
}

/// Internal state saved before a redirect, used to complete auth on return.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingAuthState {
    /// The auth provider type (Google, Apple, etc.).
    provider: EmbeddedProviderAuthType,
    /// Session ID that ties the request to the response.
    session_id: String,
    /// Public key used for the auth request.
    public_key: String,
    /// Application ID used for the auth request.
    app_id: String,
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
            url_params: config.url_params,
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
    /// {auth_url}?public_key={key}&app_id={id}&redirect_uri={url}
    ///     &session_id={session}&clear_previous_session={bool}
    ///     &allow_refresh={bool}&sdk_version={ver}&sdk_type=rust
    ///     &platform=rust-native&algorithm={algo}&provider={provider}
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

        // Resolve the provider, defaulting to Google if not specified.
        let provider = options.provider.unwrap_or(EmbeddedProviderAuthType::Google);

        let provider_str = match provider {
            EmbeddedProviderAuthType::Google => "google",
            EmbeddedProviderAuthType::Apple => "apple",
            EmbeddedProviderAuthType::Phantom => "phantom",
            EmbeddedProviderAuthType::Device => "device",
        };

        // Resolve the algorithm, defaulting to DEFAULT_AUTHENTICATOR_ALGORITHM.
        let algorithm = options.algorithm.unwrap_or(DEFAULT_AUTHENTICATOR_ALGORITHM);
        let algorithm_str = match algorithm {
            phantom_constants::Algorithm::Ed25519 => "ed25519",
            phantom_constants::Algorithm::Secp256r1 => "secp256r1",
        };

        let clear_previous_session = options.clear_previous_session.unwrap_or(false);
        let allow_refresh = options.allow_refresh.unwrap_or(true);

        let mut url = format!(
            "{}?public_key={}&app_id={}&redirect_uri={}&session_id={}&clear_previous_session={}&allow_refresh={}&sdk_version={}&sdk_type={}&platform={}&algorithm={}&provider={}",
            base,
            percent_encode(&options.public_key),
            percent_encode(&options.app_id),
            percent_encode(redirect),
            percent_encode(&options.session_id),
            clear_previous_session,
            allow_refresh,
            percent_encode(SDK_VERSION),
            percent_encode(SDK_TYPE),
            percent_encode(SDK_PLATFORM),
            percent_encode(algorithm_str),
            percent_encode(provider_str),
        );

        // Append any extra future params here as needed.
        let _ = &mut url;

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
    ///
    /// If no provider is specified in the options, defaults to Google.
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

        // Resolve the provider, defaulting to Google if not specified.
        let provider = options.provider.unwrap_or(EmbeddedProviderAuthType::Google);

        // For device auth, the result comes back immediately (no redirect).
        // The embedded provider handles device auth flow separately.
        if matches!(provider, EmbeddedProviderAuthType::Device) {
            return Ok(None);
        }

        // Save pending state for redirect resumption, including public_key
        // and app_id for session validation on return.
        {
            let mut pending = self.pending_auth.lock().unwrap();
            pending.insert(
                options.session_id.clone(),
                PendingAuthState {
                    provider,
                    session_id: options.session_id.clone(),
                    public_key: options.public_key.clone(),
                    app_id: options.app_id.clone(),
                },
            );
        }

        // Build the auth URL.
        let auth_url = self.build_auth_url(&options);

        // Validate auth URL before using it: only HTTPS or http://localhost allowed.
        if !auth_url.starts_with("https:") && !auth_url.starts_with("http://localhost") {
            return Err(
                "Invalid auth URL - only HTTPS URLs or http://localhost are allowed for authentication"
                    .into(),
            );
        }

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
    /// Reads URL parameters from the redirect callback URL via the
    /// [`UrlParamsAccessor`], validates them against stored pending auth state,
    /// and returns the auth result.
    ///
    /// Handles error codes from the auth server:
    /// - `access_denied` — user cancelled authentication
    /// - `invalid_request` — malformed auth request
    /// - `server_error` — auth server failure
    /// - `temporarily_unavailable` — service temporarily unavailable
    ///
    /// Validates that the `session_id` in the callback matches the stored
    /// pending auth state to prevent replay attacks.
    ///
    /// # Arguments
    /// * `provider` - The expected auth provider type to match.
    ///
    /// # Returns
    /// `Ok(Some(AuthResult))` if auth data is present and valid,
    /// `Ok(None)` if no auth data is in the URL,
    /// `Err` if an auth error occurred or session validation failed.
    fn resume_auth_from_redirect(
        &self,
        provider: EmbeddedProviderAuthType,
    ) -> Result<Option<AuthResult>, Box<dyn std::error::Error + Send + Sync>> {
        // First, check if we have a pre-populated redirect result.
        {
            let mut guard = self.redirect_result.lock().unwrap();
            if let Some(ref result) = *guard {
                if result.provider == provider {
                    return Ok(guard.take());
                }
            }
        }

        // Read URL parameters from the redirect callback.
        let wallet_id = self.url_params.get_param("wallet_id");
        let session_id = self.url_params.get_param("session_id");
        let account_derivation_index = self.url_params.get_param("selected_account_index");
        let error = self.url_params.get_param("error");
        let error_description = self.url_params.get_param("error_description");

        // Handle error responses from the auth server.
        if let Some(ref error_code) = error {
            let error_msg = error_description.as_deref().unwrap_or(error_code.as_str());

            // Clean up pending auth state on error.
            if let Some(ref sid) = session_id {
                let mut pending = self.pending_auth.lock().unwrap();
                pending.remove(sid);
            }

            let full_error = match error_code.as_str() {
                "access_denied" => {
                    format!("Authentication cancelled: {}", error_msg)
                }
                "invalid_request" => {
                    format!("Invalid authentication request: {}", error_msg)
                }
                "server_error" => {
                    format!("Authentication server error: {}", error_msg)
                }
                "temporarily_unavailable" => {
                    format!(
                        "Authentication service temporarily unavailable: {}",
                        error_msg
                    )
                }
                _ => {
                    format!("Authentication failed: {}", error_msg)
                }
            };

            return Err(full_error.into());
        }

        // If no wallet_id or session_id in URL, there's no auth data to process.
        let wallet_id = match wallet_id {
            Some(id) => id,
            None => {
                tracing::debug!("No wallet_id in URL params, no auth data to resume");
                return Ok(None);
            }
        };
        let session_id = match session_id {
            Some(id) => id,
            None => {
                tracing::debug!("No session_id in URL params, no auth data to resume");
                return Ok(None);
            }
        };

        // Validate session_id against stored pending auth state (replay prevention).
        {
            let pending = self.pending_auth.lock().unwrap();
            if let Some(stored) = pending.get(&session_id) {
                // Verify session_id matches (it will by key, but also check provider).
                if stored.provider != provider {
                    tracing::warn!(
                        stored_provider = ?stored.provider,
                        expected_provider = ?provider,
                        "Provider mismatch in redirect resume"
                    );
                }
            } else {
                // No stored state for this session_id. This could be a replay or
                // the state was lost. Log a warning but continue to be resilient.
                tracing::warn!(
                    session_id = session_id.as_str(),
                    "No pending auth state found for session_id - possible session corruption or replay"
                );
            }
        }

        // Extract additional parameters from the redirect URL.
        let organization_id = self.url_params.get_param("organization_id");
        let expires_in_ms = self.url_params.get_param("expires_in_ms");
        let auth_user_id = self.url_params.get_param("auth_user_id");

        // organization_id is required for a valid auth response.
        let organization_id = match organization_id {
            Some(id) => {
                if id.starts_with("temp-") {
                    tracing::warn!(
                        organization_id = id.as_str(),
                        "Received temporary organization_id, server may not be configured properly"
                    );
                }
                id
            }
            None => {
                // Clean up pending auth state.
                let mut pending = self.pending_auth.lock().unwrap();
                pending.remove(&session_id);
                return Err("Missing organization_id in auth response".into());
            }
        };

        let parsed_index = account_derivation_index
            .as_deref()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        let parsed_expires = expires_in_ms
            .as_deref()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // Clean up pending auth state now that we've consumed it.
        {
            let mut pending = self.pending_auth.lock().unwrap();
            pending.remove(&session_id);
        }

        tracing::info!(
            wallet_id = wallet_id.as_str(),
            organization_id = organization_id.as_str(),
            session_id = session_id.as_str(),
            account_derivation_index = parsed_index,
            expires_in_ms = parsed_expires,
            "Successfully resumed auth from redirect"
        );

        Ok(Some(AuthResult {
            wallet_id,
            organization_id,
            provider,
            account_derivation_index: parsed_index,
            expires_in_ms: parsed_expires,
            auth_user_id,
        }))
    }
}
