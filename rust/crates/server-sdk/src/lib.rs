//! Server-side SDK for Phantom wallet integration.
//!
//! Provides a high-level interface for server applications to interact
//! with Phantom wallets, including signing messages, signing/sending
//! transactions, and managing organizations and wallets.

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_base64url::{base64url_encode, string_to_base64url};
use phantom_client::{
    CreateWalletResult, GetWalletsResult, PhantomClient, PhantomClientConfig,
    WalletAddress,
};
use phantom_constants::{
    analytics::headers as analytics_headers, DEFAULT_WALLET_API_URL,
};
use phantom_parsers::{
    parse_sign_message_response, parse_transaction_response, ParsedSignatureResult,
    ParsedTransactionResult,
};
use phantom_utils::{get_secure_timestamp_sync, is_ethereum_chain, random_uuid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// SDK version constant.
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// Types
// ============================================================================

/// Configuration for the server SDK.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSdkConfig {
    /// Organization ID.
    pub organization_id: String,
    /// Application ID.
    pub app_id: String,
    /// API base URL (defaults to Phantom production URL).
    pub api_base_url: Option<String>,
    /// API private key (base58 encoded secret key).
    pub api_private_key: String,
    /// Solana RPC URL (optional, for direct RPC calls).
    pub solana_rpc_url: Option<String>,
}

/// Parameters for signing a message on the server.
#[derive(Debug, Clone)]
pub struct ServerSignMessageParams {
    /// Wallet ID.
    pub wallet_id: String,
    /// Plain text message (automatically converted to base64url).
    pub message: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
}

/// Parameters for signing a transaction on the server.
#[derive(Debug, Clone)]
pub struct ServerSignTransactionParams {
    /// Wallet ID.
    pub wallet_id: String,
    /// Encoded transaction string (base64url for Solana, hex for EVM).
    pub transaction: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
    /// Optional specific account address for simulation.
    pub account: Option<String>,
}

/// Parameters for signing and sending a transaction on the server.
#[derive(Debug, Clone)]
pub struct ServerSignAndSendTransactionParams {
    /// Wallet ID.
    pub wallet_id: String,
    /// Encoded transaction string (base64url for Solana, hex for EVM).
    pub transaction: String,
    /// Network ID (CAIP-2).
    pub network_id: String,
    /// Optional account derivation index (defaults to 0).
    pub derivation_index: Option<u32>,
    /// Optional specific account address for simulation.
    pub account: Option<String>,
}

// ============================================================================
// Analytics headers
// ============================================================================

/// Create server SDK analytics headers.
fn create_server_sdk_headers(app_id: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        analytics_headers::SDK_TYPE.to_string(),
        "server".to_string(),
    );
    headers.insert(
        analytics_headers::SDK_VERSION.to_string(),
        SDK_VERSION.to_string(),
    );
    headers.insert(
        analytics_headers::PLATFORM.to_string(),
        "rust".to_string(),
    );
    headers.insert(
        analytics_headers::PLATFORM_VERSION.to_string(),
        "1.0".to_string(),
    );
    headers.insert(
        analytics_headers::APP_ID.to_string(),
        app_id.to_string(),
    );
    headers
}

// ============================================================================
// ServerSDK
// ============================================================================

/// Server-side SDK for Phantom wallet integration.
///
/// Wraps a `PhantomClient` with an `ApiKeyStamper` for server-to-server
/// authentication. Provides high-level methods for signing messages,
/// signing/sending transactions, and managing organizations and wallets.
pub struct ServerSdk {
    config: ServerSdkConfig,
    /// The underlying PhantomClient (public for advanced usage).
    pub client: PhantomClient,
}

