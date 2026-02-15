//! Constants and configuration for the Phantom Connect SDK.
//!
//! This crate provides all shared constants, network configurations,
//! and type definitions used throughout the Phantom SDK ecosystem.

pub mod analytics;
pub mod authenticators;
pub mod environments;
pub mod icons;
pub mod network_ids;
pub mod networks;
pub mod provider_names;

// Re-export commonly used types at crate root
pub use authenticators::{Algorithm, DEFAULT_AUTHENTICATOR_ALGORITHM};
pub use environments::{DEFAULT_AUTH_URL, DEFAULT_EMBEDDED_WALLET_TYPE, DEFAULT_WALLET_API_URL};
pub use icons::PHANTOM_ICON;
pub use network_ids::NetworkId;
pub use networks::{
    chain_id_to_network_id, get_explorer_url, get_network_config, get_networks_by_chain,
    get_supported_networks, internal_caip_to_network_id, network_id_to_chain_id,
    network_id_to_internal_caip, ExplorerConfig, ExplorerUrlType, InternalNetworkCaip,
    NetworkConfig, NetworkError,
};
pub use provider_names::{get_provider_name, ProviderNameKey};
