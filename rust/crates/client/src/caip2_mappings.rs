//! CAIP-2 Network ID to submission configuration mappings.
//!
//! Maps CAIP-2 network identifiers to chain/network pairs used
//! for transaction submission and spending limit derivation.

use once_cell::sync::Lazy;
use phantom_constants::NetworkId;
use std::collections::HashMap;

use crate::types::SubmissionConfig;

/// Network mapping entry with chain, network, and optional description.
struct NetworkMapping {
    chain: &'static str,
    network: &'static str,
    #[allow(dead_code)]
    description: &'static str,
}

/// Lazy-initialized map of CAIP-2 network IDs to network mappings.
static CAIP2_NETWORK_MAPPINGS: Lazy<HashMap<&'static str, NetworkMapping>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Solana networks
    m.insert(
        NetworkId::SolanaMainnet.as_str(),
        NetworkMapping {
            chain: "solana",
            network: "mainnet",
            description: "Solana Mainnet-Beta",
        },
    );
    m.insert(
        NetworkId::SolanaDevnet.as_str(),
        NetworkMapping {
            chain: "solana",
            network: "devnet",
            description: "Solana Devnet",
        },
    );
    m.insert(
        NetworkId::SolanaTestnet.as_str(),
        NetworkMapping {
            chain: "solana",
            network: "testnet",
            description: "Solana Testnet",
        },
    );

    // Ethereum/EVM networks
    m.insert(
        NetworkId::EthereumMainnet.as_str(),
        NetworkMapping {
            chain: "ethereum",
            network: "mainnet",
            description: "Ethereum Mainnet",
        },
    );
    m.insert(
        NetworkId::EthereumSepolia.as_str(),
        NetworkMapping {
            chain: "ethereum",
            network: "sepolia",
            description: "Sepolia Testnet",
        },
    );
    m.insert(
        NetworkId::PolygonMainnet.as_str(),
        NetworkMapping {
            chain: "polygon",
            network: "mainnet",
            description: "Polygon Mainnet",
        },
    );
    m.insert(
        NetworkId::PolygonAmoy.as_str(),
        NetworkMapping {
            chain: "polygon",
            network: "amoy",
            description: "Polygon Amoy Testnet",
        },
    );
    m.insert(
        NetworkId::BaseMainnet.as_str(),
        NetworkMapping {
            chain: "base",
            network: "mainnet",
            description: "Base Mainnet",
        },
    );
    m.insert(
        NetworkId::BaseSepolia.as_str(),
        NetworkMapping {
            chain: "base",
            network: "sepolia",
            description: "Base Sepolia Testnet",
        },
    );
    m.insert(
        NetworkId::ArbitrumOne.as_str(),
        NetworkMapping {
            chain: "arbitrum",
            network: "mainnet",
            description: "Arbitrum One",
        },
    );
    m.insert(
        NetworkId::ArbitrumSepolia.as_str(),
        NetworkMapping {
            chain: "arbitrum",
            network: "sepolia",
            description: "Arbitrum Sepolia Testnet",
        },
    );
    m.insert(
        NetworkId::MonadMainnet.as_str(),
        NetworkMapping {
            chain: "monad",
            network: "mainnet",
            description: "Monad Mainnet",
        },
    );
    m.insert(
        NetworkId::MonadTestnet.as_str(),
        NetworkMapping {
            chain: "monad",
            network: "testnet",
            description: "Monad Testnet",
        },
    );

    // Bitcoin networks
    m.insert(
        NetworkId::BitcoinMainnet.as_str(),
        NetworkMapping {
            chain: "bitcoin",
            network: "mainnet",
            description: "Bitcoin Mainnet",
        },
    );
    m.insert(
        NetworkId::BitcoinTestnet.as_str(),
        NetworkMapping {
            chain: "bitcoin",
            network: "testnet",
            description: "Bitcoin Testnet",
        },
    );

    // Sui networks
    m.insert(
        NetworkId::SuiMainnet.as_str(),
        NetworkMapping {
            chain: "sui",
            network: "mainnet",
            description: "Sui Mainnet",
        },
    );
    m.insert(
        NetworkId::SuiTestnet.as_str(),
        NetworkMapping {
            chain: "sui",
            network: "testnet",
            description: "Sui Testnet",
        },
    );

    m
});

/// Derive SubmissionConfig from a CAIP-2 network ID.
///
/// Returns `None` if the network is not found in the mappings.
pub fn derive_submission_config(network_id: &str) -> Option<SubmissionConfig> {
    CAIP2_NETWORK_MAPPINGS
        .get(network_id)
        .map(|mapping| SubmissionConfig {
            chain: mapping.chain.to_string(),
            network: mapping.network.to_string(),
        })
}

/// Check if a network ID supports transaction submission.
pub fn supports_transaction_submission(network_id: &str) -> bool {
    CAIP2_NETWORK_MAPPINGS.contains_key(network_id)
}

/// Get network description for a CAIP-2 network ID.
pub fn get_network_description(network_id: &str) -> Option<&'static str> {
    CAIP2_NETWORK_MAPPINGS
        .get(network_id)
        .map(|mapping| mapping.description)
}

/// List all supported CAIP-2 network IDs.
pub fn get_supported_network_ids() -> Vec<&'static str> {
    CAIP2_NETWORK_MAPPINGS.keys().copied().collect()
}

/// Get all network IDs for a specific chain.
pub fn get_network_ids_by_chain(chain: &str) -> Vec<&'static str> {
    CAIP2_NETWORK_MAPPINGS
        .iter()
        .filter(|(_, mapping)| mapping.chain.eq_ignore_ascii_case(chain))
        .map(|(network_id, _)| *network_id)
        .collect()
}
