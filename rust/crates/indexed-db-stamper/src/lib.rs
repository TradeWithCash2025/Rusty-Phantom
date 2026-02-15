//! IndexedDB-based key manager for browser environments.
//!
//! In the TypeScript SDK, this uses the Web Crypto API to generate non-extractable
//! keypairs stored in IndexedDB. Private keys never exist in JavaScript memory.
//!
//! In Rust, this module provides:
//! - The `IndexedDbStamperConfig` struct matching TS configuration
//! - The `IndexedDbStamper` struct implementing `StamperWithKeyManagement`
//! - A `KeyPairRecord` representing stored key state
//! - A `CryptoBackend` trait for platform-specific Web Crypto operations
//!
//! The actual Web Crypto / IndexedDB calls must be provided by a platform adapter
//! (e.g., via wasm-bindgen when targeting wasm32).

use phantom_sdk_types::{
    Algorithm, StampParams, Stamper, StamperKeyInfo, StamperType, StamperWithKeyManagement,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Web Crypto algorithm configuration.
#[derive(Debug, Clone)]
pub struct WebCryptoAlgorithmConfig {
    pub generate_params_name: &'static str,
    pub generate_params_curve: Option<&'static str>,
    pub sign_params_name: &'static str,
    pub sign_params_hash: Option<&'static str>,
}

/// Algorithm configs matching the TypeScript WEB_CRYPTO_ALGORITHM_CONFIGS.
pub fn web_crypto_algorithm_configs() -> Vec<(Algorithm, WebCryptoAlgorithmConfig)> {
    vec![
        (
            Algorithm::Ed25519,
            WebCryptoAlgorithmConfig {
                generate_params_name: "Ed25519",
                generate_params_curve: None,
                sign_params_name: "Ed25519",
                sign_params_hash: None,
            },
        ),
        (
            Algorithm::Secp256r1,
            WebCryptoAlgorithmConfig {
                generate_params_name: "ECDSA",
                generate_params_curve: Some("P-256"),
                sign_params_name: "ECDSA",
                sign_params_hash: Some("SHA-256"),
            },
        ),
    ]
}

/// Status of a stored key pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyPairStatus {
    Active,
    Pending,
    Expired,
}

/// Record stored in IndexedDB for a key pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPairRecord {
    pub key_info: StamperKeyInfo,
    pub created_at: u64,
    pub expires_at: u64,
    pub authenticator_id: Option<String>,
    pub status: KeyPairStatus,
}

/// Configuration for the IndexedDB stamper.
#[derive(Debug, Clone)]
pub struct IndexedDbStamperConfig {
    pub db_name: String,
    pub store_name: String,
    pub key_name: String,
    pub stamper_type: StamperType,
    pub id_token: Option<String>,
    pub salt: Option<String>,
}

impl Default for IndexedDbStamperConfig {
    fn default() -> Self {
        Self {
            db_name: "phantom-indexed-db-stamper".to_string(),
            store_name: "crypto-keys".to_string(),
            key_name: "signing-key".to_string(),
            stamper_type: StamperType::Pki,
            id_token: None,
            salt: None,
        }
    }
}

