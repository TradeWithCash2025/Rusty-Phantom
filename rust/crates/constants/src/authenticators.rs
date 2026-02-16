//! Authenticator-related constants.

use serde::{Deserialize, Serialize};

/// Cryptographic algorithm used for authenticator operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Algorithm {
    /// Ed25519 elliptic curve digital signature algorithm.
    #[serde(rename = "Ed25519")]
    Ed25519,
    /// ECDSA with the P-256 (secp256r1) curve.
    #[serde(rename = "Secp256r1")]
    Secp256r1,
}

/// Default authenticator algorithm — Ed25519.
pub const DEFAULT_AUTHENTICATOR_ALGORITHM: Algorithm = Algorithm::Ed25519;
