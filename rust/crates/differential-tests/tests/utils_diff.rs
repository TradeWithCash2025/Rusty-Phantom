//! Differential tests: utils (TS) vs phantom-utils (Rust).

use phantom_differential_tests::compare::{assert_diff_match, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use phantom_utils::{get_chain_prefix, is_ethereum_chain, is_solana_chain};
use proptest::prelude::*;
use serde_json::json;

// --- getChainPrefix ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn get_chain_prefix_matches_ts(s in "[a-zA-Z0-9]{1,10}:[a-zA-Z0-9]{1,20}") {
        let rust_result = get_chain_prefix(&s);
        let ts_result = oracle_call("utils.getChainPrefix", &json!([s])).unwrap_ok();
        assert_diff_match(
            &CompareMode::Exact,
            "utils.getChainPrefix",
            &json!([s]),
            &ts_result,
            &json!(rust_result),
        );
    }

    #[test]
    fn is_ethereum_chain_matches_ts(s in "[a-zA-Z0-9]{1,10}:[a-zA-Z0-9]{1,20}") {
        let rust_result = is_ethereum_chain(&s);
        let ts_result = oracle_call("utils.isEthereumChain", &json!([s])).unwrap_ok();
        assert_diff_match(
            &CompareMode::Exact,
            "utils.isEthereumChain",
            &json!([s]),
            &ts_result,
            &json!(rust_result),
        );
    }

    #[test]
    fn is_solana_chain_matches_ts(s in "[a-zA-Z0-9]{1,10}:[a-zA-Z0-9]{1,20}") {
        let rust_result = is_solana_chain(&s);
        let ts_result = oracle_call("utils.isSolanaChain", &json!([s])).unwrap_ok();
        assert_diff_match(
            &CompareMode::Exact,
            "utils.isSolanaChain",
            &json!([s]),
            &ts_result,
            &json!(rust_result),
        );
    }
}

// --- Known network IDs ---

#[test]
fn get_chain_prefix_known_ids() {
    let cases = [
        ("eip155:1", "eip155"),
        ("eip155:137", "eip155"),
        ("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp", "solana"),
        ("bip122:000000000019d6689c085ae165831e93", "bip122"),
        ("sui:35834a8a", "sui"),
        ("EIP155:1", "eip155"),       // case insensitive
        ("SOLANA:mainnet", "solana"), // case insensitive
    ];

    for (input, expected) in &cases {
        let rust_result = get_chain_prefix(input);
        let ts_result = oracle_call("utils.getChainPrefix", &json!([input])).unwrap_ok();

        assert_eq!(rust_result, *expected, "Rust getChainPrefix({input})");
        assert_diff_match(
            &CompareMode::Exact,
            "utils.getChainPrefix",
            &json!([input]),
            &ts_result,
            &json!(rust_result),
        );
    }
}

#[test]
fn is_ethereum_chain_known_ids() {
    let eth_ids = ["eip155:1", "eip155:137", "eip155:42161", "EIP155:1"];
    let non_eth_ids = ["solana:mainnet", "bitcoin:mainnet", "sui:mainnet", ""];

    for id in &eth_ids {
        assert!(is_ethereum_chain(id), "Rust: {id} should be ethereum");
        let ts = oracle_call("utils.isEthereumChain", &json!([id])).unwrap_ok();
        assert_eq!(ts, json!(true), "TS: {id} should be ethereum");
    }

    for id in &non_eth_ids {
        assert!(!is_ethereum_chain(id), "Rust: {id} should NOT be ethereum");
        let ts = oracle_call("utils.isEthereumChain", &json!([id])).unwrap_ok();
        assert_eq!(ts, json!(false), "TS: {id} should NOT be ethereum");
    }
}

#[test]
fn is_solana_chain_known_ids() {
    let sol_ids = [
        "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp",
        "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1",
        "SOLANA:mainnet",
    ];
    let non_sol_ids = ["eip155:1", "bitcoin:mainnet", "sui:mainnet"];

    for id in &sol_ids {
        assert!(is_solana_chain(id), "Rust: {id} should be solana");
        let ts = oracle_call("utils.isSolanaChain", &json!([id])).unwrap_ok();
        assert_eq!(ts, json!(true), "TS: {id} should be solana");
    }

    for id in &non_sol_ids {
        assert!(!is_solana_chain(id), "Rust: {id} should NOT be solana");
        let ts = oracle_call("utils.isSolanaChain", &json!([id])).unwrap_ok();
        assert_eq!(ts, json!(false), "TS: {id} should NOT be solana");
    }
}

// --- Edge cases ---

#[test]
fn get_chain_prefix_no_colon() {
    let input = "nodelimiter";
    let rust_result = get_chain_prefix(input);
    let ts_result = oracle_call("utils.getChainPrefix", &json!([input])).unwrap_ok();
    assert_diff_match(
        &CompareMode::Exact,
        "utils.getChainPrefix",
        &json!([input]),
        &ts_result,
        &json!(rust_result),
    );
}

#[test]
fn get_chain_prefix_multiple_colons() {
    let input = "eip155:1:extra:stuff";
    let rust_result = get_chain_prefix(input);
    let ts_result = oracle_call("utils.getChainPrefix", &json!([input])).unwrap_ok();
    assert_diff_match(
        &CompareMode::Exact,
        "utils.getChainPrefix",
        &json!([input]),
        &ts_result,
        &json!(rust_result),
    );
}