/// Trait for platform-specific cryptographic operations.
///
/// In WASM environments, this would be implemented using `web_sys::SubtleCrypto`
/// and `web_sys::IdbDatabase`. In native environments, a different backend
/// (e.g., ring or ed25519-dalek) could be used.
#[async_trait::async_trait]
pub trait CryptoBackend: Send + Sync {
    /// Generate a new key pair for the given algorithm.
    /// Returns (public_key_bytes, opaque_key_handle).
    async fn generate_key_pair(
        &self,
        algorithm: Algorithm,
    ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>>;

    /// Sign data with the given key handle.
    async fn sign(
        &self,
        algorithm: Algorithm,
        key_handle: &[u8],
        data: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

    /// Check if an algorithm is supported.
    async fn is_algorithm_supported(&self, algorithm: Algorithm) -> bool;
}

/// Trait for persistent key storage.
///
/// In browser environments, this is backed by IndexedDB.
/// In other environments, it could use filesystem, keychain, etc.
#[async_trait::async_trait]
pub trait KeyStorage: Send + Sync {
    /// Store a key pair record.
    async fn store(
        &self,
        key: &str,
        record: &KeyPairRecord,
        key_handle: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Load a key pair record and its handle.
    async fn load(
        &self,
        key: &str,
    ) -> Result<Option<(KeyPairRecord, Vec<u8>)>, Box<dyn std::error::Error + Send + Sync>>;

    /// Remove a key pair record.
    async fn remove(
        &self,
        key: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Interior mutable state for the stamper.
struct StamperState {
    algorithm: Algorithm,
    active_record: Option<(KeyPairRecord, Vec<u8>)>,
    pending_record: Option<(KeyPairRecord, Vec<u8>)>,
}

/// IndexedDB-based stamper that manages cryptographic keys.
///
/// Algorithm selection:
/// - Prefers Ed25519 for maximum security and performance
/// - Falls back to ECDSA P-256 on platforms that don't support Ed25519
///
/// Security model:
/// - Generates non-extractable keypairs using Web Crypto API (in browser)
/// - Private keys NEVER exist in application memory (in browser)
///
/// Uses interior mutability via `RwLock` so that trait methods taking `&self`
/// can still modify internal state.
pub struct IndexedDbStamper {
    config: IndexedDbStamperConfig,
    crypto: Box<dyn CryptoBackend>,
    storage: Box<dyn KeyStorage>,
    state: RwLock<StamperState>,
}

impl IndexedDbStamper {
    /// Create a new IndexedDB stamper.
    pub fn new(
        config: IndexedDbStamperConfig,
        crypto: Box<dyn CryptoBackend>,
        storage: Box<dyn KeyStorage>,
    ) -> Self {
        Self {
            config,
            crypto,
            storage,
            state: RwLock::new(StamperState {
                algorithm: Algorithm::Ed25519,
                active_record: None,
                pending_record: None,
            }),
        }
    }

    async fn get_supported_algorithm(
        &self,
    ) -> Result<Algorithm, Box<dyn std::error::Error + Send + Sync>> {
        for (alg, _) in web_crypto_algorithm_configs() {
            if self.crypto.is_algorithm_supported(alg).await {
                return Ok(alg);
            }
        }
        Err("No supported cryptographic algorithm found".into())
    }

    async fn generate_and_store_key_pair(
        &self,
        algorithm: Algorithm,
        status: KeyPairStatus,
    ) -> Result<(KeyPairRecord, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
        let (public_key_bytes, key_handle) =
            self.crypto.generate_key_pair(algorithm).await?;

        let public_key_base58 = bs58::encode(&public_key_bytes).into_string();

        // Create deterministic key ID from public key hash
        let key_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            public_key_bytes.hash(&mut hasher);
            format!("{:016x}", hasher.finish())
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let key_info = StamperKeyInfo {
            key_id,
            public_key: public_key_base58,
            created_at: Some(now),
            authenticator_id: None,
        };

        let record = KeyPairRecord {
            key_info,
            created_at: now,
            expires_at: 0,
            authenticator_id: None,
            status,
        };

        let storage_key = match status {
            KeyPairStatus::Active => format!("{}-active", self.config.key_name),
            KeyPairStatus::Pending => format!("{}-pending", self.config.key_name),
            KeyPairStatus::Expired => format!("{}-expired", self.config.key_name),
        };

        self.storage
            .store(&storage_key, &record, &key_handle)
            .await?;

        Ok((record, key_handle))
    }
}

/// Implement the Stamper trait (required by StamperWithKeyManagement).
#[async_trait::async_trait]
impl Stamper for IndexedDbStamper {
    async fn stamp(
        &self,
        params: StampParams,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let state = self.state.read().await;
        let (record, key_handle) = state
            .active_record
            .as_ref()
            .ok_or("Stamper not initialized. Call init() first.")?;

        let data = match &params {
            StampParams::Pki { data } => data,
            StampParams::Oidc { data, .. } => data,
        };

        let algorithm = state.algorithm;
        let key_handle = key_handle.clone();
        let record_clone = record.clone();
        drop(state);

        let signature = self
            .crypto
            .sign(algorithm, &key_handle, data)
            .await?;

        let signature_base64url = phantom_base64url::base64url_encode(&signature);

        let pub_key_bytes = bs58::decode(&record_clone.key_info.public_key).into_vec()?;
        let pub_key_base64url = phantom_base64url::base64url_encode(&pub_key_bytes);

        let stamp_data = match self.config.stamper_type {
            StamperType::Pki => {
                serde_json::json!({
                    "publicKey": pub_key_base64url,
                    "signature": signature_base64url,
                    "kind": "PKI",
                    "algorithm": format!("{:?}", algorithm),
                })
            }
            StamperType::Oidc => {
                let (id_token, salt) = match &params {
                    StampParams::Oidc {
                        id_token, salt, ..
                    } => (id_token.clone(), salt.clone()),
                    _ => (
                        self.config.id_token.clone().unwrap_or_default(),
                        self.config.salt.clone().unwrap_or_default(),
                    ),
                };
                serde_json::json!({
                    "kind": "OIDC",
                    "idToken": id_token,
                    "publicKey": pub_key_base64url,
                    "salt": salt,
                    "algorithm": format!("{:?}", algorithm),
                    "signature": signature_base64url,
                })
            }
        };

        let stamp_json = serde_json::to_string(&stamp_data)?;
        Ok(phantom_base64url::base64url_encode(stamp_json.as_bytes()))
    }

    fn algorithm(&self) -> Algorithm {
        // This is a sync method; use try_read to avoid blocking.
        // Falls back to Ed25519 if lock is held.
        self.state
            .try_read()
            .map(|s| s.algorithm)
            .unwrap_or(Algorithm::Ed25519)
    }

    fn stamper_type(&self) -> StamperType {
        self.config.stamper_type
    }

    fn id_token(&self) -> Option<&str> {
        self.config.id_token.as_deref()
    }

    fn salt(&self) -> Option<&str> {
        self.config.salt.as_deref()
    }
}

/// Implement the StamperWithKeyManagement trait.
#[async_trait::async_trait]
impl StamperWithKeyManagement for IndexedDbStamper {
    async fn init(
        &self,
    ) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Try to load existing active keypair
        let active_key = format!("{}-active", self.config.key_name);
        if let Some(record) = self.storage.load(&active_key).await? {
            let key_info = record.0.key_info.clone();
            let mut state = self.state.write().await;
            state.active_record = Some(record);
            drop(state);

            // Check for pending keypair from previous rotation
            let pending_key = format!("{}-pending", self.config.key_name);
            if let Some(pending) = self.storage.load(&pending_key).await? {
                let mut state = self.state.write().await;
                state.pending_record = Some(pending);
            }

            return Ok(key_info);
        }

        // Determine best supported algorithm
        let algorithm = self.get_supported_algorithm().await?;
        {
            let mut state = self.state.write().await;
            state.algorithm = algorithm;
        }

        // Generate new key pair
        let record = self
            .generate_and_store_key_pair(algorithm, KeyPairStatus::Active)
            .await?;
        let key_info = record.0.key_info.clone();
        {
            let mut state = self.state.write().await;
            state.active_record = Some(record);
        }

        // Check for pending keypair from previous rotation
        let pending_key = format!("{}-pending", self.config.key_name);
        if let Some(pending) = self.storage.load(&pending_key).await? {
            let mut state = self.state.write().await;
            state.pending_record = Some(pending);
        }

        Ok(key_info)
    }

    fn get_key_info(&self) -> Option<StamperKeyInfo> {
        self.state
            .try_read()
            .ok()
            .and_then(|s| s.active_record.as_ref().map(|(r, _)| r.key_info.clone()))
    }

    async fn reset_key_pair(
        &self,
    ) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>> {
        self.clear().await?;
        let algorithm = {
            let state = self.state.read().await;
            state.algorithm
        };
        let record = self
            .generate_and_store_key_pair(algorithm, KeyPairStatus::Active)
            .await?;
        let key_info = record.0.key_info.clone();
        {
            let mut state = self.state.write().await;
            state.active_record = Some(record);
        }
        Ok(key_info)
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let active_key = format!("{}-active", self.config.key_name);
        let pending_key = format!("{}-pending", self.config.key_name);
        self.storage.remove(&active_key).await?;
        self.storage.remove(&pending_key).await?;
        let mut state = self.state.write().await;
        state.active_record = None;
        state.pending_record = None;
        Ok(())
    }

    async fn rotate_key_pair(
        &self,
    ) -> Result<StamperKeyInfo, Box<dyn std::error::Error + Send + Sync>> {
        let algorithm = {
            let state = self.state.read().await;
            state.algorithm
        };
        let record = self
            .generate_and_store_key_pair(algorithm, KeyPairStatus::Pending)
            .await?;
        let key_info = record.0.key_info.clone();
        {
            let mut state = self.state.write().await;
            state.pending_record = Some(record);
        }
        Ok(key_info)
    }

    async fn commit_rotation(
        &self,
        authenticator_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (mut record, handle) = {
            let mut state = self.state.write().await;
            state
                .pending_record
                .take()
                .ok_or("No pending keypair to commit")?
        };

        // Remove old active
        let active_key = format!("{}-active", self.config.key_name);
        self.storage.remove(&active_key).await?;

        // Promote pending to active
        record.status = KeyPairStatus::Active;
        record.authenticator_id = Some(authenticator_id.to_string());
        record.key_info.authenticator_id = Some(authenticator_id.to_string());
        self.storage.store(&active_key, &record, &handle).await?;

        {
            let mut state = self.state.write().await;
            state.active_record = Some((record, handle));
        }

        // Remove pending record from storage
        let pending_key = format!("{}-pending", self.config.key_name);
        self.storage.remove(&pending_key).await?;

        Ok(())
    }

    async fn rollback_rotation(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let has_pending = {
            let state = self.state.read().await;
            state.pending_record.is_some()
        };

        if has_pending {
            let pending_key = format!("{}-pending", self.config.key_name);
            self.storage.remove(&pending_key).await?;
            let mut state = self.state.write().await;
            state.pending_record = None;
        }
        Ok(())
    }
}
