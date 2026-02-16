//! Main embedded provider implementation.
//!
//! Mirrors the TypeScript `EmbeddedProvider` class from
//! `packages/embedded-provider-core/src/embedded-provider.ts`.

use phantom_base64url::string_to_base64url;
use phantom_client::{AddressFormat, PhantomClient, PhantomClientConfig};
use phantom_sdk_types::StamperWithKeyManagement;
use phantom_parsers::{
    parse_sign_message_response, parse_transaction_response, ParsedSignatureResult,
    ParsedTransactionResult,
};
use phantom_utils::network::get_chain_prefix;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants::{AUTHENTICATOR_EXPIRATION_TIME_MS, EMBEDDED_PROVIDER_AUTH_TYPES};
use crate::interfaces::*;
use crate::types::*;
use crate::utils::generate_session_id;

/// Events emitted by the embedded provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddedProviderEvent {
    /// Connection established.
    Connect,
    /// Connection starting.
    ConnectStart,
    /// Connection error.
    ConnectError,
    /// Disconnected.
    Disconnect,
    /// General error.
    Error,
    /// Spending limit reached.
    SpendingLimitReached,
}

/// Event callback type.
pub type EventCallback = Box<dyn Fn(serde_json::Value) + Send + Sync>;

/// The embedded wallet provider.
///
/// Manages connection lifecycle, authentication, signing operations,
/// and event emission for the embedded wallet.
pub struct EmbeddedProvider {
    config: EmbeddedProviderConfig,
    platform: Arc<dyn PlatformAdapter>,
    logger: Arc<dyn DebugLogger>,
    client: RwLock<Option<PhantomClient>>,
    wallet_id: RwLock<Option<String>>,
    addresses: RwLock<Vec<WalletAddress>>,
    event_listeners: RwLock<HashMap<EmbeddedProviderEvent, Vec<Arc<EventCallback>>>>,
}

impl EmbeddedProvider {
    /// Create a new embedded provider.
    pub fn new(
        config: EmbeddedProviderConfig,
        platform: Arc<dyn PlatformAdapter>,
        logger: Arc<dyn DebugLogger>,
    ) -> Result<Self, String> {
        logger.log(
            "EMBEDDED_PROVIDER",
            "Initializing EmbeddedProvider",
            None,
        );

        if config.embedded_wallet_type == "app-wallet" {
            return Err(
                "app-wallet type is not currently supported. Please use 'user-wallet' instead."
                    .to_string(),
            );
        }

        logger.info("EMBEDDED_PROVIDER", "EmbeddedProvider initialized", None);

        Ok(Self {
            config,
            platform,
            logger,
            client: RwLock::new(None),
            wallet_id: RwLock::new(None),
            addresses: RwLock::new(Vec::new()),
            event_listeners: RwLock::new(HashMap::new()),
        })
    }

    /// Register an event listener.
    pub async fn on(&self, event: EmbeddedProviderEvent, callback: Arc<EventCallback>) {
        let mut listeners = self.event_listeners.write().await;
        listeners
            .entry(event)
            .or_insert_with(Vec::new)
            .push(callback);
    }

    /// Remove an event listener.
    pub async fn off(&self, event: EmbeddedProviderEvent, callback_id: usize) {
        let mut listeners = self.event_listeners.write().await;
        if let Some(cbs) = listeners.get_mut(&event) {
            if callback_id < cbs.len() {
                cbs.remove(callback_id);
            }
        }
    }

    /// Emit an event to all listeners.
    async fn emit(&self, event: EmbeddedProviderEvent, data: serde_json::Value) {
        let listeners = self.event_listeners.read().await;
        if let Some(cbs) = listeners.get(&event) {
            for callback in cbs {
                callback(data.clone());
            }
        }
    }

    /// Check if the provider is connected.
    pub async fn is_connected(&self) -> bool {
        let client = self.client.read().await;
        let wallet_id = self.wallet_id.read().await;
        client.is_some() && wallet_id.is_some()
    }

    /// Get the current wallet addresses.
    pub async fn get_addresses(&self) -> Vec<WalletAddress> {
        self.addresses.read().await.clone()
    }

    /// Auto-connect using an existing valid session.
    pub async fn auto_connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger
            .log("EMBEDDED_PROVIDER", "Starting auto-connect attempt", None);

        self.emit(
            EmbeddedProviderEvent::ConnectStart,
            serde_json::json!({ "source": "auto-connect" }),
        )
        .await;

