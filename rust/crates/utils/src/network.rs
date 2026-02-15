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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_ethereum_chain_eip155() {
        assert!(is_ethereum_chain("eip155:1"));
        assert!(is_ethereum_chain("eip155:137"));
        assert!(is_ethereum_chain("eip155:42161"));
        assert!(is_ethereum_chain("eip155:8453"));
    }

    #[test]
    fn is_ethereum_chain_case_insensitive() {
        assert!(is_ethereum_chain("EIP155:1"));
        assert!(is_ethereum_chain("Eip155:1"));
    }

    #[test]
    fn is_ethereum_chain_false_for_other_chains() {
        assert!(!is_ethereum_chain("solana:mainnet"));
        assert!(!is_ethereum_chain("solana:devnet"));
        assert!(!is_ethereum_chain("bitcoin:mainnet"));
        assert!(!is_ethereum_chain("sui:mainnet"));
    }

    #[test]
    fn get_chain_prefix_extracts_correctly() {
        assert_eq!(get_chain_prefix("eip155:1"), "eip155");
        assert_eq!(get_chain_prefix("solana:mainnet"), "solana");
        assert_eq!(get_chain_prefix("bitcoin:mainnet"), "bitcoin");
        assert_eq!(get_chain_prefix("sui:mainnet"), "sui");
    }

    #[test]
    fn get_chain_prefix_no_colon() {
        assert_eq!(get_chain_prefix("eip155"), "eip155");
    }

    #[test]
    fn is_solana_chain_true() {
        assert!(is_solana_chain("solana:mainnet"));
        assert!(is_solana_chain("solana:devnet"));
    }

    #[test]
    fn is_solana_chain_false() {
        assert!(!is_solana_chain("eip155:1"));
        assert!(!is_solana_chain("bitcoin:mainnet"));
    }
}
