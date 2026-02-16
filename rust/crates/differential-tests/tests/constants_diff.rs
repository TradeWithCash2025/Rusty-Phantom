//! Differential tests: constants (TS) vs phantom-constants (Rust).

use phantom_constants::{
    chain_id_to_network_id, get_explorer_url, get_network_config, get_networks_by_chain,
    get_provider_name, get_supported_networks, network_id_to_chain_id, ExplorerUrlType, NetworkId,
};
use phantom_differential_tests::compare::{assert_diff_match, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use serde_json::json;

/// All NetworkId CAIP-2 strings, matching the TS enum values exactly.
const ALL_NETWORK_IDS: &[(&str, NetworkId)] = &[
    ("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp", NetworkId::SolanaMainnet),
    ("solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1", NetworkId::SolanaDevnet),
    ("solana:4uhcVJyU9pJkvQyS88uRDiswHXSCkY3z", NetworkId::SolanaTestnet),
    ("eip155:1", NetworkId::EthereumMainnet),
    ("eip155:11155111", NetworkId::EthereumSepolia),
    ("eip155:137", NetworkId::PolygonMainnet),
    ("eip155:80002", NetworkId::PolygonAmoy),
    ("eip155:8453", NetworkId::BaseMainnet),
    ("eip155:84532", NetworkId::BaseSepolia),
    ("eip155:42161", NetworkId::ArbitrumOne),
    ("eip155:421614", NetworkId::ArbitrumSepolia),
    ("eip155:143", NetworkId::MonadMainnet),
    ("eip155:10143", NetworkId::MonadTestnet),
    ("bip122:000000000019d6689c085ae165831e93", NetworkId::BitcoinMainnet),
    ("bip122:000000000933ea01ad0ee984209779ba", NetworkId::BitcoinTestnet),
    ("sui:35834a8a", NetworkId::SuiMainnet),
    ("sui:4c78adac", NetworkId::SuiTestnet),
    ("sui:devnet", NetworkId::SuiDevnet),
];

// --- getSupportedNetworks ---

#[test]
fn get_supported_networks_matches_ts() {
    let rust_networks = get_supported_networks();
    let ts_result = oracle_call("constants.getSupportedNetworks", &json!([])).unwrap_ok();

    // Convert Rust to sorted string list
    let mut rust_strs: Vec<String> = rust_networks.iter().map(|n| n.as_str().to_string()).collect();
    rust_strs.sort();

    // TS returns array of strings
    let mut ts_strs: Vec<String> = ts_result
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    ts_strs.sort();

    assert_eq!(rust_strs, ts_strs, "getSupportedNetworks mismatch");
}

// --- getNetworkConfig ---

#[test]
fn get_network_config_matches_ts_for_all_networks() {
    for (caip_str, network_id) in ALL_NETWORK_IDS {
        let rust_config = get_network_config(*network_id);
        let ts_result = oracle_call("constants.getNetworkConfig", &json!([caip_str])).unwrap_ok();

        if let Some(config) = rust_config {
            // Serialize Rust config to JSON for comparison
            let rust_json = serde_json::to_value(config).unwrap();
            assert_diff_match(
                &CompareMode::CanonicalizeSortedKeys,
                "constants.getNetworkConfig",
                &json!([caip_str]),
                &ts_result,
                &rust_json,
            );
        } else {
            assert!(
                ts_result.is_null(),
                "TS returned config for {caip_str} but Rust returned None"
            );
        }
    }
}

// --- getExplorerUrl ---

#[test]
fn get_explorer_url_matches_ts() {
    let test_value = "abc123testhash";
    for (caip_str, network_id) in ALL_NETWORK_IDS {
        for (ts_type, rust_type) in &[
            ("transaction", ExplorerUrlType::Transaction),
            ("address", ExplorerUrlType::Address),
        ] {
            let rust_url = get_explorer_url(*network_id, *rust_type, test_value);
            let ts_result =
                oracle_call("constants.getExplorerUrl", &json!([caip_str, ts_type, test_value]))
                    .unwrap_ok();

            match rust_url {
                Some(url) => {
                    assert_diff_match(
                        &CompareMode::Exact,
                        "constants.getExplorerUrl",
                        &json!([caip_str, ts_type, test_value]),
                        &ts_result,
                        &json!(url),
                    );
                }
                None => {
                    assert!(
                        ts_result.is_null(),
                        "TS returned explorer URL for {caip_str}/{ts_type} but Rust returned None"
                    );
                }
            }
        }
    }
}

// --- getNetworksByChain ---

#[test]
fn get_networks_by_chain_matches_ts() {
    let chains = [
        "solana",
        "ethereum",
        "polygon",
        "base",
        "arbitrum",
        "monad",
        "bitcoin",
        "sui",
        "nonexistent",
    ];

    for chain in &chains {
        let rust_networks = get_networks_by_chain(chain);
        let ts_result = oracle_call("constants.getNetworksByChain", &json!([chain])).unwrap_ok();

        let mut rust_strs: Vec<String> =
            rust_networks.iter().map(|n| n.as_str().to_string()).collect();
        rust_strs.sort();

        let mut ts_strs: Vec<String> = ts_result
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        ts_strs.sort();

        assert_eq!(
            rust_strs, ts_strs,
            "getNetworksByChain({chain}) mismatch"
        );
    }
}

// --- chainIdToNetworkId ---

#[test]
fn chain_id_to_network_id_matches_ts() {
    let chain_ids: &[u64] = &[1, 11155111, 137, 80002, 8453, 84532, 42161, 421614, 143, 10143, 999999];

    for &chain_id in chain_ids {
        let rust_result = chain_id_to_network_id(chain_id);
        let ts_result =
            oracle_call("constants.chainIdToNetworkId", &json!([chain_id])).unwrap_ok();

        match rust_result {
            Some(nid) => {
                assert_diff_match(
                    &CompareMode::Exact,
                    "constants.chainIdToNetworkId",
                    &json!([chain_id]),
                    &ts_result,
                    &json!(nid.as_str()),
                );
            }
            None => {
                assert!(
                    ts_result.is_null(),
                    "TS returned network ID for chain_id={chain_id} but Rust returned None"
                );
            }
        }
    }
}

// --- networkIdToChainId ---

#[test]
fn network_id_to_chain_id_matches_ts() {
    for (caip_str, network_id) in ALL_NETWORK_IDS {
        let rust_result = network_id_to_chain_id(*network_id);
        let ts_result =
            oracle_call("constants.networkIdToChainId", &json!([caip_str])).unwrap_ok();

        match rust_result {
            Some(chain_id) => {
                assert_diff_match(
                    &CompareMode::Exact,
                    "constants.networkIdToChainId",
                    &json!([caip_str]),
                    &ts_result,
                    &json!(chain_id),
                );
            }
            None => {
                assert!(
                    ts_result.is_null(),
                    "TS returned chain ID for {caip_str} but Rust returned None"
                );
            }
        }
    }
}

// --- getProviderName ---

#[test]
fn get_provider_name_matches_ts() {
    let providers = [
        "google", "apple", "phantom", "device", "injected", "deeplink",
        "unknown_provider", "random_string", "",
    ];

    for provider in &providers {
        let rust_result = get_provider_name(provider);
        let ts_result =
            oracle_call("constants.getProviderName", &json!([provider])).unwrap_ok();

        assert_diff_match(
            &CompareMode::Exact,
            "constants.getProviderName",
            &json!([provider]),
            &ts_result,
            &json!(rust_result),
        );
    }
}
