//! Shared types for the browser-injected SDK.

use serde::{Deserialize, Serialize};

/// Strategy for how to obtain the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderStrategy {
    /// Use the browser-injected provider (window.phantom).
    Injected,
}

impl std::fmt::Display for ProviderStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderStrategy::Injected => write!(f, "injected"),
        }
    }
}
