//! Provider name constants and lookup functions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Key identifying a provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderNameKey {
    /// Google OAuth provider.
    Google,
    /// Apple OAuth provider.
    Apple,
    /// Phantom native provider.
    Phantom,
    /// Device-based provider.
    Device,
    /// Injected wallet provider.
    Injected,
    /// Deeplink provider.
    Deeplink,
}

impl ProviderNameKey {
    /// Returns the human-readable display name for this provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Apple => "Apple",
            Self::Phantom => "Phantom",
            Self::Device => "Device",
            Self::Injected => "Wallet",
            Self::Deeplink => "Deeplink",
        }
    }
}

impl fmt::Display for ProviderNameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Get the human-readable display name for a provider.
///
/// If the provider string matches a known key, returns its display name.
/// Otherwise returns "Wallet" as the default.
pub fn get_provider_name(provider: &str) -> &str {
    match provider {
        "google" => "Google",
        "apple" => "Apple",
        "phantom" => "Phantom",
        "device" => "Device",
        "injected" => "Wallet",
        "deeplink" => "Deeplink",
        _ => "Wallet",
    }
}
