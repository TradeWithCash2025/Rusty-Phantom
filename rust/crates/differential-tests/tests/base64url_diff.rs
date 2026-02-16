//! Differential tests: base64url (TS) vs phantom-base64url (Rust).

use phantom_differential_tests::compare::{assert_diff_match, CompareMode};
use phantom_differential_tests::oracle::oracle_call;
use proptest::prelude::*;
use serde_json::json;

/// Helper: convert Rust bytes to the oracle's __bytes__ format and call encode.
fn ts_encode(data: &[u8]) -> serde_json::Value {
    let args = json!([{ "__bytes__": data }]);
    oracle_call("base64url.base64urlEncode", &args).unwrap_ok()
}

/// Helper: call TS decode.
fn ts_decode(encoded: &str) -> serde_json::Value {
    let args = json!([encoded]);
    oracle_call("base64url.base64urlDecode", &args).unwrap_ok()
}

/// Helper: call TS stringToBase64url.
fn ts_string_to_base64url(s: &str) -> serde_json::Value {
    let args = json!([s]);
    oracle_call("base64url.stringToBase64url", &args).unwrap_ok()
}

/// Helper: call TS decodeToString.
fn ts_decode_to_string(encoded: &str) -> serde_json::Value {
    let args = json!([encoded]);
    oracle_call("base64url.base64urlDecodeToString", &args).unwrap_ok()
}

// --- Proptest differential tests ---

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn base64url_encode_matches_ts(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let rust_result = phantom_base64url::base64url_encode(&data);
        let ts_result = ts_encode(&data);
        assert_diff_match(
            &CompareMode::Exact,
            "base64url.base64urlEncode",
            &json!([{ "__bytes__": data }]),
            &ts_result,
            &json!(rust_result),
        );
    }

    #[test]
    fn base64url_decode_roundtrip_matches_ts(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        // Encode with Rust, then decode with both sides
        let encoded = phantom_base64url::base64url_encode(&data);
        let rust_decoded: Vec<u8> = phantom_base64url::base64url_decode(&encoded).unwrap();
        let ts_decoded = ts_decode(&encoded);

        // Convert Rust result to JSON array of numbers for comparison
        let rust_json: serde_json::Value = rust_decoded.iter().map(|&b| json!(b)).collect::<Vec<_>>().into();
        assert_diff_match(
            &CompareMode::ByteArray,
            "base64url.base64urlDecode",
            &json!([encoded]),
            &ts_decoded,
            &rust_json,
        );
    }

    #[test]
    fn string_to_base64url_matches_ts(s in "[\\x20-\\x7e]{0,256}") {
        // Use ASCII printable range for safe cross-language string comparison
        let rust_result = phantom_base64url::string_to_base64url(&s);
        let ts_result = ts_string_to_base64url(&s);
        assert_diff_match(
            &CompareMode::Exact,
            "base64url.stringToBase64url",
            &json!([s]),
            &ts_result,
            &json!(rust_result),
        );
    }

    #[test]
    fn base64url_decode_to_string_roundtrip_matches_ts(s in "[\\x20-\\x7e]{0,256}") {
        let encoded = phantom_base64url::string_to_base64url(&s);
        let rust_decoded = phantom_base64url::base64url_decode_to_string(&encoded).unwrap();
        let ts_decoded = ts_decode_to_string(&encoded);
        assert_diff_match(
            &CompareMode::Exact,
            "base64url.base64urlDecodeToString",
            &json!([encoded]),
            &ts_decoded,
            &json!(rust_decoded),
        );
    }
}

// --- Edge case tests ---

#[test]
fn base64url_encode_empty() {
    let rust_result = phantom_base64url::base64url_encode(&[]);
    let ts_result = ts_encode(&[]);
    assert_diff_match(
        &CompareMode::Exact,
        "base64url.base64urlEncode",
        &json!([{ "__bytes__": Vec::<u8>::new() }]),
        &ts_result,
        &json!(rust_result),
    );
}

#[test]
fn base64url_encode_url_unsafe_bytes() {
    // Bytes that produce +, /, = in standard base64
    let data = vec![62, 63, 64, 255, 254, 253];
    let rust_result = phantom_base64url::base64url_encode(&data);
    let ts_result = ts_encode(&data);
    assert_diff_match(
        &CompareMode::Exact,
        "base64url.base64urlEncode",
        &json!([{ "__bytes__": data }]),
        &ts_result,
        &json!(rust_result),
    );

    // Verify no forbidden characters
    assert!(!rust_result.contains('+'));
    assert!(!rust_result.contains('/'));
    assert!(!rust_result.contains('='));
}

#[test]
fn string_to_base64url_unicode() {
    let cases = [
        "Hello \u{1f30d} World",
        "\u{00e9}\u{00e8}\u{00ea}",
        "\u{4e16}\u{754c}",
    ];
    for s in &cases {
        let rust_result = phantom_base64url::string_to_base64url(s);
        let ts_result = ts_string_to_base64url(s);
        assert_diff_match(
            &CompareMode::Exact,
            "base64url.stringToBase64url",
            &json!([s]),
            &ts_result,
            &json!(rust_result),
        );
    }
}
