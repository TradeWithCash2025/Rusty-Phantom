//! Authenticator-related constants.

use serde::{Deserialize, Serialize};

/// Cryptographic algorithm used for authenticator operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algorithm {
    /// Ed25519 elliptic curve digital signature algorithm.
    #[serde(rename = "ed25519")]
    Ed25519,
}

/// Default authenticator algorithm — Ed25519.
pub const DEFAULT_AUTHENTICATOR_ALGORITHM: Algorithm = Algorithm::Ed25519;
