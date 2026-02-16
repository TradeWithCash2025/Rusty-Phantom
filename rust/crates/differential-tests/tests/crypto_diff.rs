//! Differential tests: crypto (TS tweetnacl) vs phantom-crypto (Rust ed25519-dalek).

mod test_helpers;

use phantom_crypto::{
    create_key_pair_from_secret, generate_key_pair, sign_with_secret, SecretKeyInput,
};
use phantom_differential_tests::compare::{assert_diff_match, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use proptest::prelude::*;
use serde_json::json;
use test_helpers::seed_to_secret_b58;

/// Generate a valid base58-encoded 64-byte Ed25519 secret key using Rust.
fn generate_valid_secret_key() -> String {
    let kp = generate_key_pair();
    kp.secret_key
}

/// Call TS createKeyPairFromSecret.
fn ts_create_keypair(secret_b58: &str) -> serde_json::Value {
    oracle_call("crypto.createKeyPairFromSecret", &json!([secret_b58])).unwrap_ok()
}

/// Call TS signWithSecret.
fn ts_sign(secret_b58: &str, data: &[u8]) -> serde_json::Value {
    let args = json!([secret_b58, { "__bytes__": data }]);
    oracle_call("crypto.signWithSecret", &args).unwrap_ok()
}

// --- createKeyPairFromSecret ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn create_keypair_from_secret_matches_ts(seed in proptest::collection::vec(any::<u8>(), 32..=32)) {
        let secret_b58 = seed_to_secret_b58(&seed);

        let rust_kp = create_key_pair_from_secret(&secret_b58).unwrap();
        let ts_kp = ts_create_keypair(&secret_b58);

        let rust_json = json!({
            "publicKey": rust_kp.public_key,
            "secretKey": rust_kp.secret_key,
        });

        assert_diff_match(
            &CompareMode::CanonicalizeSortedKeys,
            "crypto.createKeyPairFromSecret",
            &json!([secret_b58]),
            &ts_kp,
            &rust_json,
        );
    }
}

// --- signWithSecret ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn sign_with_secret_matches_ts(
        seed in proptest::collection::vec(any::<u8>(), 32..=32),
        data in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let secret_b58 = seed_to_secret_b58(&seed);

        let rust_sig = sign_with_secret(&SecretKeyInput::Base58(&secret_b58), &data).unwrap();
        let ts_sig = ts_sign(&secret_b58, &data);

        let rust_json: serde_json::Value = rust_sig.iter().map(|&b| json!(b)).collect::<Vec<_>>().into();

        assert_diff_match(
            &CompareMode::ByteArray,
            "crypto.signWithSecret",
            &json!([secret_b58, { "__bytes__": data }]),
            &ts_sig,
            &rust_json,
        );
    }
}

// --- Edge cases ---

#[test]
fn sign_empty_data_matches_ts() {
    let secret_b58 = generate_valid_secret_key();
    let data: Vec<u8> = vec![];

    let rust_sig = sign_with_secret(&SecretKeyInput::Base58(&secret_b58), &data).unwrap();
    let ts_sig = ts_sign(&secret_b58, &data);

    let rust_json: serde_json::Value = rust_sig
        .iter()
        .map(|&b| json!(b))
        .collect::<Vec<_>>()
        .into();

    assert_diff_match(
        &CompareMode::ByteArray,
        "crypto.signWithSecret",
        &json!([secret_b58, { "__bytes__": data }]),
        &ts_sig,
        &rust_json,
    );
}

#[test]
fn keypair_deterministic() {
    let secret_b58 = generate_valid_secret_key();

    let rust1 = create_key_pair_from_secret(&secret_b58).unwrap();
    let rust2 = create_key_pair_from_secret(&secret_b58).unwrap();
    assert_eq!(rust1.public_key, rust2.public_key);
    assert_eq!(rust1.secret_key, rust2.secret_key);

    let ts1 = ts_create_keypair(&secret_b58);
    let ts2 = ts_create_keypair(&secret_b58);
    assert_eq!(ts1, ts2);
}
