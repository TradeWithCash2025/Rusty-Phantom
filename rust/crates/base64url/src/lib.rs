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
