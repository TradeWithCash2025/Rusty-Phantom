//! Network utility functions for working with blockchain network identifiers.

/// Extract the chain prefix from a CAIP-2 network identifier.
///
/// # Examples
/// ```
/// use phantom_utils::network::get_chain_prefix;
/// assert_eq!(get_chain_prefix("eip155:1"), "eip155");
/// assert_eq!(get_chain_prefix("solana:101"), "solana");
/// ```
pub fn get_chain_prefix(network_id: &str) -> &str {
    network_id
        .split(':')
        .next()
        .unwrap_or(network_id)
}

/// Check if a network identifier is for an Ethereum/EVM chain.
///
/// Returns `true` if the network ID starts with `eip155:`.
pub fn is_ethereum_chain(network_id: &str) -> bool {
    get_chain_prefix(network_id).eq_ignore_ascii_case("eip155")
}

/// Check if a network identifier is for a Solana chain.
///
/// Returns `true` if the network ID starts with `solana:`.
pub fn is_solana_chain(network_id: &str) -> bool {
    get_chain_prefix(network_id).eq_ignore_ascii_case("solana")
}
