//! Auto-confirm functionality for the Phantom browser-injected SDK.
//!
//! Provides types and operations for enabling/disabling auto-confirm
//! and querying auto-confirm status and supported chains.

use crate::Plugin;
use phantom_constants::NetworkId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// Types
// ============================================================================

/// Parameters for enabling auto-confirm.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoConfirmEnableParams {
    /// Optional list of chains to enable auto-confirm for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chains: Option<Vec<NetworkId>>,
}

/// Result of an auto-confirm state operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfirmResult {
    /// Whether auto-confirm is enabled.
    pub enabled: bool,
    /// Chains that have auto-confirm enabled.
    pub chains: Vec<NetworkId>,
}

/// Result of querying supported chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfirmSupportedChainsResult {
    /// Chains that support auto-confirm.
    pub chains: Vec<NetworkId>,
}

// ============================================================================
// Provider trait
// ============================================================================

/// Trait representing the auto-confirm provider (window.phantom.app in TS).
#[async_trait::async_trait]
pub trait AutoConfirmProvider: Send + Sync {
    /// Enable auto-confirm.
    async fn enable(
        &self,
        params: &AutoConfirmEnableParams,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Disable auto-confirm.
    async fn disable(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get auto-confirm status.
    async fn status(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get supported chains.
    async fn supported_chains(
        &self,
    ) -> Result<AutoConfirmSupportedChainsResult, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// AutoConfirm plugin struct
// ============================================================================

/// Auto-confirm plugin providing enable/disable/status/supportedChains.
pub struct AutoConfirm {
    provider: Box<dyn AutoConfirmProvider>,
}

impl AutoConfirm {
    /// Create a new AutoConfirm with the given provider.
    pub fn new(provider: Box<dyn AutoConfirmProvider>) -> Self {
        Self { provider }
    }

    /// Enable auto-confirm.
    pub async fn enable(
        &self,
        params: Option<&AutoConfirmEnableParams>,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        let default_params = AutoConfirmEnableParams::default();
        let params = params.unwrap_or(&default_params);
        self.provider.enable(params).await
    }

    /// Disable auto-confirm.
    pub async fn disable(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        self.provider.disable().await
    }

    /// Get auto-confirm status.
    pub async fn status(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        self.provider.status().await
    }

    /// Get supported chains.
    pub async fn supported_chains(
        &self,
    ) -> Result<AutoConfirmSupportedChainsResult, Box<dyn std::error::Error + Send + Sync>> {
        self.provider.supported_chains().await
    }
}

/// Create an auto-confirm plugin for the Phantom instance.
///
/// The returned `Plugin` wraps the given provider so callers can
/// enable/disable/query auto-confirm at runtime. The `AutoConfirm`
/// instance is created once and shared via `Arc` across multiple
/// calls to the plugin factory.
pub fn create_auto_confirm_plugin(provider: Box<dyn AutoConfirmProvider>) -> Plugin {
    let auto_confirm = Arc::new(AutoConfirm::new(provider));
    Plugin {
        name: "autoConfirm".to_string(),
        create: Box::new(move || Box::new(auto_confirm.clone())),
    }
}
