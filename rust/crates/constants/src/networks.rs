//! Network configuration and lookup functions for all supported blockchain networks.

use crate::network_ids::NetworkId;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Internal CAIP identifier used for extension communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InternalNetworkCaip {
    // BTC
    #[serde(rename = "bip122:000000000019d6689c085ae165831e93")]
    BitcoinMainnet,
    #[serde(rename = "bip122:000000000933ea01ad0ee984209779ba")]
    BitcoinTestnet,
    // Solana
    #[serde(rename = "solana:101")]
    Solana101,
    #[serde(rename = "solana:102")]
    Solana102,
    #[serde(rename = "solana:103")]
    Solana103,
    #[serde(rename = "solana:localnet")]
    SolanaLocalnet,
    // EVM
    #[serde(rename = "eip155:1")]
    Eip155_1,
    #[serde(rename = "eip155:11155111")]
    Eip155_11155111,
    #[serde(rename = "eip155:137")]
    Eip155_137,
    #[serde(rename = "eip155:80002")]
    Eip155_80002,
    #[serde(rename = "eip155:8453")]
    Eip155_8453,
    #[serde(rename = "eip155:84532")]
    Eip155_84532,
    #[serde(rename = "eip155:42161")]
    Eip155_42161,
    #[serde(rename = "eip155:421614")]
    Eip155_421614,
    #[serde(rename = "eip155:143")]
    Eip155_143,
    #[serde(rename = "eip155:10143")]
    Eip155_10143,
    // Hypercore
    #[serde(rename = "hypercore:mainnet")]
    HypercoreMainnet,
    #[serde(rename = "hypercore:testnet")]
    HypercoreTestnet,
    // Sui
    #[serde(rename = "sui:mainnet")]
    SuiMainnet,
    #[serde(rename = "sui:testnet")]
    SuiTestnet,
    #[serde(rename = "sui:devnet")]
    SuiDevnet,
}

/// Block explorer configuration for a network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerConfig {
    /// Name of the explorer service.
    pub name: String,
    /// URL template with `{hash}` placeholder for transaction lookups.
    pub transaction_url: String,
    /// URL template with `{address}` placeholder for address lookups.
    pub address_url: String,
}

/// Configuration for a blockchain network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    /// Human-readable network name.
    pub name: String,
    /// Chain family (e.g., "solana", "ethereum", "bitcoin").
    pub chain: String,
    /// Network name within the chain (e.g., "mainnet", "testnet").
    pub network: String,
    /// Internal CAIP identifier for extension communication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_caip: Option<InternalNetworkCaip>,
    /// EIP-155 chain ID (for EVM networks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,
    /// SLIP-44 coin type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slip44: Option<String>,
    /// Block explorer configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explorer: Option<ExplorerConfig>,
}

/// Errors that can occur during network operations.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// No internal CAIP mapping found for the given NetworkId.
    #[error("No internal CAIP mapping found for NetworkId: {0}")]
    NoInternalCaipMapping(NetworkId),
    /// No NetworkId mapping found for the given internal CAIP.
    #[error("No NetworkId mapping found for internal CAIP: {0:?}")]
    NoNetworkIdMapping(InternalNetworkCaip),
}

