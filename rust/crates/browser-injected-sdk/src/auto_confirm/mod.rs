//! Auto-confirm functionality for the Phantom browser-injected SDK.
//!
//! Provides types and operations for enabling/disabling auto-confirm
//! and querying auto-confirm status and supported chains.
//!
//! Mirrors the TypeScript implementation, including CAIP transformations
//! between public `NetworkId` values and internal `InternalNetworkCaip`
//! values used for extension communication.

use crate::Plugin;
use phantom_constants::{
    internal_caip_to_network_id, network_id_to_internal_caip, InternalNetworkCaip, NetworkId,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ============================================================================
// RPC method name constants
// ============================================================================

/// RPC method name for enabling auto-confirm.
pub const RPC_AUTO_CONFIRM_ENABLE: &str = "phantom_auto_confirm_enable";
/// RPC method name for disabling auto-confirm.
pub const RPC_AUTO_CONFIRM_DISABLE: &str = "phantom_auto_confirm_disable";
/// RPC method name for querying auto-confirm status.
pub const RPC_AUTO_CONFIRM_STATUS: &str = "phantom_auto_confirm_status";
/// RPC method name for querying supported chains.
pub const RPC_AUTO_CONFIRM_SUPPORTED_CHAINS: &str = "phantom_auto_confirm_supported_chains";

// ============================================================================
// Public types (using NetworkId)
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
// Internal types (using InternalNetworkCaip for extension communication)
// ============================================================================

/// Internal parameters for enabling auto-confirm, using CAIP identifiers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InternalAutoConfirmEnableParams {
    /// Optional list of chains in internal CAIP format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chains: Option<Vec<InternalNetworkCaip>>,
}

/// Internal response for auto-confirm state operations, using CAIP identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfirmStateResponse {
    /// Whether auto-confirm is enabled.
    pub enabled: bool,
    /// Chains in internal CAIP format.
    pub chains: Vec<InternalNetworkCaip>,
}

/// Internal response for supported chains queries, using CAIP identifiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfirmSupportResponse {
    /// Chains in internal CAIP format.
    pub chains: Vec<InternalNetworkCaip>,
}

// ============================================================================
// RPC request/response types (discriminated union in Rust)
// ============================================================================

/// Auto-confirm RPC request, mirroring the TS discriminated union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum AutoConfirmRequest {
    /// Enable auto-confirm.
    #[serde(rename = "phantom_auto_confirm_enable")]
    Enable(InternalAutoConfirmEnableParams),
    /// Disable auto-confirm.
    #[serde(rename = "phantom_auto_confirm_disable")]
    Disable {},
    /// Query auto-confirm status.
    #[serde(rename = "phantom_auto_confirm_status")]
    Status {},
    /// Query supported chains.
    #[serde(rename = "phantom_auto_confirm_supported_chains")]
    SupportedChains {},
}

// ============================================================================
// CAIP transformation helpers
// ============================================================================

/// Convert a slice of `NetworkId` values to their internal CAIP representations.
///
/// This mirrors the TS pattern: `params.chains.map(networkIdToInternalCaip)`.
pub fn network_ids_to_internal_caips(
    network_ids: &[NetworkId],
) -> Result<Vec<InternalNetworkCaip>, Box<dyn std::error::Error + Send + Sync>> {
    network_ids
        .iter()
        .map(|id| network_id_to_internal_caip(*id).map_err(|e| Box::new(e) as _))
        .collect()
}

/// Convert a slice of `InternalNetworkCaip` values back to `NetworkId`.
///
/// This mirrors the TS pattern: `result.chains.map(internalCaipToNetworkId)`.
pub fn internal_caips_to_network_ids(
    caips: &[InternalNetworkCaip],
) -> Result<Vec<NetworkId>, Box<dyn std::error::Error + Send + Sync>> {
    caips
        .iter()
        .map(|caip| internal_caip_to_network_id(*caip).map_err(|e| Box::new(e) as _))
        .collect()
}

// ============================================================================
// PhantomProvider trait
// ============================================================================

/// Trait representing the low-level auto-confirm provider that communicates
/// with the extension using internal CAIP identifiers.
///
/// This mirrors the TS `PhantomProvider` interface with its typed `request`
/// method. Implementations send RPC requests to the extension and return
/// raw (internal CAIP) responses.
#[async_trait::async_trait]
pub trait PhantomProvider: Send + Sync {
    /// Send an auto-confirm RPC request and receive the state response.
    ///
    /// Used for enable, disable, and status requests that return
    /// `AutoConfirmStateResponse`.
    async fn request_state(
        &self,
        request: &AutoConfirmRequest,
    ) -> Result<AutoConfirmStateResponse, Box<dyn std::error::Error + Send + Sync>>;