        match self.try_existing_connection(true).await {
            Ok(Some(result)) => {
                self.logger.info(
                    "EMBEDDED_PROVIDER",
                    "Auto-connect successful",
                    Some(&serde_json::json!({
                        "walletId": result.wallet_id,
                        "addressCount": result.addresses.len(),
                    })),
                );

                self.emit(
                    EmbeddedProviderEvent::Connect,
                    serde_json::json!({ "source": "auto-connect" }),
                )
                .await;
                Ok(())
            }
            Ok(None) => {
                self.logger.log(
                    "EMBEDDED_PROVIDER",
                    "Auto-connect failed: no valid session found",
                    None,
                );
                self.emit(
                    EmbeddedProviderEvent::ConnectError,
                    serde_json::json!({
                        "error": "No valid session found",
                        "source": "auto-connect",
                    }),
                )
                .await;
                Ok(())
            }
            Err(e) => {
                self.logger.error(
                    "EMBEDDED_PROVIDER",
                    "Auto-connect failed",
                    Some(&serde_json::json!({ "error": e.to_string() })),
                );
                self.emit(
                    EmbeddedProviderEvent::ConnectError,
                    serde_json::json!({
                        "error": e.to_string(),
                        "source": "auto-connect",
                    }),
                )
                .await;

                let _ = self
                    .platform
                    .storage()
                    .set_should_clear_previous_session(true)
                    .await;
                Err(e)
            }
        }
    }

    /// Connect with authentication options.
    pub async fn connect(
        &self,
        auth_options: AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.connect_inner(&auth_options).await;

        match result {
            Ok(r) => Ok(r),
            Err(e) => {
                // Log the full error details for debugging
                self.logger.error(
                    "EMBEDDED_PROVIDER",
                    "Connect failed with error",
                    Some(&serde_json::json!({ "error": e.to_string() })),
                );

                // Emit connect_error event for manual connect failure
                self.emit(
                    EmbeddedProviderEvent::ConnectError,
                    serde_json::json!({
                        "error": e.to_string(),
                        "source": "manual-connect",
                    }),
                )
                .await;

                // Enhanced error handling with specific error types
                let msg = e.to_string();

                if msg.contains("IndexedDB") || msg.contains("storage") {
                    return Err("Storage error: Unable to access browser storage. Please ensure storage is available and try again.".into());
                }

                if msg.contains("network") || msg.contains("fetch") {
                    return Err("Network error: Unable to connect to authentication server. Please check your internet connection and try again.".into());
                }

                if msg.contains("JWT") || msg.contains("jwt") {
                    return Err(format!("JWT Authentication error: {}", msg).into());
                }

                if msg.contains("Authentication") || msg.contains("auth") {
                    return Err(format!("Authentication error: {}", msg).into());
                }

                if msg.contains("organization") || msg.contains("wallet") {
                    return Err(format!("Wallet creation error: {}", msg).into());
                }

                Err(e)
            }
        }
    }

    /// Inner connect logic, separated to allow error categorization in the outer method.
    async fn connect_inner(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info(
            "EMBEDDED_PROVIDER",
            "Starting embedded provider connect",
            Some(&serde_json::json!({
                "provider": format!("{:?}", auth_options.provider),
            })),
        );

        self.emit(
            EmbeddedProviderEvent::ConnectStart,
            serde_json::json!({
                "source": "manual-connect",
                "authOptions": { "provider": format!("{:?}", auth_options.provider) },
            }),
        )
        .await;

        // Try existing connection first
        if let Some(result) = self.try_existing_connection(false).await? {
            self.emit(
                EmbeddedProviderEvent::Connect,
                serde_json::json!({ "source": "manual-existing" }),
            )
            .await;
            return Ok(result);
        }

        // Validate auth options
        self.validate_auth_options(auth_options)?;

        // No existing connection, create new one
        self.logger.info(
            "EMBEDDED_PROVIDER",
            "No existing connection, creating new auth flow",
            None,
        );

        // Initialize stamper
        let stamper = self.platform.stamper();
        stamper.init().await?;
        let stamper_info = stamper.reset_key_pair().await?;

        let stamper_info_data = StamperInfo {
            key_id: stamper_info.key_id.clone(),
            public_key: stamper_info.public_key.clone(),
            created_at: stamper_info.created_at,
            authenticator_id: stamper_info.authenticator_id.clone(),
        };

        // Handle auth flow based on wallet type and provider
        let session = self
            .handle_auth_flow(
                &stamper_info.public_key,
                &stamper_info_data,
                auth_options,
                AUTHENTICATOR_EXPIRATION_TIME_MS,
            )
            .await?;

        match session {
            Some(session) => {
                self.initialize_client_from_session(&session).await?;

                let wallet_id = self.wallet_id.read().await;
                let addresses = self.addresses.read().await;

                let result = ConnectResult {
                    wallet_id: wallet_id.clone(),
                    addresses: addresses.clone(),
                    status: Some(ConnectStatus::Completed),
                    auth_user_id: session.auth_user_id.clone(),
                    auth_provider: session.auth_provider,
                };

                self.emit(
                    EmbeddedProviderEvent::Connect,
                    serde_json::json!({ "source": "manual" }),
                )
                .await;

                Ok(result)
            }
            None => {
                // Redirect in progress
                Ok(ConnectResult {
                    wallet_id: None,
                    addresses: vec![],
                    status: Some(ConnectStatus::Pending),
                    auth_user_id: None,
                    auth_provider: auth_options.provider,
                })
            }
        }
    }

    /// Disconnect from the embedded wallet.
    pub async fn disconnect(
        &self,
        should_clear_previous_session: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let was_connected = self.is_connected().await;

        let storage = self.platform.storage();
        storage
            .set_should_clear_previous_session(should_clear_previous_session)
            .await?;
        storage.clear_session().await?;

        *self.client.write().await = None;
        *self.wallet_id.write().await = None;
        *self.addresses.write().await = Vec::new();

        self.logger
            .info("EMBEDDED_PROVIDER", "Disconnected from embedded wallet", None);

        if was_connected {
            self.emit(
                EmbeddedProviderEvent::Disconnect,
                serde_json::json!({ "source": "manual" }),
            )
            .await;
        }

        Ok(())
    }

    /// Sign a message.
    pub async fn sign_message(
        &self,
        params: &SignMessageParams,
    ) -> Result<ParsedSignatureResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.read().await;
        let client = client
            .as_ref()
            .ok_or("Not connected")?;

        let wallet_id = self.wallet_id.read().await;
        let wallet_id = wallet_id.as_ref().ok_or("Not connected")?;

        // Check if authenticator needs renewal before performing the operation
        self.ensure_valid_authenticator().await?;

        let session = self.platform.storage().get_session().await?;
        let derivation_index = session
            .as_ref()
            .and_then(|s| s.account_derivation_index)
            .unwrap_or(0);

        let raw_response = client
            .sign_utf8_message(&phantom_client::SignMessageParams {
                wallet_id: wallet_id.clone(),
                message: params.message.clone(),
                network_id: params.network_id.clone(),
                derivation_index: Some(derivation_index),
            })
            .await?;

        let network_id: phantom_constants::NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id.clone()))
                .map_err(|e| format!("Invalid network ID: {}", e))?;

        Ok(parse_sign_message_response(&raw_response, network_id))
    }

    /// Sign and send a transaction.
    pub async fn sign_and_send_transaction(
        &self,
        params: &SignAndSendTransactionParams,
    ) -> Result<ParsedTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Not connected")?;

        let wallet_id = self.wallet_id.read().await;
        let wallet_id = wallet_id.as_ref().ok_or("Not connected")?;

        // Check if authenticator needs renewal before performing the operation
        self.ensure_valid_authenticator().await?;

        self.logger.info(
            "EMBEDDED_PROVIDER",
            "Signing and sending transaction",
            Some(&serde_json::json!({
                "walletId": wallet_id,
                "networkId": params.network_id,
            })),
        );

        // Parse transaction to KMS format (base64url for Solana, hex for EVM) based on network
        let parsed_transaction = phantom_parsers::parse_to_kms_transaction(
            phantom_parsers::TransactionInput::Bytes(params.transaction.clone()),
            &params.network_id,
        )
        .map_err(|e| format!("Failed to parse transaction: {}", e))?;

        let session = self.platform.storage().get_session().await?;
        let derivation_index = session
            .as_ref()
            .and_then(|s| s.account_derivation_index)
            .unwrap_or(0);

        let transaction_payload = parsed_transaction
            .parsed
            .ok_or("Failed to parse transaction: no valid encoding found")?;

        let account = self.get_address_for_network(&params.network_id).await;
        if account.is_none() {
            return Err(format!("No address found for network {}", params.network_id).into());
        }

        // Sign and send with spending limit error handling
        let raw_response = match client
            .sign_and_send_transaction(&phantom_client::SignAndSendTransactionParams {
                wallet_id: wallet_id.clone(),
                transaction: transaction_payload,
                network_id: params.network_id.clone(),
                derivation_index: Some(derivation_index),
                account,
            })
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // Normalize spending limit errors into a dedicated event while preserving the rejection
                if let phantom_client::ClientError::WalletService(
                    phantom_client::WalletServiceError::SpendingLimitExceeded { .. },
                ) = &e
                {
                    self.emit(
                        EmbeddedProviderEvent::SpendingLimitReached,
                        serde_json::json!({ "error": e.to_string() }),
                    )
                    .await;
                }
                return Err(e.into());
            }
        };

        let network_id: phantom_constants::NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id.clone()))
                .map_err(|e| format!("Invalid network ID: {}", e))?;

        Ok(parse_transaction_response(
            &raw_response.raw_transaction,
            network_id,
            raw_response.hash.as_deref(),
        ))
    }

    /// Sign a transaction without broadcasting it.
    pub async fn sign_transaction(
        &self,
        params: &SignTransactionParams,
    ) -> Result<ParsedTransactionResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Not connected")?;

        let wallet_id = self.wallet_id.read().await;
        let wallet_id = wallet_id.as_ref().ok_or("Not connected")?;

        // Check if authenticator needs renewal before performing the operation
        self.ensure_valid_authenticator().await?;

        self.logger.info(
            "EMBEDDED_PROVIDER",
            "Signing transaction",
            Some(&serde_json::json!({
                "walletId": wallet_id,
                "networkId": params.network_id,
            })),
        );

        // Parse transaction to KMS format (base64url for Solana, hex for EVM) based on network
        let parsed_transaction = phantom_parsers::parse_to_kms_transaction(
            phantom_parsers::TransactionInput::Bytes(params.transaction.clone()),
            &params.network_id,
        )
        .map_err(|e| format!("Failed to parse transaction: {}", e))?;

        let session = self.platform.storage().get_session().await?;
        let derivation_index = session
            .as_ref()
            .and_then(|s| s.account_derivation_index)
            .unwrap_or(0);

        let transaction_payload = parsed_transaction
            .parsed
            .ok_or("Failed to parse transaction: no valid encoding found")?;

        let account = self.get_address_for_network(&params.network_id).await;
        if account.is_none() {
            return Err(format!("No address found for network {}", params.network_id).into());
        }

        let raw_response = client
            .sign_transaction(&phantom_client::SignTransactionParams {
                wallet_id: wallet_id.clone(),
                transaction: transaction_payload,
                network_id: params.network_id.clone(),
                derivation_index: Some(derivation_index),
                account,
            })
            .await?;

        let network_id: phantom_constants::NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id.clone()))
                .map_err(|e| format!("Invalid network ID: {}", e))?;

        Ok(parse_transaction_response(
            &raw_response.raw_transaction,
            network_id,
            None,
        ))
    }

    /// Sign EIP-712 typed data (v4).
    pub async fn sign_typed_data_v4(
        &self,
        params: &SignTypedDataV4Params,
    ) -> Result<ParsedSignatureResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Not connected")?;

        let wallet_id = self.wallet_id.read().await;
        let wallet_id = wallet_id.as_ref().ok_or("Not connected")?;

        // Check if authenticator needs renewal before performing the operation
        self.ensure_valid_authenticator().await?;

        let session = self.platform.storage().get_session().await?;
        let derivation_index = session
            .as_ref()
            .and_then(|s| s.account_derivation_index)
            .unwrap_or(0);

        let raw_response = client
            .ethereum_sign_typed_data(&phantom_client::SignTypedDataParams {
                wallet_id: wallet_id.clone(),
                typed_data: params.typed_data.clone(),
                network_id: params.network_id.clone(),
                derivation_index: Some(derivation_index),
            })
            .await?;

        let network_id: phantom_constants::NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id.clone()))
                .map_err(|e| format!("Invalid network ID: {}", e))?;

        Ok(parse_sign_message_response(&raw_response, network_id))
    }

    /// Sign an Ethereum message using EIP-191 personal sign.
    ///
    /// Detects hex-encoded input (0x-prefixed) and normalizes it, otherwise
    /// converts the raw string to base64url before delegating to the client.
    pub async fn sign_ethereum_message(
        &self,
        params: &SignMessageParams,
    ) -> Result<ParsedSignatureResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or("Not connected")?;

        let wallet_id = self.wallet_id.read().await;
        let wallet_id = wallet_id.as_ref().ok_or("Not connected")?;

        // Check if authenticator needs renewal before performing the operation
        self.ensure_valid_authenticator().await?;

        self.logger.info(
            "EMBEDDED_PROVIDER",
            "Signing Ethereum message",
            Some(&serde_json::json!({
                "walletId": wallet_id,
                "message": params.message,
            })),
        );

        // Detect hex input (starts with 0x) and normalize
        let normalized_message = if params.message.starts_with("0x")
            && params.message[2..].chars().all(|c| c.is_ascii_hexdigit())
        {
            let hex_payload = &params.message[2..];
            // Ensure even-length hex string
            let padded = if hex_payload.len() % 2 != 0 {
                format!("0{}", hex_payload)
            } else {
                hex_payload.to_string()
            };
            // Decode hex manually (no hex crate dependency)
            let bytes: Vec<u8> = (0..padded.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&padded[i..i + 2], 16))
                .collect::<Result<Vec<u8>, _>>()
                .map_err(|e| format!("Invalid hex message: {}", e))?;
            String::from_utf8(bytes)
                .map_err(|e| format!("Hex message is not valid UTF-8: {}", e))?
        } else {
            params.message.clone()
        };

        // Convert to base64url format for the client
        let base64url_message = string_to_base64url(&normalized_message);

        let session = self.platform.storage().get_session().await?;
        let derivation_index = session
            .as_ref()
            .and_then(|s| s.account_derivation_index)
            .unwrap_or(0);

        let raw_response = client
            .ethereum_sign_message(&phantom_client::SignMessageParams {
                wallet_id: wallet_id.clone(),
                message: base64url_message,
                network_id: params.network_id.clone(),
                derivation_index: Some(derivation_index),
            })
            .await?;

        let network_id: phantom_constants::NetworkId =
            serde_json::from_value(serde_json::Value::String(params.network_id.clone()))
                .map_err(|e| format!("Invalid network ID: {}", e))?;

        Ok(parse_sign_message_response(&raw_response, network_id))
    }

    /// Ensures the authenticator is valid.
    ///
    /// If the authenticator has expired, disconnects the provider and returns
    /// an error.
    pub async fn ensure_valid_authenticator(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self.platform.storage().get_session().await?;
        let session = session.ok_or("No active session found")?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Sessions without authenticator timing fields are invalid
        if session.authenticator_expires_at == 0 {
            self.logger.warn(
                "EMBEDDED_PROVIDER",
                "Session missing authenticator timing - treating as invalid session",
                None,
            );
            self.disconnect(false).await?;
            return Err("Invalid session - missing authenticator timing".into());
        }

        let time_until_expiry = if session.authenticator_expires_at > now {
            session.authenticator_expires_at - now
        } else {
            0
        };

        self.logger.log(
            "EMBEDDED_PROVIDER",
            "Checking authenticator expiration",
            Some(&serde_json::json!({
                "expiresAt": session.authenticator_expires_at,
                "timeUntilExpiry": time_until_expiry,
            })),
        );

        // Check if authenticator has expired
        if time_until_expiry == 0 {
            self.logger.error(
                "EMBEDDED_PROVIDER",
                "Authenticator has expired, disconnecting",
                None,
            );
            self.disconnect(false).await?;
            return Err("Authenticator expired".into());
        }

        Ok(())
    }

    /// Validate and clean a session from storage.
    ///
    /// Reads the session from storage and validates it. Returns `None` and
    /// clears storage if the session is invalid (not completed, missing
    /// wallet_id/organization_id, or expired authenticator).
    pub async fn validate_and_clean_session(
        &self,
    ) -> Option<Session> {
        let session = match self.platform.storage().get_session().await {
            Ok(Some(s)) => s,
            _ => return None,
        };

        self.logger.log(
            "EMBEDDED_PROVIDER",
            "Found existing session, validating",
            Some(&serde_json::json!({
                "sessionId": session.session_id,
                "status": format!("{:?}", session.status),
                "walletId": session.wallet_id,
            })),
        );

        // For completed sessions, validate required fields and expiration
        if session.status == SessionStatus::Completed {
            if !self.is_session_valid(&session) {
                self.logger.warn(
                    "EMBEDDED_PROVIDER",
                    "Session invalid due to missing fields or authenticator expiration",
                    Some(&serde_json::json!({
                        "sessionId": session.session_id,
                        "authenticatorExpiresAt": session.authenticator_expires_at,
                    })),
                );
                let _ = self.platform.storage().clear_session().await;
                return None;
            }
            return Some(session);
        }

        // For non-completed sessions, check URL params for session_id context
        let url_session_id = self.platform.url_params_accessor().get_param("session_id");

        // If we have a pending session but no sessionId in URL, this is a mismatch
        if session.status == SessionStatus::Pending && url_session_id.is_none() {
            self.logger.warn(
                "EMBEDDED_PROVIDER",
                "Session mismatch detected - pending session without redirect context",
                Some(&serde_json::json!({
                    "sessionId": session.session_id,
                    "status": format!("{:?}", session.status),
                })),
            );
            let _ = self.platform.storage().clear_session().await;
            return None;
        }

        // If sessionId in URL doesn't match stored session, clear invalid session
        if let Some(ref url_sid) = url_session_id {
            if url_sid != &session.session_id {
                self.logger.warn(
                    "EMBEDDED_PROVIDER",
                    "Session ID mismatch detected",
                    Some(&serde_json::json!({
                        "storedSessionId": session.session_id,
                        "urlSessionId": url_sid,
                    })),
                );
                let _ = self.platform.storage().clear_session().await;
                return None;
            }
        }

        // Non-completed sessions that pass URL checks are returned as-is for redirect resume
        Some(session)
    }

    /// Get the appropriate address for a given network ID from available addresses.
    ///
    /// Maps CAIP-2 network ID prefix to address format:
    /// - "solana" -> AddressFormat::Solana
    /// - "eip155" -> AddressFormat::Ethereum
    /// - "sui" -> AddressFormat::Sui
    /// - "bitcoin"/"bip122" -> None (not yet supported for signing)
    fn get_address_for_network_sync(
        &self,
        network_id: &str,
        addresses: &[WalletAddress],
    ) -> Option<String> {
        let chain = get_chain_prefix(network_id);

        let target_format = match chain.as_str() {
            "solana" => Some(AddressFormat::Solana),
            "eip155" => Some(AddressFormat::Ethereum),
            "sui" => Some(AddressFormat::Sui),
            // bitcoin/bip122 not currently supported for signing
            "bitcoin" | "btc" | "bip122" => None,
            // Default to Ethereum for unknown networks
            _ => Some(AddressFormat::Ethereum),
        };

        let target_format = target_format?;

        addresses
            .iter()
            .find(|addr| addr.address_type == target_format)
            .map(|addr| addr.address.clone())
    }

    /// Get the appropriate address for a given network ID (async, reads from stored addresses).
    pub async fn get_address_for_network(&self, network_id: &str) -> Option<String> {
        let addresses = self.addresses.read().await;
        self.get_address_for_network_sync(network_id, &addresses)
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    async fn try_existing_connection(
        &self,
        is_auto_connect: bool,
    ) -> Result<Option<ConnectResult>, Box<dyn std::error::Error + Send + Sync>> {
        self.logger.log("EMBEDDED_PROVIDER", "Getting existing session", None);

        let storage = self.platform.storage();
        let session = match self.validate_and_clean_session().await {
            Some(s) => s,
            None => {
                self.logger.log("EMBEDDED_PROVIDER", "No existing session found", None);
                return Ok(None);
            }
        };

        // First priority: If we have a completed session, use it
        if session.status == SessionStatus::Completed {
            self.logger.info(
                "EMBEDDED_PROVIDER",
                "Using existing completed session",
                Some(&serde_json::json!({
                    "sessionId": session.session_id,
                    "walletId": session.wallet_id,
                })),
            );

            self.initialize_client_from_session(&session).await?;

            // Update session last_used timestamp and save
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let mut updated_session = session.clone();
            updated_session.last_used = now;
            storage.save_session(&updated_session).await?;

            self.logger.info(
                "EMBEDDED_PROVIDER",
                "Connection from existing session successful",
                Some(&serde_json::json!({
                    "walletId": updated_session.wallet_id,
                })),
            );

            // Ensure authenticator is valid after successful connection
            self.ensure_valid_authenticator().await?;

            let wallet_id = self.wallet_id.read().await;
            let addresses = self.addresses.read().await;

            let result = ConnectResult {
                wallet_id: wallet_id.clone(),
                addresses: addresses.clone(),
                status: Some(ConnectStatus::Completed),
                auth_user_id: updated_session.auth_user_id.clone(),
                auth_provider: updated_session.auth_provider,
            };

            self.emit(
                EmbeddedProviderEvent::Connect,
                serde_json::json!({
                    "source": "existing-session",
                    "authUserId": result.auth_user_id,
                    "authProvider": format!("{:?}", result.auth_provider),
                }),
            )
            .await;

            return Ok(Some(result));
        }

        // Second priority: Check if we're resuming from a redirect
        self.logger.log(
            "EMBEDDED_PROVIDER",
            "No completed session found, checking for redirect resume",
            None,
        );

        let auth_provider = self.platform.auth_provider();
        let resume_result = auth_provider.resume_auth_from_redirect(session.auth_provider)?;
        if let Some(auth_result) = resume_result {
            self.logger.info(
                "EMBEDDED_PROVIDER",
                "Resuming from redirect",
                Some(&serde_json::json!({
                    "walletId": auth_result.wallet_id,
                    "provider": format!("{:?}", auth_result.provider),
                })),
            );

            match self.complete_auth_connection(auth_result).await {
                Ok(result) => return Ok(Some(result)),
                Err(e) => {
                    let err_msg = e.to_string();
                    // Handle the edge case where session was wiped but URL has session params
                    if err_msg.contains("No session found after redirect") && !is_auto_connect {
                        self.logger.warn(
                            "EMBEDDED_PROVIDER",
                            "Session missing during redirect resume - will start fresh auth flow",
                            Some(&serde_json::json!({ "error": err_msg })),
                        );
                        storage.clear_session().await?;
                        return Ok(None);
                    }
                    return Err(e);
                }
            }
        }

        Ok(None)
    }

    /// Complete authentication after redirect resume.
    async fn complete_auth_connection(
        &self,
        auth_result: AuthResult,
    ) -> Result<ConnectResult, Box<dyn std::error::Error + Send + Sync>> {
        let storage = self.platform.storage();
        let session = storage.get_session().await?;

        let mut session = match session {
            Some(s) => s,
            None => return Err("No session found after redirect - session may have expired".into()),
        };

        // Update session with actual wallet ID and auth info from redirect
        session.wallet_id = auth_result.wallet_id;
        session.auth_provider = auth_result.provider;
        session.organization_id = auth_result.organization_id;
        session.account_derivation_index = Some(auth_result.account_derivation_index);
        session.auth_user_id = auth_result.auth_user_id;
        session.status = SessionStatus::Completed;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        session.last_used = now;

        // Update authenticator expiration if provided by auth response
        if auth_result.expires_in_ms > 0 {
            session.authenticator_created_at = now;
            session.authenticator_expires_at = now + auth_result.expires_in_ms;
            self.logger.log(
                "EMBEDDED_PROVIDER",
                "Updated authenticator expiration from auth response",
                Some(&serde_json::json!({
                    "expiresInMs": auth_result.expires_in_ms,
                    "expiresAt": session.authenticator_expires_at,
                })),
            );
        }

        storage.save_session(&session).await?;

        // Clear the logout flag after successful authentication
        storage.set_should_clear_previous_session(false).await?;
        self.logger.log(
            "EMBEDDED_PROVIDER",
            "Cleared logout flag after successful authentication",
            None,
        );

        self.initialize_client_from_session(&session).await?;

        // Ensure authenticator is valid after successful connection
        self.ensure_valid_authenticator().await?;

        let wallet_id = self.wallet_id.read().await;
        let addresses = self.addresses.read().await;

        Ok(ConnectResult {
            wallet_id: wallet_id.clone(),
            addresses: addresses.clone(),
            status: Some(ConnectStatus::Completed),
            auth_user_id: session.auth_user_id.clone(),
            auth_provider: session.auth_provider,
        })
    }

    fn validate_auth_options(
        &self,
        auth_options: &AuthOptions,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !EMBEDDED_PROVIDER_AUTH_TYPES.contains(&auth_options.provider) {
            return Err(format!(
                "Invalid auth provider: {:?}. Must be one of: Google, Apple, Phantom",
                auth_options.provider
            )
            .into());
        }
        Ok(())
    }

    fn is_session_valid(&self, session: &Session) -> bool {
        if session.wallet_id.is_empty()
            || session.organization_id.is_empty()
            || session.stamper_info.public_key.is_empty()
        {
            self.logger.log(
                "EMBEDDED_PROVIDER",
                "Session missing required fields",
                Some(&serde_json::json!({
                    "hasWalletId": !session.wallet_id.is_empty(),
                    "hasOrganizationId": !session.organization_id.is_empty(),
                    "hasStamperInfo": !session.stamper_info.public_key.is_empty(),
                })),
            );
            return false;
        }

        if session.status != SessionStatus::Completed {
            return false;
        }

        if session.authenticator_expires_at == 0 {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        now < session.authenticator_expires_at
    }

    async fn handle_auth_flow(
        &self,
        public_key: &str,
        stamper_info: &StamperInfo,
        auth_options: &AuthOptions,
        expires_in_ms: u64,
    ) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>> {
        if self.config.embedded_wallet_type == "user-wallet" {
            if auth_options.provider == EmbeddedProviderAuthType::Phantom {
                let session = self
                    .handle_phantom_auth(public_key, stamper_info, expires_in_ms)
                    .await?;
                Ok(Some(session))
            } else {
                self.handle_redirect_auth(public_key, stamper_info, auth_options)
                    .await
            }
        } else {
            // App-wallet flow would go here
            Err("app-wallet type is not currently supported".into())
        }
    }

    async fn handle_phantom_auth(
        &self,
        public_key: &str,
        stamper_info: &StamperInfo,
        expires_in_ms: u64,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        let phantom_app = self.platform.phantom_app_provider();

        if !phantom_app.is_available() {
            return Err("Phantom app is not available. Please install the Phantom browser extension or mobile app.".into());
        }

        let session_id = generate_session_id();
        let auth_result = phantom_app
            .authenticate(PhantomAppAuthOptions {
                public_key: public_key.to_string(),
                app_id: self.config.app_id.clone(),
                session_id: session_id.clone(),
            })
            .await?;

        let effective_expires = if auth_result.expires_in_ms > 0 {
            auth_result.expires_in_ms
        } else {
            expires_in_ms
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let session = Session {
            session_id,
            wallet_id: auth_result.wallet_id,
            organization_id: auth_result.organization_id,
            app_id: self.config.app_id.clone(),
            stamper_info: stamper_info.clone(),
            keypair: None,
            auth_provider: EmbeddedProviderAuthType::Phantom,
            account_derivation_index: Some(auth_result.account_derivation_index),
            status: SessionStatus::Completed,
            created_at: now,
            last_used: now,
            authenticator_created_at: now,
            authenticator_expires_at: now + effective_expires,
            last_renewal_attempt: None,
            auth_user_id: auth_result.auth_user_id,
        };

        self.platform.storage().save_session(&session).await?;
        Ok(session)
    }

    async fn handle_redirect_auth(
        &self,
        public_key: &str,
        stamper_info: &StamperInfo,
        auth_options: &AuthOptions,
    ) -> Result<Option<Session>, Box<dyn std::error::Error + Send + Sync>> {
        let session_id = generate_session_id();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let temp_session = Session {
            session_id: session_id.clone(),
            wallet_id: format!("temp-wallet-{}", now),
            organization_id: format!("temp-org-{}", now),
            app_id: self.config.app_id.clone(),
            stamper_info: stamper_info.clone(),
            keypair: None,
            auth_provider: auth_options.provider,
            account_derivation_index: None,
            status: SessionStatus::Pending,
            created_at: now,
            last_used: now,
            authenticator_created_at: now,
            authenticator_expires_at: now + AUTHENTICATOR_EXPIRATION_TIME_MS,
            last_renewal_attempt: None,
            auth_user_id: None,
        };

        self.platform.storage().save_session(&temp_session).await?;

        let should_clear = self
            .platform
            .storage()
            .get_should_clear_previous_session()
            .await
            .unwrap_or(false);

        let auth_result = self
            .platform
            .auth_provider()
            .authenticate(PhantomConnectOptions {
                public_key: public_key.to_string(),
                app_id: self.config.app_id.clone(),
                provider: Some(auth_options.provider),
                redirect_url: Some(self.config.auth_options.redirect_url.clone()),
                auth_url: Some(self.config.auth_options.auth_url.clone()),
                session_id,
                clear_previous_session: Some(should_clear),
                allow_refresh: Some(!should_clear),
                algorithm: Some(self.platform.stamper().algorithm()),
            })
            .await?;

        match auth_result {
            Some(result) => {
                // Update temp session with actual auth result
                let mut session = temp_session;
                session.wallet_id = result.wallet_id;
                session.organization_id = result.organization_id;
                session.auth_provider = result.provider;
                session.account_derivation_index = Some(result.account_derivation_index);
                session.auth_user_id = result.auth_user_id;
                session.status = SessionStatus::Completed;
                session.last_used = now;

                if result.expires_in_ms > 0 {
                    session.authenticator_created_at = now;
                    session.authenticator_expires_at = now + result.expires_in_ms;
                }

                self.platform.storage().save_session(&session).await?;
                self.platform
                    .storage()
                    .set_should_clear_previous_session(false)
                    .await?;

                Ok(Some(session))
            }
            None => {
                // Redirect in progress
                Ok(None)
            }
        }
    }

    async fn initialize_client_from_session(
        &self,
        session: &Session,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.log(
            "EMBEDDED_PROVIDER",
            "Initializing PhantomClient from session",
            Some(&serde_json::json!({
                "organizationId": session.organization_id,
                "walletId": session.wallet_id,
            })),
        );

        // Ensure stamper is initialized
        let stamper = self.platform.stamper();
        if StamperWithKeyManagement::get_key_info(stamper).is_none() {
            stamper.init().await?;
        }

        let mut headers = self
            .platform
            .analytics_headers()
            .unwrap_or_default();

        if let Some(ref user_id) = session.auth_user_id {
            headers.insert("x-auth-user-id".to_string(), user_id.clone());
        }

        let config = PhantomClientConfig {
            api_base_url: self.config.api_base_url.clone(),
            organization_id: Some(session.organization_id.clone()),
            headers: Some(headers),
            wallet_type: self.config.embedded_wallet_type.clone(),
        };

        // Pass the platform stamper to PhantomClient for request signing.
        let stamper = self.platform.stamper_for_client();
        let new_client = PhantomClient::new(config, stamper);

        *self.client.write().await = Some(new_client);
        *self.wallet_id.write().await = Some(session.wallet_id.clone());

        // Fetch wallet addresses with retry and auto-disconnect on failure
        let derivation_index = session.account_derivation_index.unwrap_or(0);
        let wallet_id = session.wallet_id.clone();
        let address_types = self.config.address_types.clone();

        let get_addresses_result = {
            let client_guard = self.client.read().await;
            let c = client_guard.as_ref().ok_or("Client not initialized")?;

            crate::utils::retry_with_backoff(
                || c.get_wallet_addresses(&wallet_id, None, Some(derivation_index)),
                "getWalletAddresses",
                self.logger.as_ref(),
                3,
                1000,
            )
            .await
        };

        match get_addresses_result {
            Ok(raw_addresses) => {
                let filtered: Vec<WalletAddress> = raw_addresses
                    .into_iter()
                    .filter(|addr| {
                        address_types.iter().any(|t| {
                            match serde_json::to_value(t) {
                                Ok(v) => v.as_str().map_or(false, |s| s == addr.address_type),
                                Err(_) => false,
                            }
                        })
                    })
                    .map(|addr| {
                        let parsed_type = serde_json::from_value(
                            serde_json::Value::String(addr.address_type.clone()),
                        )
                        .unwrap_or(crate::constants::AddressFormat::Ethereum);
                        WalletAddress {
                            address_type: parsed_type,
                            address: addr.address,
                        }
                    })
                    .collect();

                *self.addresses.write().await = filtered;
            }
            Err(e) => {
                self.logger.error(
                    "EMBEDDED_PROVIDER",
                    "getWalletAddresses failed after retries, disconnecting",
                    Some(&serde_json::json!({
                        "walletId": wallet_id,
                        "error": e.to_string(),
                        "derivationIndex": derivation_index,
                    })),
                );

                // Clear the session if getWalletAddresses fails after retries
                let _ = self.platform.storage().clear_session().await;
                *self.client.write().await = None;
                *self.wallet_id.write().await = None;
                *self.addresses.write().await = Vec::new();

                return Err(e.into());
            }
        }

        Ok(())
    }
}
