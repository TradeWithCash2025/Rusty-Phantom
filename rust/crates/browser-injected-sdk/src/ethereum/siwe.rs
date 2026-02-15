//! SIWE (Sign In With Ethereum) message creation.
//!
//! Creates EIP-4361 messages. Adapted from viem's createSiweMessage implementation.
//! Copyright (c) 2023-present weth, LLC — Licensed under the MIT License.

use super::types::EthereumSignInData;
use regex::Regex;

/// Create an EIP-4361 Sign In With Ethereum message.
pub fn create_siwe_message(data: &EthereumSignInData) -> Result<String, String> {
    // Validate required fields
    let address_re = Regex::new(r"^0x[a-fA-F0-9]{40}$").unwrap();
    if !address_re.is_match(&data.address) {
        return Err(
            "address must be a hex value of 20 bytes (40 hex characters).".to_string(),
        );
    }

    if data.chain_id != (data.chain_id as f64).floor() as i64 {
        return Err("chainId must be a EIP-155 chain ID.".to_string());
    }

    let domain_re =
        Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}(:[0-9]{1,5})?$")
            .unwrap();
    let ip_re = Regex::new(
        r"^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)(:[0-9]{1,5})?$",
    ).unwrap();
    let localhost_re = Regex::new(r"^localhost(:[0-9]{1,5})?$").unwrap();

    if !domain_re.is_match(&data.domain)
        && !ip_re.is_match(&data.domain)
        && !localhost_re.is_match(&data.domain)
    {
        return Err("domain must be an RFC 3986 authority.".to_string());
    }

    let nonce_re = Regex::new(r"^[a-zA-Z0-9]{8,}$").unwrap();
    if !nonce_re.is_match(&data.nonce) {
        return Err("nonce must be at least 8 characters.".to_string());
    }

    if !is_uri(&data.uri) {
        return Err(
            "uri must be a RFC 3986 URI referring to the resource that is the subject of the signing."
                .to_string(),
        );
    }

    if data.version != "1" {
        return Err("version must be '1'.".to_string());
    }

    // Optional field validation
    if let Some(scheme) = &data.scheme {
        let scheme_re = Regex::new(r"^([a-zA-Z][a-zA-Z0-9+\-.]*)$").unwrap();
        if !scheme_re.is_match(scheme) {
            return Err("scheme must be an RFC 3986 URI scheme.".to_string());
        }
    }

    if let Some(statement) = &data.statement {
        if statement.contains('\n') {
            return Err("statement must not include '\\n'.".to_string());
        }
    }

    // Construct message
    let origin = if let Some(scheme) = &data.scheme {
        format!("{}://{}", scheme, data.domain)
    } else {
        data.domain.clone()
    };

    let statement = data
        .statement
        .as_ref()
        .map(|s| format!("{}\n", s))
        .unwrap_or_default();

    let prefix = format!(
        "{} wants you to sign in with your Ethereum account:\n{}\n\n{}",
        origin, data.address, statement
    );

    // Use provided issued_at or current time
    let issued_at = data
        .issued_at
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let mut suffix = format!(
        "URI: {}\nVersion: {}\nChain ID: {}\nNonce: {}\nIssued At: {}",
        data.uri, data.version, data.chain_id, data.nonce, issued_at
    );

    if let Some(expiration_time) = &data.expiration_time {
        suffix.push_str(&format!("\nExpiration Time: {}", expiration_time));
    }
    if let Some(not_before) = &data.not_before {
        suffix.push_str(&format!("\nNot Before: {}", not_before));
    }
    if let Some(request_id) = &data.request_id {
        suffix.push_str(&format!("\nRequest ID: {}", request_id));
    }
    if let Some(resources) = &data.resources {
        suffix.push_str("\nResources:");
        for resource in resources {
            if !is_uri(resource) {
                return Err("resources must be RFC 3986 URIs.".to_string());
            }
            suffix.push_str(&format!("\n- {}", resource));
        }
    }

    Ok(format!("{}\n{}", prefix, suffix))
}

/// Check if a value is a valid RFC 3986 URI.
///
/// Adapted from viem's isUri implementation.
/// Copyright (c) 2023-present weth, LLC — Licensed under the MIT License.
pub fn is_uri(value: &str) -> bool {
    // Check for illegal characters
    let illegal_re = Regex::new(r"[^a-zA-Z0-9:/?#\[\]@!$&'()*+,;=.\-_~%]").unwrap();
    if illegal_re.is_match(value) {
        return false;
    }

    // Check for incomplete hex escapes
    let hex_re1 = Regex::new(r"(?i)%[^0-9a-f]").unwrap();
    if hex_re1.is_match(value) {
        return false;
    }
    let hex_re2 = Regex::new(r"(?i)%[0-9a-f](:?[^0-9a-f]|$)").unwrap();
    if hex_re2.is_match(value) {
        return false;
    }

    // From RFC 3986
    let parts = split_uri(value);
    let (scheme, authority, path, query, fragment) = match parts {
        Some(p) => p,
        None => return false,
    };

    // Scheme and path are required (though path can be empty)
    if scheme.is_empty() {
        return false;
    }

    // If authority is present, path must be empty or begin with /
    if let Some(auth) = &authority {
        if !auth.is_empty() && !path.is_empty() && !path.starts_with('/') {
            return false;
        }
    } else if path.starts_with("//") {
        return false;
    }

    // Scheme must begin with a letter, then consist of letters, digits, +, ., or -
    let scheme_re = Regex::new(r"^[a-zA-Z][a-zA-Z0-9+\-.]*$").unwrap();
    if !scheme_re.is_match(&scheme) {
        return false;
    }

    // Re-assemble per section 5.3 in RFC 3986
    let mut _out = format!("{}:", scheme);
    if let Some(auth) = &authority {
        if !auth.is_empty() {
            _out.push_str(&format!("//{}", auth));
        }
    }
    _out.push_str(&path);
    if let Some(q) = &query {
        _out.push_str(&format!("?{}", q));
    }
    if let Some(f) = &fragment {
        _out.push_str(&format!("#{}", f));
    }

    true
}

/// Split a URI into its components per RFC 3986.
fn split_uri(value: &str) -> Option<(String, Option<String>, String, Option<String>, Option<String>)> {
    let re =
        Regex::new(r"(?:([^:/?#]+):)?(?://([^/?#]*))?([^?#]*)(?:\?([^#]*))?(?:#(.*))?").unwrap();
    let caps = re.captures(value)?;

    let scheme = caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
    let authority = caps.get(2).map(|m| m.as_str().to_string());
    let path = caps.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
    let query = caps.get(4).map(|m| m.as_str().to_string());
    let fragment = caps.get(5).map(|m| m.as_str().to_string());

    Some((scheme, authority, path, query, fragment))
}
