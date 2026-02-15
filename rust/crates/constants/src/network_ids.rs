//! User-friendly enum for CAIP-2 network identifiers.
//! Use these constants instead of hardcoding network IDs.

use serde::{Deserialize, Serialize};
use std::fmt;

/// CAIP-2 network identifier enum covering all supported blockchain networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkId {
    // Solana Networks
    /// Solana Mainnet Beta
    #[serde(rename = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp")]
    SolanaMainnet,
    /// Solana Devnet
    #[serde(rename = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1")]
    SolanaDevnet,
    /// Solana Testnet
    #[serde(rename = "solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z")]
    SolanaTestnet,

    // Ethereum Networks
    /// Ethereum Mainnet
    #[serde(rename = "eip155:1")]
    EthereumMainnet,
    /// Ethereum Sepolia Testnet
    #[serde(rename = "eip155:11155111")]
    EthereumSepolia,

    // Polygon Networks
    /// Polygon Mainnet
    #[serde(rename = "eip155:137")]
    PolygonMainnet,
    /// Polygon Amoy Testnet
    #[serde(rename = "eip155:80002")]
    PolygonAmoy,

    // Base Networks
    /// Base Mainnet
    #[serde(rename = "eip155:8453")]
    BaseMainnet,
    /// Base Sepolia Testnet
    #[serde(rename = "eip155:84532")]
    BaseSepolia,

    // Arbitrum Networks
    /// Arbitrum One
    #[serde(rename = "eip155:42161")]
    ArbitrumOne,
    /// Arbitrum Sepolia Testnet
    #[serde(rename = "eip155:421614")]
    ArbitrumSepolia,

    // Monad Networks
    /// Monad Mainnet
    #[serde(rename = "eip155:143")]
    MonadMainnet,
    /// Monad Testnet
    #[serde(rename = "eip155:10143")]
    MonadTestnet,

    // Bitcoin Networks (for future support)
    /// Bitcoin Mainnet
    #[serde(rename = "bip122:000000000019d6689c085ae165831e93")]
    BitcoinMainnet,
    /// Bitcoin Testnet
    #[serde(rename = "bip122:000000000933ea01ad0ee984209779ba")]
    BitcoinTestnet,

    // Sui Networks (for future support)
    /// Sui Mainnet
    #[serde(rename = "sui:35834a8a")]
    SuiMainnet,
    /// Sui Testnet
    #[serde(rename = "sui:4c78adac")]
    SuiTestnet,
    /// Sui Devnet
    #[serde(rename = "sui:devnet")]
    SuiDevnet,
}

impl NetworkId {
    /// Returns the CAIP-2 string representation of this network ID.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SolanaMainnet => "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
            Self::SolanaDevnet => "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
            Self::SolanaTestnet => "solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z",
            Self::EthereumMainnet => "eip155:1",
            Self::EthereumSepolia => "eip155:11155111",
            Self::PolygonMainnet => "eip155:137",
            Self::PolygonAmoy => "eip155:80002",
            Self::BaseMainnet => "eip155:8453",
            Self::BaseSepolia => "eip155:84532",
            Self::ArbitrumOne => "eip155:42161",
            Self::ArbitrumSepolia => "eip155:421614",
            Self::MonadMainnet => "eip155:143",
            Self::MonadTestnet => "eip155:10143",
            Self::BitcoinMainnet => "bip122:000000000019d6689c085ae165831e93",
            Self::BitcoinTestnet => "bip122:000000000933ea01ad0ee984209779ba",
            Self::SuiMainnet => "sui:35834a8a",
            Self::SuiTestnet => "sui:4c78adac",
            Self::SuiDevnet => "sui:devnet",
        }
    }

    /// Returns all supported network IDs.
    pub fn all() -> &'static [NetworkId] {
        &[
            Self::SolanaMainnet,
            Self::SolanaDevnet,
            Self::SolanaTestnet,
            Self::EthereumMainnet,
            Self::EthereumSepolia,
            Self::PolygonMainnet,
            Self::PolygonAmoy,
            Self::BaseMainnet,
            Self::BaseSepolia,
            Self::ArbitrumOne,
            Self::ArbitrumSepolia,
            Self::MonadMainnet,
            Self::MonadTestnet,
            Self::BitcoinMainnet,
            Self::BitcoinTestnet,
            Self::SuiMainnet,
            Self::SuiTestnet,
            Self::SuiDevnet,
        ]
    }
}

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
