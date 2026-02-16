//! Tests that run shared JSON fixtures against both Rust and TS implementations.
//!
//! Fixtures are in the `fixtures/` directory at the repo root.

use phantom_differential_tests::compare::{assert_diff_match, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Load a fixture file from the fixtures/ directory.
fn load_fixture(name: &str) -> Vec<Value> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Cannot find repo root")
        .to_path_buf();
    let path = repo_root.join("fixtures").join(name);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {e}", path.display()))
}

// --- base64url fixtures ---

#[test]
fn base64url_fixtures_match_both_sides() {
    let fixtures = load_fixture("base64url.json");

    for fixture in &fixtures {
        let fn_name = fixture["fn"].as_str().unwrap();
        let args = fixture["args"].as_array().unwrap().clone();

        // Call TS oracle
        let ts_result = oracle_call(fn_name, &Value::Array(args.clone())).unwrap_ok();

        // Call Rust and compare based on function
        match fn_name {
            "base64url.stringToBase64url" => {
                let input = args[0].as_str().unwrap();
                let rust_encoded = phantom_base64url::string_to_base64url(input);
                let expected = fixture["expected_encoded"].as_str().unwrap();

                assert_eq!(
                    rust_encoded, expected,
                    "Rust stringToBase64url fixture mismatch"
                );
                assert_eq!(
                    ts_result.as_str().unwrap(),
                    expected,
                    "TS stringToBase64url fixture mismatch"
                );
            }
            "base64url.base64urlEncode" => {
                let input_bytes: Vec<u8> = args[0]["__bytes__"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u8)
                    .collect();
                let rust_encoded = phantom_base64url::base64url_encode(&input_bytes);
                let expected = fixture["expected_encoded"].as_str().unwrap();

                assert_eq!(
                    rust_encoded, expected,
                    "Rust base64urlEncode fixture mismatch"
                );
                assert_eq!(
                    ts_result.as_str().unwrap(),
                    expected,
                    "TS base64urlEncode fixture mismatch"
                );
            }
            _ => {
                // For other functions, just verify TS oracle returns something
                assert!(
                    !ts_result.is_null() || fixture.get("expected").is_some_and(|v| v.is_null()),
                    "Unexpected null from TS for {fn_name}"
                );
            }
        }
    }
}

// --- crypto fixtures ---

#[test]
fn crypto_fixtures_match_both_sides() {
    let fixtures = load_fixture("crypto.json");

    for fixture in &fixtures {
        let fn_name = fixture["fn"].as_str().unwrap();
        let args = fixture["args"].as_array().unwrap().clone();

        let ts_result = oracle_call(fn_name, &Value::Array(args.clone())).unwrap_ok();

        match fn_name {
            "crypto.createKeyPairFromSecret" => {
                let secret_b58 = args[0].as_str().unwrap();
                let rust_kp = phantom_crypto::create_key_pair_from_secret(secret_b58).unwrap();
                let expected = &fixture["expected"];

                assert_eq!(
                    rust_kp.public_key,
                    expected["publicKey"].as_str().unwrap(),
                    "Rust publicKey mismatch"
                );
                assert_eq!(
                    rust_kp.secret_key,
                    expected["secretKey"].as_str().unwrap(),
                    "Rust secretKey mismatch"
                );

                // TS should also match
                assert_diff_match(
                    &CompareMode::CanonicalizeSortedKeys,
                    fn_name,
                    &Value::Array(args),
                    &ts_result,
                    &json!({
                        "publicKey": rust_kp.public_key,
                        "secretKey": rust_kp.secret_key,
                    }),
                );
            }
            "crypto.signWithSecret" => {
                let secret_b58 = args[0].as_str().unwrap();
                let data_bytes: Vec<u8> = args[1]["__bytes__"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u8)
                    .collect();

                let rust_sig = phantom_crypto::sign_with_secret(
                    &phantom_crypto::SecretKeyInput::Base58(secret_b58),
                    &data_bytes,
                )
                .unwrap();

                let expected_sig: Vec<u8> = fixture["expected_signature"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_u64().unwrap() as u8)
                    .collect();

                assert_eq!(rust_sig, expected_sig, "Rust signature fixture mismatch");

                // TS should also match
                let rust_json: Value = rust_sig
                    .iter()
                    .map(|&b| json!(b))
                    .collect::<Vec<_>>()
                    .into();
                assert_diff_match(
                    &CompareMode::ByteArray,
                    fn_name,
                    &Value::Array(args),
                    &ts_result,
                    &rust_json,
                );
            }
            _ => {}
        }
    }
}

// --- constants fixtures ---

#[test]
fn constants_fixtures_match_ts() {
    let fixtures = load_fixture("constants.json");

    for fixture in &fixtures {
        let fn_name = fixture["fn"].as_str().unwrap();
        let args = fixture["args"].as_array().unwrap().clone();

        let ts_result = oracle_call(fn_name, &Value::Array(args.clone())).unwrap_ok();
        let expected = &fixture["expected"];

        // For constants, we primarily verify TS oracle matches the fixture
        // (the Rust side is tested separately in constants_diff.rs)
        if expected.is_null() {
            assert!(
                ts_result.is_null(),
                "TS returned non-null for {fn_name} where fixture expected null"
            );
        } else if expected.is_array() {
            // Compare as sorted arrays for network lists
            let mut ts_arr: Vec<String> = ts_result
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            ts_arr.sort();

            let mut exp_arr: Vec<String> = expected
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            exp_arr.sort();

            assert_eq!(ts_arr, exp_arr, "TS {fn_name} fixture mismatch");
        } else {
            assert_diff_match(
                &CompareMode::CanonicalizeSortedKeys,
                fn_name,
                &Value::Array(args),
                &ts_result,
                expected,
            );
        }
    }
}

// --- api_key_stamper fixtures ---

#[test]
fn api_key_stamper_fixtures_match_both_sides() {
    let fixtures = load_fixture("api_key_stamper.json");

    for fixture in &fixtures {
        let fn_name = fixture["fn"].as_str().unwrap();
        let args = fixture["args"].as_array().unwrap().clone();

        let ts_result = oracle_call(fn_name, &Value::Array(args.clone())).unwrap_ok();
        let expected = fixture["expected"].as_str().unwrap();

        // TS should match the recorded fixture
        assert_eq!(
            ts_result.as_str().unwrap(),
            expected,
            "TS stamp fixture mismatch"
        );

        // Rust should also produce the same stamp
        let secret_b58 = args[0].as_str().unwrap();
        let data_bytes: Vec<u8> = args[1]["__bytes__"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u8)
            .collect();

        let stamper = phantom_api_key_stamper::ApiKeyStamper::new(
            phantom_api_key_stamper::ApiKeyStamperConfig {
                api_secret_key: secret_b58.to_string(),
            },
        )
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let rust_stamp = rt
            .block_on(phantom_sdk_types::Stamper::stamp(
                &stamper,
                phantom_sdk_types::StampParams::Pki { data: data_bytes },
            ))
            .unwrap();

        assert_eq!(rust_stamp, expected, "Rust stamp fixture mismatch");
    }
}
