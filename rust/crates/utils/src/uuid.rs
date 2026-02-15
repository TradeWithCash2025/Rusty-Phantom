//! UUID v4 generation and random string utilities.

use rand::Rng;

/// Generate a random UUID v4 string.
///
/// Returns a UUID v4 string in the format `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.
/// RFC 4122 compliant.
pub fn random_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a random string of specified length using alphanumeric characters.
///
/// # Arguments
/// * `length` - The length of the string to generate.
///
/// # Returns
/// A random alphanumeric string of the specified length.
pub fn random_string(length: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}