    /// Send a supported-chains RPC request and receive the support response.
    ///
    /// Used for the supported_chains request that returns
    /// `AutoConfirmSupportResponse`.
    async fn request_supported_chains(
        &self,
        request: &AutoConfirmRequest,
    ) -> Result<AutoConfirmSupportResponse, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// Legacy provider trait (kept for backward compatibility)
// ============================================================================

/// Trait representing the auto-confirm provider (window.phantom.app in TS).
///
/// This is the higher-level provider that works with `NetworkId` directly.
/// For new implementations, prefer `PhantomProvider` which operates at the
/// internal CAIP level and lets `AutoConfirm` handle the transformations.
#[async_trait::async_trait]
pub trait AutoConfirmProvider: Send + Sync {
    /// Enable auto-confirm.
    async fn enable(
        &self,
        params: &AutoConfirmEnableParams,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Disable auto-confirm.
    async fn disable(&self) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get auto-confirm status.
    async fn status(&self) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>>;

    /// Get supported chains.
    async fn supported_chains(
        &self,
    ) -> Result<AutoConfirmSupportedChainsResult, Box<dyn std::error::Error + Send + Sync>>;
}

// ============================================================================
// AutoConfirm plugin struct
// ============================================================================

/// The provider backend for the `AutoConfirm` struct.
///
/// Supports either a legacy `AutoConfirmProvider` that already handles
/// CAIP transformations, or a `PhantomProvider` where `AutoConfirm`
/// performs the transformations itself.
enum ProviderBackend {
    /// Legacy provider that works directly with `NetworkId`.
    Legacy(Box<dyn AutoConfirmProvider>),
    /// Low-level provider that works with internal CAIP identifiers.
    Phantom(Box<dyn PhantomProvider>),
}

/// Auto-confirm plugin providing enable/disable/status/supportedChains.
///
/// When backed by a `PhantomProvider`, the `AutoConfirm` struct performs
/// CAIP transformations at the SDK level: it converts `NetworkId` values
/// to `InternalNetworkCaip` before sending requests to the provider, and
/// converts the internal CAIP values in responses back to `NetworkId`.
pub struct AutoConfirm {
    backend: ProviderBackend,
}

impl AutoConfirm {
    /// Create a new AutoConfirm with a legacy `AutoConfirmProvider`.
    pub fn new(provider: Box<dyn AutoConfirmProvider>) -> Self {
        Self {
            backend: ProviderBackend::Legacy(provider),
        }
    }

    /// Create a new AutoConfirm with a `PhantomProvider` that uses
    /// internal CAIP identifiers.
    ///
    /// The `AutoConfirm` struct will handle CAIP transformations
    /// automatically, converting `NetworkId` to/from `InternalNetworkCaip`.
    pub fn with_phantom_provider(provider: Box<dyn PhantomProvider>) -> Self {
        Self {
            backend: ProviderBackend::Phantom(provider),
        }
    }

    /// Enable auto-confirm.
    ///
    /// Transforms `NetworkId` chains to `InternalNetworkCaip` before sending
    /// to the provider, and transforms the response back.
    pub async fn enable(
        &self,
        params: Option<&AutoConfirmEnableParams>,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        let default_params = AutoConfirmEnableParams::default();
        let params = params.unwrap_or(&default_params);

        match &self.backend {
            ProviderBackend::Legacy(provider) => provider.enable(params).await,
            ProviderBackend::Phantom(provider) => {
                // Transform NetworkId -> InternalNetworkCaip for extension communication
                let internal_chains = match &params.chains {
                    Some(chains) => Some(network_ids_to_internal_caips(chains)?),
                    None => None,
                };

                let request = AutoConfirmRequest::Enable(InternalAutoConfirmEnableParams {
                    chains: internal_chains,
                });

                let response = provider.request_state(&request).await?;

                // Transform InternalNetworkCaip -> NetworkId for public interface
                Ok(AutoConfirmResult {
                    enabled: response.enabled,
                    chains: internal_caips_to_network_ids(&response.chains)?,
                })
            }
        }
    }

    /// Disable auto-confirm.
    ///
    /// Transforms the internal CAIP response back to `NetworkId`.
    pub async fn disable(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        match &self.backend {
            ProviderBackend::Legacy(provider) => provider.disable().await,
            ProviderBackend::Phantom(provider) => {
                let request = AutoConfirmRequest::Disable {};
                let response = provider.request_state(&request).await?;

                // Transform InternalNetworkCaip -> NetworkId for public interface
                Ok(AutoConfirmResult {
                    enabled: response.enabled,
                    chains: internal_caips_to_network_ids(&response.chains)?,
                })
            }
        }
    }

    /// Get auto-confirm status.
    ///
    /// Transforms the internal CAIP response back to `NetworkId`.
    pub async fn status(
        &self,
    ) -> Result<AutoConfirmResult, Box<dyn std::error::Error + Send + Sync>> {
        match &self.backend {
            ProviderBackend::Legacy(provider) => provider.status().await,
            ProviderBackend::Phantom(provider) => {
                let request = AutoConfirmRequest::Status {};
                let response = provider.request_state(&request).await?;

                // Transform InternalNetworkCaip -> NetworkId for public interface
                Ok(AutoConfirmResult {
                    enabled: response.enabled,
                    chains: internal_caips_to_network_ids(&response.chains)?,
                })
            }
        }
    }

    /// Get supported chains.
    ///
    /// Transforms the internal CAIP response back to `NetworkId`.
    pub async fn supported_chains(
        &self,
    ) -> Result<AutoConfirmSupportedChainsResult, Box<dyn std::error::Error + Send + Sync>> {
        match &self.backend {
            ProviderBackend::Legacy(provider) => provider.supported_chains().await,
            ProviderBackend::Phantom(provider) => {
                let request = AutoConfirmRequest::SupportedChains {};
                let response = provider.request_supported_chains(&request).await?;

                // Transform InternalNetworkCaip -> NetworkId for public interface
                Ok(AutoConfirmSupportedChainsResult {
                    chains: internal_caips_to_network_ids(&response.chains)?,
                })
            }
        }
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

/// Create an auto-confirm plugin backed by a `PhantomProvider`.
///
/// The `AutoConfirm` struct will handle CAIP transformations automatically,
/// converting `NetworkId` to/from `InternalNetworkCaip` at the SDK boundary.
pub fn create_auto_confirm_plugin_with_phantom_provider(
    provider: Box<dyn PhantomProvider>,
) -> Plugin {
    let auto_confirm = Arc::new(AutoConfirm::with_phantom_provider(provider));
    Plugin {
        name: "autoConfirm".to_string(),
        create: Box::new(move || Box::new(auto_confirm.clone())),
    }
}
