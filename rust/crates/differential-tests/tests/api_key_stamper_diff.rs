//! Differential tests: ApiKeyStamper (TS) vs phantom-api-key-stamper (Rust).

mod test_helpers;

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_crypto::generate_key_pair;
use phantom_differential_tests::compare::{assert_diff_match, canonicalize, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use phantom_sdk_types::StampParams;
use proptest::prelude::*;
use serde_json::json;
use test_helpers::seed_to_secret_b58;

/// Helper: call TS ApiKeyStamper.stamp via oracle.
fn ts_stamp(secret_b58: &str, data: &[u8]) -> serde_json::Value {
    let args = json!([secret_b58, { "__bytes__": data }]);
    oracle_call("apiKeyStamper.stamp", &args).unwrap_ok()
}

/// Decode a base64url stamp string into parsed JSON.
fn decode_stamp(stamp: &str) -> serde_json::Value {
    let bytes =
        phantom_base64url::base64url_decode(stamp).expect("stamp should be valid base64url");
    let s = String::from_utf8(bytes).expect("stamp should be valid UTF-8");
    serde_json::from_str(&s).expect("stamp should be valid JSON")
}

/// Create a tokio runtime for blocking on async stamp calls in tests.
fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

// --- Proptest: stamp PKI ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn stamp_pki_matches_ts(
        seed in proptest::collection::vec(any::<u8>(), 32..=32),
        data in proptest::collection::vec(any::<u8>(), 1..256),
    ) {
        let secret_b58 = seed_to_secret_b58(&seed);

        // Rust stamp
        let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
            api_secret_key: secret_b58.clone(),
        })
        .unwrap();

        let rt = test_runtime();
        let rust_stamp = rt
            .block_on(phantom_sdk_types::Stamper::stamp(
                &stamper,
                StampParams::Pki { data: data.clone() },
            ))
            .unwrap();

        // TS stamp
        let ts_stamp_str = ts_stamp(&secret_b58, &data);
        let ts_stamp_str = ts_stamp_str.as_str().expect("TS stamp should be a string");

        // Compare decoded JSON (handles key ordering differences)
        let rust_parsed = canonicalize(&decode_stamp(&rust_stamp));
        let ts_parsed = canonicalize(&decode_stamp(ts_stamp_str));

        assert_diff_match(
            &CompareMode::Exact,
            "apiKeyStamper.stamp",
            &json!([secret_b58, { "__bytes__": data }]),
            &ts_parsed,
            &rust_parsed,
        );
    }
}

// --- Edge cases ---

#[test]
fn stamp_single_byte_data() {
    let kp = generate_key_pair();

    let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
        api_secret_key: kp.secret_key.clone(),
    })
    .unwrap();

    let data = vec![42u8];

    let rt = test_runtime();
    let rust_stamp = rt
        .block_on(phantom_sdk_types::Stamper::stamp(
            &stamper,
            StampParams::Pki { data: data.clone() },
        ))
        .unwrap();

    let ts_stamp_str = ts_stamp(&kp.secret_key, &data);
    let ts_stamp_str = ts_stamp_str.as_str().expect("TS stamp should be a string");

    let rust_parsed = canonicalize(&decode_stamp(&rust_stamp));
    let ts_parsed = canonicalize(&decode_stamp(ts_stamp_str));

    assert_diff_match(
        &CompareMode::Exact,
        "apiKeyStamper.stamp",
        &json!([kp.secret_key, { "__bytes__": data }]),
        &ts_parsed,
        &rust_parsed,
    );
}

#[test]
fn stamp_contains_expected_fields() {
    let kp = generate_key_pair();

    let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
        api_secret_key: kp.secret_key.clone(),
    })
    .unwrap();

    let data = b"test data for field validation".to_vec();

    let rt = test_runtime();
    let rust_stamp = rt
        .block_on(phantom_sdk_types::Stamper::stamp(
            &stamper,
            StampParams::Pki { data },
        ))
        .unwrap();

    let parsed = decode_stamp(&rust_stamp);
    let obj = parsed.as_object().expect("stamp should be an object");

    assert!(obj.contains_key("publicKey"), "stamp missing publicKey");
    assert!(obj.contains_key("signature"), "stamp missing signature");
    assert!(obj.contains_key("kind"), "stamp missing kind");
    assert_eq!(obj["kind"], "PKI", "stamp kind should be PKI");
    assert!(obj.contains_key("algorithm"), "stamp missing algorithm");
    assert_eq!(
        obj["algorithm"], "Ed25519",
        "stamp algorithm should be Ed25519"
    );
}
