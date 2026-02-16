//! Shared test helpers for differential integration tests.

/// Build a valid base58-encoded 64-byte Ed25519 secret key from a 32-byte seed.
///
/// This is the standard format expected by both the TS and Rust crypto packages:
/// `[signing_key_bytes(32) || verifying_key_bytes(32)]` encoded as base58.
pub fn seed_to_secret_b58(seed: &[u8]) -> String {
    use ed25519_dalek::SigningKey;

    let seed_array: [u8; 32] = seed.try_into().expect("seed must be 32 bytes");
    let signing_key = SigningKey::from_bytes(&seed_array);
    let verifying_key = signing_key.verifying_key();
    let mut full_secret = [0u8; 64];
    full_secret[..32].copy_from_slice(&signing_key.to_bytes());
    full_secret[32..].copy_from_slice(verifying_key.as_bytes());
    bs58::encode(&full_secret).into_string()
}
