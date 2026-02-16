//! Differential tests: ApiKeyStamper (TS) vs phantom-api-key-stamper (Rust).

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_crypto::generate_key_pair;
use phantom_differential_tests::compare::{assert_diff_match, canonicalize, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use phantom_sdk_types::StampParams;
use proptest::prelude::*;
use serde_json::json;

/// Helper: call TS ApiKeyStamper.stamp via oracle.
fn ts_stamp(secret_b58: &str, data: &[u8]) -> serde_json::Value {
    let args = json!([secret_b58, { "__bytes__": data }]);
    oracle_call("apiKeyStamper.stamp", &args).unwrap_ok()
}

/// Decode a base64url stamp string into parsed JSON.
fn decode_stamp(stamp: &str) -> serde_json::Value {
    let bytes = phantom_base64url::base64url_decode(stamp)
        .expect("stamp should be valid base64url");
    let s = String::from_utf8(bytes).expect("stamp should be valid UTF-8");
    serde_json::from_str(&s).expect("stamp should be valid JSON")
}

// --- Proptest: stamp PKI ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn stamp_pki_matches_ts(
        seed in proptest::collection::vec(any::<u8>(), 32..=32),
        data in proptest::collection::vec(any::<u8>(), 1..256),
    ) {
        // Build valid secret key
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&seed.try_into().unwrap());
        let verifying_key = signing_key.verifying_key();
        let mut full_secret = [0u8; 64];
        full_secret[..32].copy_from_slice(&signing_key.to_bytes());
        full_secret[32..].copy_from_slice(verifying_key.as_bytes());
        let secret_b58 = bs58::encode(&full_secret).into_string();

        // Rust stamp
        let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
            api_secret_key: secret_b58.clone(),
        })
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
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

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
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

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
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
}