impl ServerSdk {
    /// Create a new ServerSDK instance.
    pub fn new(config: ServerSdkConfig) -> Self {
        // Create stamper - panics if the API key is invalid
        let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
            api_secret_key: config.api_private_key.clone(),
        })
        .expect("Invalid API private key");

        let headers = create_server_sdk_headers(&config.app_id);

        let api_base_url = config
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_WALLET_API_URL.to_string());

        let client = PhantomClient::new(
            PhantomClientConfig {
                api_base_url,
                organization_id: Some(config.organization_id.clone()),
                headers: Some(headers),
                wallet_type: "server-wallet".to_string(),
            },
            Some(Arc::new(stamper)),
        );

        Self { config, client }
    }

    /// Sign a message — supports plain text and automatically converts to base64url.
    ///
    /// Routes to the appropriate signing method based on network type
    /// (Ethereum personal_sign vs Solana signUtf8Message).
    pub async fn sign_message(
        &self,
        params: ServerSignMessageParams,
    ) -> Result<ParsedSignatureResult, phantom_client::ClientError> {
        let raw_response = if is_ethereum_chain(&params.network_id) {
            self.client
                .ethereum_sign_message(&phantom_client::SignMessageParams {
                    wallet_id: params.wallet_id,
                    message: string_to_base64url(&params.message),
                    network_id: params.network_id.clone(),
                    derivation_index: params.derivation_index,
                })
                .await?
        } else {
            self.client
                .sign_utf8_message(&phantom_client::SignMessageParams {
                    wallet_id: params.wallet_id,
                    message: params.message,
                    network_id: params.network_id.clone(),
                    derivation_index: params.derivation_index,
                })
                .await?
        };

        let network_id: NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id))
                .map_err(|e| phantom_client::ClientError::Config(format!("Invalid network ID: {}", e)))?;

        Ok(parse_sign_message_response(&raw_response, network_id))
    }

    /// Sign a transaction — transaction should already be encoded (base64url for Solana, hex for EVM).
    pub async fn sign_transaction(
        &self,
        params: ServerSignTransactionParams,
    ) -> Result<ParsedTransactionResult, phantom_client::ClientError> {
        let raw_response = self
            .client
            .sign_transaction(&phantom_client::SignTransactionParams {
                wallet_id: params.wallet_id,
                transaction: params.transaction,
                network_id: params.network_id.clone(),
                derivation_index: params.derivation_index,
                account: params.account,
            })
            .await?;

        let network_id: NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id))
                .map_err(|e| phantom_client::ClientError::Config(format!("Invalid network ID: {}", e)))?;

        Ok(parse_transaction_response(
            &raw_response.raw_transaction,
            network_id,
            None,
        ))
    }

    /// Sign and send a transaction.
    pub async fn sign_and_send_transaction(
        &self,
        params: ServerSignAndSendTransactionParams,
    ) -> Result<ParsedTransactionResult, phantom_client::ClientError> {
        let raw_response = self
            .client
            .sign_and_send_transaction(&phantom_client::SignAndSendTransactionParams {
                wallet_id: params.wallet_id,
                transaction: params.transaction,
                network_id: params.network_id.clone(),
                derivation_index: params.derivation_index,
                account: params.account,
            })
            .await?;

        let network_id: NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id))
                .map_err(|e| phantom_client::ClientError::Config(format!("Invalid network ID: {}", e)))?;

        Ok(parse_transaction_response(
            &raw_response.raw_transaction,
            network_id,
            raw_response.hash.as_deref(),
        ))
    }

    /// Create a new organization.
    pub async fn create_organization(
        &self,
        name: &str,
        key_pair: &phantom_crypto::Keypair,
    ) -> Result<serde_json::Value, phantom_client::ClientError> {
        let headers = create_server_sdk_headers(&self.config.app_id);

        let api_base_url = self
            .config
            .api_base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_WALLET_API_URL.to_string());

        let temp_stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
            api_secret_key: key_pair.secret_key.clone(),
        })
        .map_err(|e| phantom_client::ClientError::Config(format!("Invalid key: {}", e)))?;

        let temp_client = PhantomClient::new(
            PhantomClientConfig {
                api_base_url,
                organization_id: Some(self.config.organization_id.clone()),
                headers: Some(headers),
                wallet_type: "server-wallet".to_string(),
            },
            Some(Arc::new(temp_stamper)),
        );

        let base64url_public_key =
            base64url_encode(&bs58::decode(&key_pair.public_key).into_vec().unwrap_or_default());

        temp_client
            .create_organization(
                name,
                &[phantom_client::UserConfig {
                    username: format!("user-{}", random_uuid()),
                    role: Some("ADMIN".to_string()),
                    authenticators: vec![phantom_client::AuthenticatorConfig::Keypair {
                        authenticator_name: format!("auth-{}", get_secure_timestamp_sync()),
                        public_key: base64url_public_key,
                        algorithm: phantom_client::ClientAlgorithm::Ed25519,
                        expires_in_ms: None,
                    }],
                }],
                None,
            )
            .await
    }

    /// Get wallets for the current organization.
    pub async fn get_wallets(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<GetWalletsResult, phantom_client::ClientError> {
        self.client.get_wallets(limit, offset).await
    }

    /// Create a new wallet.
    pub async fn create_wallet(
        &self,
        name: &str,
    ) -> Result<CreateWalletResult, phantom_client::ClientError> {
        self.client.create_wallet(Some(name)).await
    }

    /// Get wallet addresses.
    pub async fn get_wallet_addresses(
        &self,
        wallet_id: &str,
        derivation_paths: Option<&[String]>,
        derivation_index: Option<u32>,
    ) -> Result<Vec<WalletAddress>, phantom_client::ClientError> {
        self.client
            .get_wallet_addresses(wallet_id, derivation_paths, derivation_index)
            .await
    }
}

// Re-export from dependencies for convenience (matches TS re-exports)
pub use phantom_client::{
    self,
    derive_submission_config, get_client_network_config, get_derivation_path_for_network,
    get_network_description, get_network_ids_by_chain, get_supported_network_ids,
    supports_transaction_submission, AddressFormat, ClientAlgorithm, ClientNetworkConfig,
    Curve, DerivationPath, NetworkId as ClientNetworkId,
    SignedTransactionResult, Transaction, Wallet,
};
pub use phantom_api_key_stamper::{self as api_key_stamper, ApiKeyStamper as ApiKeyStamperExport};
pub use phantom_constants::NetworkId;
pub use phantom_crypto::generate_key_pair;
