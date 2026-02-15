//! Ed25519 cryptographic operations for the Phantom Connect SDK.
//!
//! Provides keypair generation, keypair restoration from secret keys,
//! and Ed25519 detached signature creation.

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use thiserror::Error;

/// Errors that can occur during cryptographic operations.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Failed to decode a base58-encoded key.
    #[error("base58 decode error: {0}")]
    Base58Decode(#[from] bs58::decode::Error),
    /// The secret key has an invalid length.
    #[error("invalid secret key length: expected 64 bytes, got {0}")]
    InvalidSecretKeyLength(usize),
    /// Ed25519 signature error.
    #[error("ed25519 error: {0}")]
    Ed25519(#[from] ed25519_dalek::SignatureError),
}

/// An Ed25519 keypair with base58-encoded public and secret keys.
#[derive(Debug, Clone)]
pub struct Keypair {
    /// Base58-encoded public key.
    pub public_key: String,
    /// Base58-encoded secret key (64 bytes: 32-byte seed + 32-byte public key).
    pub secret_key: String,
}

/// Generate a new Ed25519 keypair.
///
/// Returns a keypair with base58-encoded public and secret keys.
pub fn generate_key_pair() -> Keypair {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key: VerifyingKey = signing_key.verifying_key();

    // ed25519-dalek secret key is 32-byte seed; tweetnacl uses 64-byte (seed+pubkey)
    let mut full_secret = [0u8; 64];
    full_secret[..32].copy_from_slice(&signing_key.to_bytes());
    full_secret[32..].copy_from_slice(verifying_key.as_bytes());

    Keypair {
        public_key: bs58::encode(verifying_key.as_bytes()).into_string(),
        secret_key: bs58::encode(&full_secret).into_string(),
    }
}

/// Create a keypair from a base58-encoded private key.
///
/// The private key is expected to be 64 bytes (32-byte seed + 32-byte public key),
/// matching the tweetnacl format used by the TypeScript SDK.
///
/// # Arguments
/// * `b58_private_key` - Base58-encoded 64-byte secret key.
///
/// # Errors
/// Returns an error if the key cannot be decoded or has an invalid length.
pub fn create_key_pair_from_secret(b58_private_key: &str) -> Result<Keypair, CryptoError> {
    let secret_bytes = bs58::decode(b58_private_key).into_vec()?;

    if secret_bytes.len() != 64 {
        return Err(CryptoError::InvalidSecretKeyLength(secret_bytes.len()));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&secret_bytes[..32]);
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    let mut full_secret = [0u8; 64];
    full_secret[..32].copy_from_slice(&signing_key.to_bytes());
    full_secret[32..].copy_from_slice(verifying_key.as_bytes());

    Ok(Keypair {
        public_key: bs58::encode(verifying_key.as_bytes()).into_string(),
        secret_key: bs58::encode(&full_secret).into_string(),
    })
}

/// Sign data using Ed25519 with a secret key (detached signature).
///
/// # Arguments
/// * `secret_key` - Either a base58-encoded secret key string or raw bytes.
/// * `data` - The data bytes to sign.
///
/// # Returns
/// The 64-byte Ed25519 detached signature.
///
/// # Errors
/// Returns an error if the secret key cannot be decoded or has an invalid length.
pub fn sign_with_secret(secret_key: &SecretKeyInput, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let secret_bytes = match secret_key {
        SecretKeyInput::Base58(s) => bs58::decode(s).into_vec()?,
        SecretKeyInput::Bytes(b) => b.to_vec(),
    };

    if secret_bytes.len() != 64 {
        return Err(CryptoError::InvalidSecretKeyLength(secret_bytes.len()));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&secret_bytes[..32]);
    let signing_key = SigningKey::from_bytes(&seed);

    let signature = signing_key.sign(data);
    Ok(signature.to_bytes().to_vec())
}

/// Input type for secret key — either base58-encoded string or raw bytes.
pub enum SecretKeyInput<'a> {
    /// Base58-encoded secret key.
    Base58(&'a str),
    /// Raw secret key bytes.
    Bytes(&'a [u8]),
}
