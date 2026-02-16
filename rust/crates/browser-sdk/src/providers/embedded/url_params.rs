//! Browser-specific URL parameter accessor implementation.
//!
//! Implements the [`UrlParamsAccessor`] trait from `phantom-embedded-provider-core`.
//! In the TypeScript SDK, this reads from `window.location.search` via
//! `URLSearchParams`. In native Rust, this supports multiple parameter sources:
//! environment variables, explicit parameter maps, or URL string parsing.

use phantom_embedded_provider_core::UrlParamsAccessor;
use std::collections::HashMap;

/// Browser URL parameters accessor.
///
/// Provides access to URL query parameters for auth callback detection and
/// redirect flow resumption. Supports three modes of operation:
///
/// 1. **Explicit parameters** — pass a pre-parsed `HashMap` of key-value pairs.
/// 2. **URL string** — parse parameters from a URL query string.
/// 3. **Environment variable fallback** — reads `PHANTOM_AUTH_PARAMS` as a
///    `key=value&key2=value2` string (useful for CLI/testing).
///
/// # Examples
///
/// ```
/// use phantom_browser_sdk::providers::embedded::BrowserURLParamsAccessor;
/// use phantom_embedded_provider_core::UrlParamsAccessor;
/// use std::collections::HashMap;
///
/// // From explicit parameters
/// let mut params = HashMap::new();
/// params.insert("session_id".to_string(), "abc123".to_string());
/// params.insert("wallet_id".to_string(), "wallet456".to_string());
/// let accessor = BrowserURLParamsAccessor::from_params(params);
/// assert_eq!(accessor.get_param("session_id"), Some("abc123".to_string()));
///
/// // From a URL string
/// let accessor = BrowserURLParamsAccessor::from_url("https://example.com?session_id=abc&wallet_id=xyz");
/// assert_eq!(accessor.get_param("session_id"), Some("abc".to_string()));
/// ```
pub struct BrowserURLParamsAccessor {
    params: HashMap<String, String>,
}

impl BrowserURLParamsAccessor {
    /// Create a new accessor with empty parameters.
    ///
    /// Falls back to reading the `PHANTOM_AUTH_PARAMS` environment variable
    /// if set, parsing it as URL-encoded query parameters.
    pub fn new() -> Self {
        let params = Self::params_from_env().unwrap_or_default();
        Self { params }
    }

    /// Create an accessor from an explicit parameter map.
    pub fn from_params(params: HashMap<String, String>) -> Self {
        Self { params }
    }

    /// Create an accessor by parsing a URL string.
    ///
    /// Extracts query parameters from the URL's query component.
    /// If the URL has no query string, the accessor will have no parameters.
    pub fn from_url(url: &str) -> Self {
        let params = Self::parse_query_string(url);
        Self { params }
    }

    /// Parse query parameters from a URL or raw query string.
    ///
    /// Handles both full URLs (`https://example.com?key=val`) and
    /// bare query strings (`key=val&key2=val2`).
    fn parse_query_string(input: &str) -> HashMap<String, String> {
        let query = if let Some(pos) = input.find('?') {
            &input[pos + 1..]
        } else {
            input
        };

        // Strip fragment if present.
        let query = if let Some(pos) = query.find('#') {
            &query[..pos]
        } else {
            query
        };

        query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((Self::url_decode(key), Self::url_decode(value)))
            })
            .collect()
    }

    /// Read parameters from the `PHANTOM_AUTH_PARAMS` environment variable.
    fn params_from_env() -> Option<HashMap<String, String>> {
        let env_val = std::env::var("PHANTOM_AUTH_PARAMS").ok()?;
        if env_val.is_empty() {
            return None;
        }
        Some(Self::parse_query_string(&env_val))
    }

    /// Basic percent-decoding for URL parameter values.
    fn url_decode(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.bytes();

        while let Some(b) = chars.next() {
            if b == b'%' {
                let high = chars.next();
                let low = chars.next();
                if let (Some(h), Some(l)) = (high, low) {
                    let hex = [h, l];
                    if let Ok(s) = std::str::from_utf8(&hex) {
                        if let Ok(byte) = u8::from_str_radix(s, 16) {
                            result.push(byte as char);
                            continue;
                        }
                    }
                }
                // Malformed percent encoding: emit literally.
                result.push('%');
                if let Some(h) = high {
                    result.push(h as char);
                }
                if let Some(l) = low {
                    result.push(l as char);
                }
            } else if b == b'+' {
                result.push(' ');
            } else {
                result.push(b as char);
            }
        }

        result
    }
}

impl Default for BrowserURLParamsAccessor {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlParamsAccessor for BrowserURLParamsAccessor {
    fn get_param(&self, key: &str) -> Option<String> {
        self.params.get(key).cloned()
    }
}
