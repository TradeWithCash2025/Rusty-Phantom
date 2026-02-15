//! Isomorphic base64url encoding/decoding utilities.
//!
//! In Rust there is no browser/Node.js distinction — we use the `base64` crate
//! with the URL-safe alphabet uniformly.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Encode data to base64url format.
///
/// Accepts either raw bytes or a UTF-8 string. Returns a base64url-encoded string
/// with no padding characters.
///
/// # Arguments
/// * `data` - Byte slice to encode.
pub fn base64url_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

/// Decode a base64url string to bytes.
///
/// Handles both padded and unpadded base64url input.
///
/// # Arguments
/// * `s` - base64url encoded string.
///
/// # Errors
/// Returns an error if the input is not valid base64url.
pub fn base64url_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    // The URL_SAFE_NO_PAD engine also accepts padded input
    URL_SAFE_NO_PAD.decode(s)
}

/// Decode a base64url string to a UTF-8 string.
///
/// # Arguments
/// * `s` - base64url encoded string.
///
/// # Errors
/// Returns an error if the input is not valid base64url or not valid UTF-8.
pub fn base64url_decode_to_string(s: &str) -> Result<String, Base64UrlError> {
    let bytes = base64url_decode(s)?;
    String::from_utf8(bytes).map_err(Base64UrlError::Utf8)
}

/// Encode a UTF-8 string to base64url format.
///
/// # Arguments
/// * `s` - UTF-8 string to encode.
pub fn string_to_base64url(s: &str) -> String {
    base64url_encode(s.as_bytes())
}

/// Errors that can occur during base64url operations.
#[derive(Debug, thiserror::Error)]
pub enum Base64UrlError {
    /// Base64 decoding failed.
    #[error("base64 decode error: {0}")]
    Decode(#[from] base64::DecodeError),
    /// Decoded bytes are not valid UTF-8.
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_string_to_base64url() {
        assert_eq!(base64url_encode(b"Hello World"), "SGVsbG8gV29ybGQ");
    }

    #[test]
    fn encode_bytes_to_base64url() {
        let input = [72, 101, 108, 108, 111]; // "Hello"
        assert_eq!(base64url_encode(&input), "SGVsbG8");
    }

    #[test]
    fn encode_empty_input() {
        assert_eq!(base64url_encode(b""), "");
    }

    #[test]
    fn encode_no_plus_slash_padding() {
        let input = [62, 63, 64];
        let result = base64url_encode(&input);
        assert!(!result.contains('+'));
        assert!(!result.contains('/'));
        assert!(!result.contains('='));
    }

    #[test]
    fn decode_base64url_to_bytes() {
        let result = base64url_decode("SGVsbG8gV29ybGQ").unwrap();
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn decode_empty_string() {
        let result = base64url_decode("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn decode_no_padding() {
        let result = base64url_decode("SGVsbG8").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn decode_to_string() {
        let result = base64url_decode_to_string("SGVsbG8gV29ybGQ").unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn decode_to_string_empty() {
        let result = base64url_decode_to_string("").unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn string_to_base64url_roundtrip() {
        assert_eq!(string_to_base64url("Hello World"), "SGVsbG8gV29ybGQ");
    }

    #[test]
    fn string_to_base64url_empty() {
        assert_eq!(string_to_base64url(""), "");
    }

    #[test]
    fn roundtrip_strings() {
        let cases = vec![
            "Hello World",
            "Simple test",
            r#"{"key":"value"}"#,
            r#"{"publicKey":"AQIDBAU","signature":"BgcICQo","kind":"PKI"}"#,
            "",
            "Special chars: !@#$%^&*()",
            "Multi-line\ntext\twith\ttabs",
        ];
        for s in cases {
            let encoded = string_to_base64url(s);
            let decoded = base64url_decode_to_string(&encoded).unwrap();
            assert_eq!(decoded, s, "roundtrip failed for: {s}");
        }
    }

    #[test]
    fn roundtrip_bytes() {
        let arrays: Vec<Vec<u8>> = vec![
            vec![0, 1, 2, 3, 4, 5],
            vec![255, 254, 253],
            vec![62, 63, 64, 65],
            (0u8..=255).collect(),
        ];
        for arr in arrays {
            let encoded = base64url_encode(&arr);
            let decoded = base64url_decode(&encoded).unwrap();
            assert_eq!(decoded, arr);
        }
    }

    #[test]
    fn output_is_valid_base64url_chars() {
        for s in ["Hello World", "Special chars: !@#$%^&*()"] {
            let result = string_to_base64url(s);
            for c in result.chars() {
                assert!(
                    c.is_ascii_alphanumeric() || c == '-' || c == '_',
                    "invalid char '{c}' in base64url"
                );
            }
        }
    }

    #[test]
    fn compatibility_phantom_string() {
        assert_eq!(
            string_to_base64url("Hello from Phantom!"),
            "SGVsbG8gZnJvbSBQaGFudG9tIQ"
        );
    }

    #[test]
    fn jwt_like_payload() {
        let payload = r#"{"sub":"1234567890","name":"John Doe","iat":1516239022}"#;
        let encoded = string_to_base64url(payload);
        let decoded = base64url_decode_to_string(&encoded).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn unicode_roundtrip() {
        let input = "Hello \u{1f30d} World";
        let encoded = string_to_base64url(input);
        let decoded = base64url_decode_to_string(&encoded).unwrap();
        assert_eq!(decoded, input);
    }
}
