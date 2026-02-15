//! Deeplink generation for Phantom mobile app.

/// Generate a deeplink URL to open the current page in Phantom mobile app.
///
/// # Arguments
/// * `current_href` - The URL to open in Phantom (must be HTTP or HTTPS)
/// * `referrer` - Optional referrer parameter
pub fn get_deeplink_to_phantom(current_href: &str, referrer: Option<&str>) -> Result<String, String> {
    if !current_href.starts_with("http:") && !current_href.starts_with("https:") {
        return Err(
            "Invalid URL protocol - only HTTP/HTTPS URLs are supported for deeplinks".to_string(),
        );
    }

    let encoded_url = urlencoding_encode(current_href);
    let ref_param = match referrer {
        Some(r) => format!("?ref={}", urlencoding_encode(r)),
        None => String::new(),
    };

    Ok(format!(
        "https://phantom.app/ul/browse/{}{}",
        encoded_url, ref_param
    ))
}

/// Simple percent-encoding for URLs (subset of encodeURIComponent).
fn urlencoding_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => result.push(byte as char),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}
