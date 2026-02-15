//! Shared SDK type definitions for the Phantom Connect SDK.
//!
//! This crate defines the core traits and types used across all SDK packages,
//! including the `Stamper` trait for request signing and key management interfaces.

use serde::{Deserialize, Serialize};

// Re-export Algorithm from constants
pub use phantom_constants::Algorithm;

/// Parameters for a stamp operation.
#[derive(Debug, Clone)]
pub enum StampParams {
    /// PKI-based stamp using keypair signing.
    Pki {
        /// Raw data to sign.
        data: Vec<u8>,
    },
    /// OIDC-based stamp using an ID token.
    Oidc {
        /// Raw data to sign.
        data: Vec<u8>,
        /// OIDC ID token.
        id_token: String,
        /// Salt for the stamp.
        salt: String,
    },
}

/// Type of stamper authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StamperType {
    /// Public key infrastructure (keypair-based) authentication.
    #[serde(rename = "PKI")]
    Pki,
    /// OpenID Connect (token-based) authentication.
    #[serde(rename = "OIDC")]
    Oidc,
}

/// Stamper trait — takes data and returns a complete X-Phantom-Stamp header value.
///
/// Implementors sign request data and produce a base64url-encoded stamp
/// containing the signature, public key, and algorithm metadata.
#[async_trait::async_trait]
pub trait Stamper: Send + Sync {
    /// Sign data and produce a stamp header value.
    async fn stamp(&self, params: StampParams) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// The cryptographic algorithm used by this stamper.
    fn algorithm(&self) -> Algorithm;

    /// The authentication type of this stamper.
    fn stamper_type(&self) -> StamperType;

    /// OIDC ID token (only for OIDC stampers).
    fn id_token(&self) -> Option<&str> {
        None
    }

    /// OIDC salt (only for OIDC stampers).
    fn salt(&self) -> Option<&str> {
        None
    }

    /// Get key info if this stamper supports key management.
    ///
    /// Default returns `None`. Override in stampers that implement
    /// `StamperWithKeyManagement` to return the current key info.
    fn get_key_info(&self) -> Option<StamperKeyInfo> {
        None
    }
}

/// Key information structure returned by stampers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StamperKeyInfo {
    /// Key identifier.
    pub key_id: String,
    /// Base58-encoded public key.
    pub public_key: String,
    /// Optional timestamp when the key was created.
    pub created_at: Option<u64>,
    /// Optional authenticator ID from the server.
    pub authenticator_id: Option<String>,
}

/// Extended stamper trait for stampers that manage their own keys.
///
/// Adds key initialization, retrieval, reset, and rotation capabilities
/// on top of the base `Stamper` trait.
#[async_trait::async_trait]
pub trait StamperWithKeyManagement: Stamper {
    /// Initialize the stamper and generate or load a keypair.
    async fn init(&self) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the current key information, if available.
    fn get_key_info(&self) -> Option<StamperKeyInfo>;

    /// Reset the keypair, generating a new one.
    async fn reset_key_pair(&self) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>>;

    /// Clear all stored key material.
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Generate a new keypair for rotation, keeping the old one as pending.
    async fn rotate_key_pair(&self) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>>;

    /// Commit the pending rotation, switching to the new keypair.
    async fn commit_rotation(&self, authenticator_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Roll back the pending rotation, discarding the new keypair.
    async fn rollback_rotation(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}