/// Static map of all network configurations indexed by NetworkId.
static NETWORK_CONFIGS: Lazy<HashMap<NetworkId, NetworkConfig>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Solana Networks
    m.insert(
        NetworkId::SolanaMainnet,
        NetworkConfig {
            name: "Solana Mainnet".into(),
            chain: "solana".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Solana101),
            chain_id: None,
            slip44: Some("501".into()),
            explorer: Some(ExplorerConfig {
                name: "Solscan".into(),
                transaction_url: "https://solscan.io/tx/{hash}".into(),
                address_url: "https://solscan.io/account/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::SolanaDevnet,
        NetworkConfig {
            name: "Solana Devnet".into(),
            chain: "solana".into(),
            network: "devnet".into(),
            internal_caip: Some(InternalNetworkCaip::Solana103),
            chain_id: None,
            slip44: Some("501".into()),
            explorer: Some(ExplorerConfig {
                name: "Solscan".into(),
                transaction_url: "https://solscan.io/tx/{hash}?cluster=devnet".into(),
                address_url: "https://solscan.io/account/{address}?cluster=devnet".into(),
            }),
        },
    );
    m.insert(
        NetworkId::SolanaTestnet,
        NetworkConfig {
            name: "Solana Testnet".into(),
            chain: "solana".into(),
            network: "testnet".into(),
            internal_caip: Some(InternalNetworkCaip::Solana102),
            chain_id: None,
            slip44: Some("501".into()),
            explorer: Some(ExplorerConfig {
                name: "Solscan".into(),
                transaction_url: "https://solscan.io/tx/{hash}?cluster=testnet".into(),
                address_url: "https://solscan.io/account/{address}?cluster=testnet".into(),
            }),
        },
    );

    // Ethereum Networks
    m.insert(
        NetworkId::EthereumMainnet,
        NetworkConfig {
            name: "Ethereum Mainnet".into(),
            chain: "ethereum".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_1),
            chain_id: Some(1),
            slip44: Some("60".into()),
            explorer: Some(ExplorerConfig {
                name: "Etherscan".into(),
                transaction_url: "https://etherscan.io/tx/{hash}".into(),
                address_url: "https://etherscan.io/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::EthereumSepolia,
        NetworkConfig {
            name: "Ethereum Sepolia".into(),
            chain: "ethereum".into(),
            network: "sepolia".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_11155111),
            chain_id: Some(11155111),
            slip44: Some("60".into()),
            explorer: Some(ExplorerConfig {
                name: "Etherscan".into(),
                transaction_url: "https://sepolia.etherscan.io/tx/{hash}".into(),
                address_url: "https://sepolia.etherscan.io/address/{address}".into(),
            }),
        },
    );

    // Polygon Networks
    m.insert(
        NetworkId::PolygonMainnet,
        NetworkConfig {
            name: "Polygon Mainnet".into(),
            chain: "polygon".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_137),
            chain_id: Some(137),
            slip44: Some("137".into()),
            explorer: Some(ExplorerConfig {
                name: "Polygonscan".into(),
                transaction_url: "https://polygonscan.com/tx/{hash}".into(),
                address_url: "https://polygonscan.com/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::PolygonAmoy,
        NetworkConfig {
            name: "Polygon Amoy".into(),
            chain: "polygon".into(),
            network: "amoy".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_80002),
            chain_id: Some(80002),
            slip44: Some("137".into()),
            explorer: Some(ExplorerConfig {
                name: "Polygonscan".into(),
                transaction_url: "https://amoy.polygonscan.com/tx/{hash}".into(),
                address_url: "https://amoy.polygonscan.com/address/{address}".into(),
            }),
        },
    );

    // Base Networks
    m.insert(
        NetworkId::BaseMainnet,
        NetworkConfig {
            name: "Base Mainnet".into(),
            chain: "base".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_8453),
            chain_id: Some(8453),
            slip44: Some("8453".into()),
            explorer: Some(ExplorerConfig {
                name: "Basescan".into(),
                transaction_url: "https://basescan.org/tx/{hash}".into(),
                address_url: "https://basescan.org/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::BaseSepolia,
        NetworkConfig {
            name: "Base Sepolia".into(),
            chain: "base".into(),
            network: "sepolia".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_84532),
            chain_id: Some(84532),
            slip44: Some("8453".into()),
            explorer: Some(ExplorerConfig {
                name: "Basescan".into(),
                transaction_url: "https://sepolia.basescan.org/tx/{hash}".into(),
                address_url: "https://sepolia.basescan.org/address/{address}".into(),
            }),
        },
    );

    // Arbitrum Networks
    m.insert(
        NetworkId::ArbitrumOne,
        NetworkConfig {
            name: "Arbitrum One".into(),
            chain: "arbitrum".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_42161),
            chain_id: Some(42161),
            slip44: Some("42161".into()),
            explorer: Some(ExplorerConfig {
                name: "Arbiscan".into(),
                transaction_url: "https://arbiscan.io/tx/{hash}".into(),
                address_url: "https://arbiscan.io/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::ArbitrumSepolia,
        NetworkConfig {
            name: "Arbitrum Sepolia".into(),
            chain: "arbitrum".into(),
            network: "sepolia".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_421614),
            chain_id: Some(421614),
            slip44: Some("42161".into()),
            explorer: Some(ExplorerConfig {
                name: "Arbiscan".into(),
                transaction_url: "https://sepolia.arbiscan.io/tx/{hash}".into(),
                address_url: "https://sepolia.arbiscan.io/address/{address}".into(),
            }),
        },
    );

    // Monad Networks
    m.insert(
        NetworkId::MonadMainnet,
        NetworkConfig {
            name: "Monad Mainnet".into(),
            chain: "monad".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_143),
            chain_id: Some(143),
            slip44: Some("60".into()), // Uses Ethereum SLIP-44
            explorer: Some(ExplorerConfig {
                name: "Monad Explorer".into(),
                transaction_url: "https://monadexplorer.com/tx/{hash}".into(),
                address_url: "https://monadexplorer.com/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::MonadTestnet,
        NetworkConfig {
            name: "Monad Testnet".into(),
            chain: "monad".into(),
            network: "testnet".into(),
            internal_caip: Some(InternalNetworkCaip::Eip155_10143),
            chain_id: Some(10143),
            slip44: Some("60".into()), // Uses Ethereum SLIP-44
            explorer: Some(ExplorerConfig {
                name: "Monad Testnet Explorer".into(),
                transaction_url: "https://testnet.monadexplorer.com/tx/{hash}".into(),
                address_url: "https://testnet.monadexplorer.com/address/{address}".into(),
            }),
        },
    );

    // Bitcoin Networks
    m.insert(
        NetworkId::BitcoinMainnet,
        NetworkConfig {
            name: "Bitcoin Mainnet".into(),
            chain: "bitcoin".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::BitcoinMainnet),
            chain_id: None,
            slip44: Some("0".into()),
            explorer: Some(ExplorerConfig {
                name: "Blockstream".into(),
                transaction_url: "https://blockstream.info/tx/{hash}".into(),
                address_url: "https://blockstream.info/address/{address}".into(),
            }),
        },
    );
    m.insert(
        NetworkId::BitcoinTestnet,
        NetworkConfig {
            name: "Bitcoin Testnet".into(),
            chain: "bitcoin".into(),
            network: "testnet".into(),
            internal_caip: Some(InternalNetworkCaip::BitcoinTestnet),
            chain_id: None,
            slip44: Some("0".into()),
            explorer: Some(ExplorerConfig {
                name: "Blockstream".into(),
                transaction_url: "https://blockstream.info/testnet/tx/{hash}".into(),
                address_url: "https://blockstream.info/testnet/address/{address}".into(),
            }),
        },
    );

    // Sui Networks
    m.insert(
        NetworkId::SuiMainnet,
        NetworkConfig {
            name: "Sui Mainnet".into(),
            chain: "sui".into(),
            network: "mainnet".into(),
            internal_caip: Some(InternalNetworkCaip::SuiMainnet),
            chain_id: None,
            slip44: Some("784".into()),
            explorer: Some(ExplorerConfig {
                name: "Sui Explorer".into(),
                transaction_url: "https://explorer.sui.io/txblock/{hash}?network=mainnet".into(),
                address_url: "https://explorer.sui.io/address/{address}?network=mainnet".into(),
            }),
        },
    );
    m.insert(
        NetworkId::SuiTestnet,
        NetworkConfig {
            name: "Sui Testnet".into(),
            chain: "sui".into(),
            network: "testnet".into(),
            internal_caip: Some(InternalNetworkCaip::SuiTestnet),
            chain_id: None,
            slip44: Some("784".into()),
            explorer: Some(ExplorerConfig {
                name: "Sui Explorer".into(),
                transaction_url: "https://explorer.sui.io/txblock/{hash}?network=testnet".into(),
                address_url: "https://explorer.sui.io/address/{address}?network=testnet".into(),
            }),
        },
    );
    m.insert(
        NetworkId::SuiDevnet,
        NetworkConfig {
            name: "Sui Devnet".into(),
            chain: "sui".into(),
            network: "devnet".into(),
            internal_caip: Some(InternalNetworkCaip::SuiDevnet),
            chain_id: None,
            slip44: Some("784".into()),
            explorer: Some(ExplorerConfig {
                name: "Sui Explorer".into(),
                transaction_url: "https://explorer.sui.io/txblock/{hash}?network=devnet".into(),
                address_url: "https://explorer.sui.io/address/{address}?network=devnet".into(),
            }),
        },
    );

    m
});

/// Type of explorer URL to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerUrlType {
    /// Transaction explorer URL.
    Transaction,
    /// Address explorer URL.
    Address,
}

/// Get the network configuration for a given network ID.
pub fn get_network_config(network_id: NetworkId) -> Option<&'static NetworkConfig> {
    NETWORK_CONFIGS.get(&network_id)
}

/// Get a block explorer URL for a transaction or address on a given network.
pub fn get_explorer_url(
    network_id: NetworkId,
    url_type: ExplorerUrlType,
    value: &str,
) -> Option<String> {
    let config = get_network_config(network_id)?;
    let explorer = config.explorer.as_ref()?;

    let (template, placeholder) = match url_type {
        ExplorerUrlType::Transaction => (&explorer.transaction_url, "{hash}"),
        ExplorerUrlType::Address => (&explorer.address_url, "{address}"),
    };

    Some(template.replace(placeholder, value))
}

/// Get all supported network IDs.
pub fn get_supported_networks() -> Vec<NetworkId> {
    NETWORK_CONFIGS.keys().copied().collect()
}

/// Get all network IDs for a given chain family (e.g., "solana", "ethereum").
pub fn get_networks_by_chain(chain: &str) -> Vec<NetworkId> {
    NETWORK_CONFIGS
        .iter()
        .filter(|(_, config)| config.chain == chain)
        .map(|(id, _)| *id)
        .collect()
}

/// Convert an EIP-155 chain ID to its corresponding NetworkId.
pub fn chain_id_to_network_id(chain_id: u64) -> Option<NetworkId> {
    NETWORK_CONFIGS
        .iter()
        .find(|(_, config)| config.chain_id == Some(chain_id))
        .map(|(id, _)| *id)
}

/// Extract the EIP-155 chain ID from a NetworkId (for EVM networks).
pub fn network_id_to_chain_id(network_id: NetworkId) -> Option<u64> {
    get_network_config(network_id).and_then(|c| c.chain_id)
}

/// Convert a NetworkId to its InternalNetworkCaip for extension communication.
pub fn network_id_to_internal_caip(
    network_id: NetworkId,
) -> Result<InternalNetworkCaip, NetworkError> {
    get_network_config(network_id)
        .and_then(|c| c.internal_caip)
        .ok_or(NetworkError::NoInternalCaipMapping(network_id))
}

/// Convert an InternalNetworkCaip back to a NetworkId from extension responses.
pub fn internal_caip_to_network_id(
    internal_caip: InternalNetworkCaip,
) -> Result<NetworkId, NetworkError> {
    NETWORK_CONFIGS
        .iter()
        .find(|(_, config)| config.internal_caip == Some(internal_caip))
        .map(|(id, _)| *id)
        .ok_or(NetworkError::NoNetworkIdMapping(internal_caip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solana_mainnet_config() {
        let config = get_network_config(NetworkId::SolanaMainnet).unwrap();
        assert_eq!(config.chain, "solana");
        assert_eq!(config.network, "mainnet");
        assert_eq!(config.name, "Solana Mainnet");
    }

    #[test]
    fn ethereum_mainnet_config() {
        let config = get_network_config(NetworkId::EthereumMainnet).unwrap();
        assert_eq!(config.chain, "ethereum");
        assert_eq!(config.network, "mainnet");
    }

    #[test]
    fn all_networks_have_explorers() {
        for (_, config) in NETWORK_CONFIGS.iter() {
            let explorer = config.explorer.as_ref().expect(&format!(
                "Network {} should have explorer",
                config.name
            ));
            assert!(
                explorer.transaction_url.contains("{hash}"),
                "transaction_url for {} should contain {{hash}}",
                config.name
            );
            assert!(
                explorer.address_url.contains("{address}"),
                "address_url for {} should contain {{address}}",
                config.name
            );
        }
    }

    #[test]
    fn get_explorer_url_transaction() {
        let url = get_explorer_url(
            NetworkId::SolanaMainnet,
            ExplorerUrlType::Transaction,
            "test-hash",
        );
        assert_eq!(url, Some("https://solscan.io/tx/test-hash".to_string()));
    }

    #[test]
    fn get_explorer_url_address() {
        let url = get_explorer_url(
            NetworkId::EthereumMainnet,
            ExplorerUrlType::Address,
            "0x123456",
        );
        assert_eq!(
            url,
            Some("https://etherscan.io/address/0x123456".to_string())
        );
    }

    #[test]
    fn get_supported_networks_not_empty() {
        let networks = get_supported_networks();
        assert!(!networks.is_empty());
        assert!(networks.contains(&NetworkId::SolanaMainnet));
        assert!(networks.contains(&NetworkId::EthereumMainnet));
    }

    #[test]
    fn get_networks_by_chain_solana() {
        let solana = get_networks_by_chain("solana");
        assert!(solana.contains(&NetworkId::SolanaMainnet));
        assert!(solana.contains(&NetworkId::SolanaDevnet));
        // Verify all returned are actually solana
        for id in &solana {
            let config = get_network_config(*id).unwrap();
            assert_eq!(config.chain, "solana");
        }
    }

    #[test]
    fn get_networks_by_chain_ethereum() {
        let eth = get_networks_by_chain("ethereum");
        assert!(eth.contains(&NetworkId::EthereumMainnet));
        assert!(eth.contains(&NetworkId::EthereumSepolia));
    }

    #[test]
    fn get_networks_by_chain_unsupported() {
        let networks = get_networks_by_chain("unsupported-chain");
        assert!(networks.is_empty());
    }

    #[test]
    fn chain_id_roundtrip() {
        assert_eq!(
            chain_id_to_network_id(1),
            Some(NetworkId::EthereumMainnet)
        );
        assert_eq!(
            network_id_to_chain_id(NetworkId::EthereumMainnet),
            Some(1)
        );
    }

    #[test]
    fn solana_has_no_chain_id() {
        assert_eq!(
            network_id_to_chain_id(NetworkId::SolanaMainnet),
            None
        );
    }
}
