//! Simple stamper that takes a pre-existing secret key and creates stamps.
//!
//! Does not manage keys — just signs with the provided secret key.
//! Implements the `Stamper` trait from `phantom-sdk-types`.

use phantom_base64url::base64url_encode;
use phantom_constants::{Algorithm, DEFAULT_AUTHENTICATOR_ALGORITHM};
use phantom_crypto::{create_key_pair_from_secret, sign_with_secret, Keypair, SecretKeyInput};
use phantom_sdk_types::{StampParams, Stamper, StamperType};
use serde::Serialize;

/// Configuration for the API key stamper.
#[derive(Debug, Clone)]
pub struct ApiKeyStamperConfig {
    /// Base58-encoded API secret key.
    pub api_secret_key: String,
}

/// Simple stamper that signs requests using a pre-existing Ed25519 secret key.
///
/// This stamper only supports PKI-type authentication. It does not manage
/// key lifecycle — it simply signs data with the provided key.
pub struct ApiKeyStamper {
    keypair: Keypair,
}

/// PKI stamp data structure for JSON serialization.
#[derive(Serialize)]
struct PkiStampData {
    #[serde(rename = "publicKey")]
    public_key: String,
    signature: String,
    kind: String,
    algorithm: String,
}

/// OIDC stamp data structure for JSON serialization.
#[derive(Serialize)]
struct OidcStampData {
    kind: String,
    #[serde(rename = "idToken")]
    id_token: String,
    #[serde(rename = "publicKey")]
    public_key: String,
    salt: String,
    algorithm: String,
    signature: String,
}

impl ApiKeyStamper {
    /// Create a new ApiKeyStamper from a configuration.
    ///
    /// # Errors
    /// Returns an error if the API secret key cannot be decoded.
    pub fn new(config: ApiKeyStamperConfig) -> Result<Self, phantom_crypto::CryptoError> {
        let keypair = create_key_pair_from_secret(&config.api_secret_key)?;
        Ok(Self { keypair })
    }

    /// Get the base64url-encoded public key.
    fn public_key_base64url(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let pk_bytes = bs58::decode(&self.keypair.public_key).into_vec()?;
        Ok(base64url_encode(&pk_bytes))
    }
}

#[async_trait::async_trait]
impl Stamper for ApiKeyStamper {
    /// Create X-Phantom-Stamp header value.
    ///
    /// Signs the provided data and returns a base64url-encoded JSON stamp
    /// containing the signature, public key, and algorithm metadata.
    async fn stamp(&self, params: StampParams) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let (data, stamp_type, id_token, salt) = match &params {
            StampParams::Pki { data } => (data.as_slice(), StamperType::Pki, None, None),
            StampParams::Oidc { data, id_token, salt } => {
                (data.as_slice(), StamperType::Oidc, Some(id_token.as_str()), Some(salt.as_str()))
            }
        };

        // Sign the data
        let signature = sign_with_secret(&SecretKeyInput::Base58(&self.keypair.secret_key), data)?;
        let signature_base64url = base64url_encode(&signature);
        let public_key_b64 = self.public_key_base64url()?;

        // Create the stamp structure based on stamp type
        let stamp_json = match stamp_type {
            StamperType::Pki => {
                let stamp = PkiStampData {
                    public_key: public_key_b64,
                    signature: signature_base64url,
                    kind: "PKI".to_string(),
                    algorithm: "ed25519".to_string(),
                };
                serde_json::to_string(&stamp)?
            }
            StamperType::Oidc => {
                let stamp = OidcStampData {
                    kind: "OIDC".to_string(),
                    id_token: id_token.unwrap_or_default().to_string(),
                    public_key: public_key_b64,
                    salt: salt.unwrap_or_default().to_string(),
                    algorithm: "ed25519".to_string(),
                    signature: signature_base64url,
                };
                serde_json::to_string(&stamp)?
            }
        };

        // Encode the entire stamp as base64url JSON
        Ok(base64url_encode(stamp_json.as_bytes()))
    }

    fn algorithm(&self) -> Algorithm {
        DEFAULT_AUTHENTICATOR_ALGORITHM
    }

    fn stamper_type(&self) -> StamperType {
        StamperType::Pki
    }
}
