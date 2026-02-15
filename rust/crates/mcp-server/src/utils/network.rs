//! Network ID normalization functions for Solana chains and swapper API.

use phantom_constants::NetworkId;

/// Normalizes user-friendly network IDs to canonical CAIP-2 format.
///
/// Converts short forms like "solana:mainnet" to the full CAIP-2 chain ID format.
///
/// # Examples
/// - "solana:mainnet" → "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
/// - "solana:devnet" → "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
/// - "ethereum:1" → "ethereum:1" (no mapping, returned as-is)
pub fn normalize_network_id(network_id: &str) -> String {
    let normalized = network_id.to_lowercase();

    match normalized.as_str() {
        "solana:mainnet" | "solana:mainnet-beta" => NetworkId::SolanaMainnet.to_string(),
        "solana:devnet" => NetworkId::SolanaDevnet.to_string(),
        "solana:testnet" => NetworkId::SolanaTestnet.to_string(),
        _ => network_id.to_string(),
    }
}

/// Normalizes network IDs to the chain ID format expected by Phantom's swapper API.
///
/// Converts various Solana network identifier formats to the numeric chain ID format
/// used by the quotes API:
/// - "solana:101" for mainnet
/// - "solana:103" for devnet
/// - "solana:102" for testnet
pub fn normalize_swapper_chain_id(network_id: &str) -> String {
    let normalized = network_id.to_lowercase();

    match normalized.as_str() {
        "solana:mainnet" | "solana:mainnet-beta" | "solana:101" => "solana:101".to_string(),
        s if s == NetworkId::SolanaMainnet.as_str().to_lowercase() => "solana:101".to_string(),
        "solana:devnet" | "solana:103" => "solana:103".to_string(),
        s if s == NetworkId::SolanaDevnet.as_str().to_lowercase() => "solana:103".to_string(),
        "solana:testnet" | "solana:102" => "solana:102".to_string(),
        s if s == NetworkId::SolanaTestnet.as_str().to_lowercase() => "solana:102".to_string(),
        _ => network_id.to_string(),
    }
}
