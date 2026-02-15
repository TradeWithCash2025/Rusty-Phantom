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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_uuid_format() {
        let uuid = random_uuid();
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn random_uuid_unique() {
        let uuid1 = random_uuid();
        let uuid2 = random_uuid();
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn random_uuid_version_4() {
        let uuid = random_uuid();
        assert_eq!(uuid.as_bytes()[14], b'4');
    }

    #[test]
    fn random_uuid_variant_bits() {
        let uuid = random_uuid();
        let variant_char = uuid.as_bytes()[19];
        assert!(
            [b'8', b'9', b'a', b'b'].contains(&variant_char),
            "variant char should be 8,9,a,b, got {}",
            variant_char as char
        );
    }

    #[test]
    fn random_string_length() {
        assert_eq!(random_string(5).len(), 5);
        assert_eq!(random_string(10).len(), 10);
        assert_eq!(random_string(20).len(), 20);
    }

    #[test]
    fn random_string_unique() {
        let s1 = random_string(10);
        let s2 = random_string(10);
        assert_ne!(s1, s2);
    }

    #[test]
    fn random_string_alphanumeric() {
        let s = random_string(100);
        for c in s.chars() {
            assert!(c.is_ascii_alphanumeric(), "expected alphanumeric, got '{c}'");
        }
    }

    #[test]
    fn random_string_empty() {
        assert_eq!(random_string(0).len(), 0);
    }

    #[test]
    fn random_string_single_char() {
        assert_eq!(random_string(1).len(), 1);
    }
}
