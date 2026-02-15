//! Client-specific constants including derivation paths and network configuration.
//!
//! Mirrors the TypeScript constants from `packages/client/src/constants.ts`.

use serde::{Deserialize, Serialize};

/// Cryptographic curve used for key derivation.
///
/// Corresponds to `DerivationInfoCurveEnum` from the OpenAPI spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Curve {
    /// Ed25519 elliptic curve.
    Ed25519,
    /// Secp256k1 elliptic curve.
    Secp256k1,
}

/// Cryptographic algorithm for signing.
///
/// Corresponds to `Algorithm` from the OpenAPI spec.
/// Extended from the constants crate's `Algorithm` to include secp256k1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientAlgorithm {
    /// Ed25519 signing algorithm.
    Ed25519,
    /// Secp256k1 signing algorithm.
    Secp256k1,
}

/// Address format for derived accounts.
///
/// Corresponds to `DerivationInfoAddressFormatEnum` from the OpenAPI spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AddressFormat {
    /// Solana address format.
    Solana,
    /// Ethereum address format.
    Ethereum,
    /// Sui address format.
    Sui,
    /// Bitcoin SegWit address format.
    BitcoinSegwit,
}

/// Default derivation paths for different blockchain networks.
pub struct DerivationPath;

impl DerivationPath {
    /// Solana - BIP44 standard for Solana (coin type 501).
    pub fn solana(account_index: u32) -> String {
        format!("m/44'/501'/{account_index}'/0'")
    }

    /// Ethereum - BIP44 standard for Ethereum and all EVM-compatible chains (coin type 60).
    pub fn ethereum(account_index: u32) -> String {
        format!("m/44'/60'/0'/0/{account_index}")
    }

    /// Bitcoin - BIP84 standard for Bitcoin (coin type 0).
    pub fn bitcoin(account_index: u32) -> String {
        format!("m/84'/0'/{account_index}'/0")
    }

    /// Sui - BIP44 standard for Sui (coin type 784).
    pub fn sui(account_index: u32) -> String {
        format!("m/44'/784'/{account_index}'/0'/0'")
    }
}

/// Client-specific network configuration.
///
/// Includes derivation path, curve, algorithm, and address format
/// for a given network and account index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientNetworkConfig {
    /// BIP-44/84 derivation path.
    pub derivation_path: String,
    /// Elliptic curve for key derivation.
    pub curve: Curve,
    /// Signing algorithm.
    pub algorithm: ClientAlgorithm,
    /// Address format for derived accounts.
    pub address_format: AddressFormat,
}

/// Get derivation path based on network ID.
///
/// Extracts the chain name from the network ID and returns the appropriate
/// derivation path for the given account index.
pub fn get_derivation_path_for_network(network_id: &str, account_index: u32) -> String {
    let network = network_id
        .split(':')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match network.as_str() {
        "solana" => DerivationPath::solana(account_index),
        "sui" => DerivationPath::sui(account_index),
        "bitcoin" | "btc" | "bip122" => DerivationPath::bitcoin(account_index),
        // Default to Ethereum path for all EVM-compatible chains
        _ => DerivationPath::ethereum(account_index),
    }
}

/// Get network configuration with derivation index.
///
/// Deprecated: use [`get_client_network_config()`] instead.
#[deprecated(note = "Use get_client_network_config instead")]
pub fn get_network_config_with_index(
    network_id: &str,
    derivation_index: u32,
) -> Option<ClientNetworkConfig> {
    get_client_network_config(network_id, derivation_index)
}

/// Get complete network configuration for a given network ID and account index.
///
/// Returns `None` if the network is not supported.
pub fn get_client_network_config(
    network_id: &str,
    account_index: u32,
) -> Option<ClientNetworkConfig> {
    let network = network_id
        .split(':')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match network.as_str() {
        "solana" => Some(ClientNetworkConfig {
            derivation_path: DerivationPath::solana(account_index),
            curve: Curve::Ed25519,
            algorithm: ClientAlgorithm::Ed25519,
            address_format: AddressFormat::Solana,
        }),
        "sui" => Some(ClientNetworkConfig {
            derivation_path: DerivationPath::sui(account_index),
            curve: Curve::Ed25519,
            algorithm: ClientAlgorithm::Ed25519,
            address_format: AddressFormat::Sui,
        }),
        "bitcoin" | "btc" | "bip122" => Some(ClientNetworkConfig {
            derivation_path: DerivationPath::bitcoin(account_index),
            curve: Curve::Secp256k1,
            algorithm: ClientAlgorithm::Secp256k1,
            address_format: AddressFormat::BitcoinSegwit,
        }),
        "eip155" => Some(ClientNetworkConfig {
            derivation_path: DerivationPath::ethereum(account_index),
            curve: Curve::Secp256k1,
            algorithm: ClientAlgorithm::Secp256k1,
            address_format: AddressFormat::Ethereum,
        }),
        _ => None,
    }
}
