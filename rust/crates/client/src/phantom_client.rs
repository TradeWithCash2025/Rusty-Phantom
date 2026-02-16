//! PhantomClient — main HTTP client for interacting with the Phantom wallet API.
//!
//! Mirrors the TypeScript `PhantomClient` class from `packages/client/src/PhantomClient.ts`.

use phantom_sdk_types::Stamper;
use phantom_utils::{get_secure_timestamp, is_ethereum_chain, is_solana_chain, random_uuid};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;
use std::sync::Arc;

use crate::caip2_mappings::derive_submission_config;
use crate::constants::{get_client_network_config, DerivationPath};
use crate::errors::{parse_wallet_service_error, ClientError};
use crate::types::*;

/// Maximum length for names (organization, username, authenticator).
const MAX_NAME_LENGTH: usize = 64;

/// The Phantom wallet API client.
///
/// Handles all communication with the Phantom KMS RPC API including
/// wallet creation, transaction signing, and organization management.
#[derive(Clone)]
pub struct PhantomClient {
    config: PhantomClientConfig,
    http: reqwest::Client,
    stamper: Option<Arc<dyn Stamper>>,
}

impl PhantomClient {
    /// Create a new PhantomClient.
    ///
    /// # Arguments
    /// * `config` - Client configuration including API URL and organization ID.
    /// * `stamper` - Optional stamper for request authentication.
    pub fn new(config: PhantomClientConfig, stamper: Option<Arc<dyn Stamper>>) -> Self {
        let mut default_headers = HeaderMap::new();

        // Add any additional headers provided in config
        if let Some(ref headers) = config.headers {
            for (key, value) in headers {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    HeaderValue::from_str(value),
                ) {
                    default_headers.insert(name, val);
                }
            }
        }

        let http = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()
            .unwrap_or_default();

        let mut resolved_config = config;
        if resolved_config.wallet_type.is_empty() {
            resolved_config.wallet_type = "user-wallet".to_string();
        }

        Self {
            config: resolved_config,
            http,
            stamper,
        }
    }

    /// Set the organization ID.
    #[allow(clippy::result_large_err)]
    pub fn set_organization_id(&mut self, organization_id: String) -> Result<(), ClientError> {
        if organization_id.is_empty() {
            return Err(ClientError::Config(
                "organizationId is required".to_string(),
            ));
        }
        self.config.organization_id = Some(organization_id);
        Ok(())
    }

    /// Get the stamper's cryptographic algorithm, if a stamper is present.
    ///
    /// Returns `None` when no stamper is configured.
    /// Mirrors the TS `this.stamper?.algorithm`.
    pub fn stamper_algorithm(&self) -> Option<phantom_constants::Algorithm> {
        self.stamper.as_ref().map(|s| s.algorithm())
    }

    /// Get the authenticator public key from the stamper, if available.
    ///
    /// Checks if the stamper supports key management by calling `get_key_info()`.
    /// This mirrors the TS `"getKeyInfo" in this.stamper` duck-type check.
    fn get_authenticator_public_key(&self) -> Option<String> {
        self.stamper
            .as_ref()
            .and_then(|s| s.get_key_info())
            .map(|info| info.public_key)
    }

    /// Create a new wallet.
    pub async fn create_wallet(
        &self,
        wallet_name: Option<&str>,
    ) -> Result<CreateWalletResult, ClientError> {
        let org_id = self.config.organization_id.as_ref().ok_or_else(|| {
            ClientError::Config("organizationId is required to create a wallet".to_string())
        })?;

        let timestamp = get_secure_timestamp().await;

        let name = wallet_name
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("Wallet {}", timestamp));

        // Create wallet request
        let wallet_request = serde_json::json!({
            "organizationId": org_id,
            "walletName": name,
            "accounts": [
                DerivationPath::solana(0),
                DerivationPath::ethereum(0),
                DerivationPath::bitcoin(0),
                DerivationPath::sui(0),
            ]
        });

        let request = serde_json::json!({
            "method": "createWallet",
            "params": wallet_request,
            "timestampMs": timestamp,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        let wallet_id = response["result"]["walletId"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        // Fetch the accounts
        let accounts_request = serde_json::json!({
            "method": "getAccounts",
            "params": {
                "accounts": [
                    DerivationPath::solana(0),
                    DerivationPath::ethereum(0),
                    DerivationPath::bitcoin(0),
                    DerivationPath::sui(0),
                ],
                "organizationId": org_id,
                "walletId": wallet_id,
            },
            "timestampMs": timestamp,
        });

        let accounts_response = self.post_kms_rpc(&accounts_request, None).await?;
        let accounts = accounts_response["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|account| WalletAddress {
                        address_type: account["addressFormat"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        address: account["address"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(CreateWalletResult {
            wallet_id,
            addresses: accounts,
        })
    }

    /// Sign a transaction.
    pub async fn sign_transaction(
        &self,
        params: &SignTransactionParams,
    ) -> Result<SignedTransactionResult, ClientError> {
        let result = self.perform_transaction_signing(params, false).await?;
        Ok(SignedTransactionResult {
            raw_transaction: result.0,
        })
    }

    /// Sign and send a transaction.
    pub async fn sign_and_send_transaction(
        &self,
        params: &SignAndSendTransactionParams,
    ) -> Result<SignedTransaction, ClientError> {
        let sign_params = SignTransactionParams {
            wallet_id: params.wallet_id.clone(),
            transaction: params.transaction.clone(),
            network_id: params.network_id.clone(),
            derivation_index: params.derivation_index,
            account: params.account.clone(),
        };
        let result = self.perform_transaction_signing(&sign_params, true).await?;
        Ok(SignedTransaction {
            raw_transaction: result.0,
            hash: result.1,
        })
    }

    /// Get wallet addresses for a given wallet.
    pub async fn get_wallet_addresses(
        &self,
        wallet_id: &str,
        derivation_paths: Option<&[String]>,
        derivation_index: Option<u32>,
    ) -> Result<Vec<WalletAddress>, ClientError> {
        let org_id = self.config.organization_id.as_deref();
        let account_index = derivation_index.unwrap_or(0);

        let paths: Vec<String> = derivation_paths.map(|p| p.to_vec()).unwrap_or_else(|| {
            vec![
                DerivationPath::solana(account_index),
                DerivationPath::ethereum(account_index),
                DerivationPath::bitcoin(account_index),
                DerivationPath::sui(account_index),
            ]
        });

        let request = serde_json::json!({
            "method": "getAccounts",
            "params": {
                "accounts": paths,
                "organizationId": org_id,
                "walletId": wallet_id,
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        let addresses = response["result"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|account| WalletAddress {
                        address_type: account["addressFormat"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        address: account["address"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(addresses)
    }

    /// Sign an Ethereum message using EIP-191 personal sign.
    pub async fn ethereum_sign_message(
        &self,
        params: &SignMessageParams,
    ) -> Result<String, ClientError> {
        let org_id = self.config.organization_id.as_ref().ok_or_else(|| {
            ClientError::Config("organizationId is required to sign a message".to_string())
        })?;

        let derivation_index = params.derivation_index.unwrap_or(0);
        let network_config = get_client_network_config(&params.network_id, derivation_index)
            .ok_or_else(|| ClientError::UnsupportedNetwork(params.network_id.clone()))?;

        let request = serde_json::json!({
            "method": "ethereumSignMessage",
            "params": {
                "message": params.message,
                "organizationId": org_id,
                "walletId": params.wallet_id,
                "derivationInfo": {
                    "derivationPath": network_config.derivation_path,
                    "curve": network_config.curve,
                    "addressFormat": network_config.address_format,
                },
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"]["signature"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Sign a UTF-8 message for Solana.
    pub async fn sign_utf8_message(
        &self,
        params: &SignMessageParams,
    ) -> Result<String, ClientError> {
        let org_id = self.config.organization_id.as_ref().ok_or_else(|| {
            ClientError::Config("organizationId is required to sign a message".to_string())
        })?;

        let derivation_index = params.derivation_index.unwrap_or(0);
        let network_config = get_client_network_config(&params.network_id, derivation_index)
            .ok_or_else(|| ClientError::UnsupportedNetwork(params.network_id.clone()))?;

        let request = serde_json::json!({
            "method": "signUtf8Message",
            "params": {
                "organizationId": org_id,
                "walletId": params.wallet_id,
                "message": params.message,
                "algorithm": network_config.algorithm,
                "derivationInfo": {
                    "derivationPath": network_config.derivation_path,
                    "curve": network_config.curve,
                    "addressFormat": network_config.address_format,
                },
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"]["signature"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Sign EIP-712 typed data for Ethereum.
    pub async fn ethereum_sign_typed_data(
        &self,
        params: &SignTypedDataParams,
    ) -> Result<String, ClientError> {
        let org_id = self.config.organization_id.as_ref().ok_or_else(|| {
            ClientError::Config("organizationId is required to sign typed data".to_string())
        })?;

        let derivation_index = params.derivation_index.unwrap_or(0);
        let network_config = get_client_network_config(&params.network_id, derivation_index)
            .ok_or_else(|| ClientError::UnsupportedNetwork(params.network_id.clone()))?;

        let request = serde_json::json!({
            "method": "ethereumSignTypedData",
            "params": {
                "typedData": params.typed_data,
                "organizationId": org_id,
                "walletId": params.wallet_id,
                "derivationInfo": {
                    "derivationPath": network_config.derivation_path,
                    "curve": network_config.curve,
                    "addressFormat": network_config.address_format,
                },
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"]["signature"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Get wallets for the current organization.
    pub async fn get_wallets(
        &self,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<GetWalletsResult, ClientError> {
        let request = serde_json::json!({
            "method": "getOrganizationWallets",
            "params": {
                "organizationId": self.config.organization_id,
                "limit": limit.unwrap_or(20),
                "offset": offset.unwrap_or(0),
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        let result = &response["result"];

        let wallets = result["wallets"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|w| Wallet {
                        wallet_id: w["walletId"].as_str().unwrap_or_default().to_string(),
                        wallet_name: w["walletName"].as_str().unwrap_or_default().to_string(),
                        created_at: w["createdAt"].as_str().map(|s| s.to_string()),
                        updated_at: w["updatedAt"].as_str().map(|s| s.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(GetWalletsResult {
            wallets,
            total_count: result["totalCount"].as_u64().unwrap_or(0),
            limit: result["limit"].as_u64().unwrap_or(20),
            offset: result["offset"].as_u64().unwrap_or(0),
        })
    }

    /// Get organization details by organization ID.
    pub async fn get_organization(&self, organization_id: &str) -> Result<Value, ClientError> {
        let request = serde_json::json!({
            "method": "getOrganization",
            "params": {
                "organizationId": organization_id,
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    /// Create a new organization.
    pub async fn create_organization(
        &self,
        name: &str,
        users: &[UserConfig],
        tags: Option<&[String]>,
    ) -> Result<Value, ClientError> {
        if name.is_empty() {
            return Err(ClientError::Config(
                "Organization name is required".to_string(),
            ));
        }

        validate_name_length(name, "Organization")?;

        if users.is_empty() {
            return Err(ClientError::Config(
                "At least one user is required".to_string(),
            ));
        }

        // Validate user names and authenticator names
        for user in users {
            if !user.username.is_empty() {
                validate_name_length(&user.username, "Username")?;
            }
            for auth in &user.authenticators {
                let auth_name = match auth {
                    AuthenticatorConfig::Keypair {
                        authenticator_name, ..
                    }
                    | AuthenticatorConfig::Passkey {
                        authenticator_name, ..
                    }
                    | AuthenticatorConfig::Oidc {
                        authenticator_name, ..
                    } => authenticator_name,
                };
                if !auth_name.is_empty() {
                    validate_name_length(auth_name, "Authenticator")?;
                }
            }
        }

        let users_json: Vec<Value> = users
            .iter()
            .map(|user_config| {
                let username = if user_config.username.is_empty() {
                    format!("user-{}", random_uuid())
                } else {
                    user_config.username.clone()
                };

                let policy = match user_config.role.as_deref() {
                    Some("ADMIN") | None => serde_json::json!({ "type": "root" }),
                    _ => serde_json::json!({ "type": "CEL", "preset": "LEGACY_USER_ROLE" }),
                };

                serde_json::json!({
                    "username": username,
                    "authenticators": user_config.authenticators,
                    "policy": policy,
                })
            })
            .collect();

        let mut params = serde_json::json!({
            "organizationName": name,
            "users": users_json,
        });

        if let Some(tags) = tags {
            params["tags"] = serde_json::json!(tags);
        }

        let request = serde_json::json!({
            "method": "createOrganization",
            "params": params,
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    /// Create an authenticator for a user in an organization.
    pub async fn create_authenticator(
        &self,
        params: &CreateAuthenticatorParams,
    ) -> Result<Value, ClientError> {
        if !params.username.is_empty() {
            validate_name_length(&params.username, "Username")?;
        }
        if !params.authenticator_name.is_empty() {
            validate_name_length(&params.authenticator_name, "Authenticator")?;
        }

        // Validate the nested authenticator's own name (mirrors TS params.authenticator?.authenticatorName check)
        let nested_auth_name = match &params.authenticator {
            AuthenticatorConfig::Keypair {
                authenticator_name, ..
            }
            | AuthenticatorConfig::Passkey {
                authenticator_name, ..
            }
            | AuthenticatorConfig::Oidc {
                authenticator_name, ..
            } => authenticator_name,
        };
        if !nested_auth_name.is_empty() {
            validate_name_length(nested_auth_name, "Authenticator")?;
        }

        let request = serde_json::json!({
            "method": "createAuthenticator",
            "params": {
                "organizationId": params.organization_id,
                "username": params.username,
                "authenticatorName": params.authenticator_name,
                "authenticator": params.authenticator,
                "replaceExpirable": params.replace_expirable,
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    /// Delete an authenticator for a user in an organization.
    pub async fn delete_authenticator(
        &self,
        params: &DeleteAuthenticatorParams,
    ) -> Result<Value, ClientError> {
        let request = serde_json::json!({
            "method": "deleteAuthenticator",
            "params": {
                "organizationId": params.organization_id,
                "username": params.username,
                "authenticatorId": params.authenticator_id,
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    /// Grant organization access.
    pub async fn grant_organization_access(&self, params: &Value) -> Result<Value, ClientError> {
        let request = serde_json::json!({
            "method": "grantOrganizationAccess",
            "params": params,
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    /// Add a new user to an organization.
    pub async fn add_user_to_organization(&self, params: &Value) -> Result<(), ClientError> {
        let request = serde_json::json!({
            "method": "addUserToOrganization",
            "params": params,
            "timestampMs": get_secure_timestamp().await,
        });

        self.post_kms_rpc(&request, None).await?;
        Ok(())
    }

    /// Get a wallet by tag from the specified organization.
    pub async fn get_wallet_with_tag(
        &self,
        params: &GetWalletWithTagParams,
    ) -> Result<Value, ClientError> {
        let request = serde_json::json!({
            "method": "getWalletWithTag",
            "params": {
                "organizationId": params.organization_id,
                "tag": params.tag,
                "derivationPaths": params.derivation_paths,
            },
            "timestampMs": get_secure_timestamp().await,
        });

        let response = self.post_kms_rpc(&request, None).await?;
        Ok(response["result"].clone())
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Shared signing logic for signTransaction and signAndSendTransaction.
    async fn perform_transaction_signing(
        &self,
        params: &SignTransactionParams,
        include_submission_config: bool,
    ) -> Result<(String, Option<String>), ClientError> {
        let org_id = self.config.organization_id.as_ref().ok_or_else(|| {
            ClientError::Config("organizationId is required to sign a transaction".to_string())
        })?;

        let derivation_index = params.derivation_index.unwrap_or(0);
        let method_name = self.get_rpc_method_name(&params.network_id, include_submission_config);

        let submission_config = derive_submission_config(&params.network_id).ok_or_else(|| {
            ClientError::Config(format!(
                "SubmissionConfig could not be derived for network ID: {}",
                params.network_id
            ))
        })?;

        let network_config = get_client_network_config(&params.network_id, derivation_index)
            .ok_or_else(|| ClientError::UnsupportedNetwork(params.network_id.clone()))?;

        let authenticator_public_key = self.get_authenticator_public_key();

        let transaction_for_signing = self
            .get_transaction_for_signing(
                &params.transaction,
                &params.network_id,
                &submission_config,
                authenticator_public_key.as_deref(),
                params.account.as_deref(),
                &method_name,
            )
            .await?;

        let mut sign_params = serde_json::json!({
            "organizationId": org_id,
            "walletId": params.wallet_id,
            "transaction": transaction_for_signing,
            "derivationInfo": {
                "derivationPath": network_config.derivation_path,
                "curve": network_config.curve,
                "addressFormat": network_config.address_format,
            },
        });

        if include_submission_config {
            sign_params["submissionConfig"] =
                serde_json::to_value(&submission_config).unwrap_or_default();
        }

        if include_submission_config {
            if let Some(ref account) = params.account {
                sign_params["simulationConfig"] = serde_json::json!({ "account": account });
            }
        }

        let request = serde_json::json!({
            "method": "signTransaction",
            "params": sign_params,
            "timestampMs": get_secure_timestamp().await,
        });

        let extra_headers = vec![("X-Rpc-Method".to_string(), method_name)];
        let response = self.post_kms_rpc(&request, Some(&extra_headers)).await?;

        let signed_transaction = response["result"]["transaction"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        let hash = if include_submission_config {
            response["rpc_submission_result"]["result"]
                .as_str()
                .map(|s| s.to_string())
        } else {
            None
        };

        Ok((signed_transaction, hash))
    }

    /// Determine the RPC method name based on network and submission mode.
    fn get_rpc_method_name(&self, network_id: &str, include_submission_config: bool) -> String {
        let is_evm = is_ethereum_chain(network_id);
        if is_evm {
            if include_submission_config {
                "eth_sendTransaction".to_string()
            } else {
                "eth_signTransaction".to_string()
            }
        } else if include_submission_config {
            "signAndSendTransaction".to_string()
        } else {
            "signTransaction".to_string()
        }
    }

    /// Get the transaction in the format expected for signing.
    ///
    /// For EVM: wraps in `{ kind: "RLP_ENCODED", bytes: ... }` format.
    /// For Solana user-wallets: goes through the two-phase spending limits flow.
    /// For others: returns the transaction as-is.
    async fn get_transaction_for_signing(
        &self,
        encoded_transaction: &str,
        network_id: &str,
        submission_config: &SubmissionConfig,
        authenticator_public_key: Option<&str>,
        account: Option<&str>,
        method_name: &str,
    ) -> Result<Value, ClientError> {
        let is_evm = is_ethereum_chain(network_id);
        let is_solana = is_solana_chain(network_id);

        // For EVM transactions, use the object format with kind and bytes
        if is_evm {
            return Ok(serde_json::json!({
                "kind": "RLP_ENCODED",
                "bytes": encoded_transaction,
            }));
        }

        // TWO-PHASE SPENDING LIMITS FLOW (Solana user-wallet only)
        if is_solana && self.config.wallet_type == "user-wallet" {
            let account = account.ok_or_else(|| {
                ClientError::Config(
                    "Account is required to simulate Solana transactions with spending limits"
                        .to_string(),
                )
            })?;

            let prepare_response = self
                .prepare(
                    encoded_transaction,
                    self.config.organization_id.as_deref().unwrap_or_default(),
                    submission_config,
                    account,
                    authenticator_public_key,
                    method_name,
                )
                .await?;

            return Ok(Value::String(prepare_response.transaction));
        }

        // Non-EVM chains (including Solana server-wallet): send as-is
        Ok(Value::String(encoded_transaction.to_string()))
    }

    /// Call the prepare endpoint for two-phase spending limits.
    async fn prepare(
        &self,
        transaction: &str,
        organization_id: &str,
        submission_config: &SubmissionConfig,
        account: &str,
        authenticator_public_key: Option<&str>,
        method_name: &str,
    ) -> Result<PrepareResponse, ClientError> {
        let url = format!("{}/prepare", self.config.api_base_url);

        let mut body = serde_json::json!({
            "transaction": transaction,
            "organizationId": organization_id,
            "submissionConfig": submission_config,
            "simulationConfig": { "account": account },
        });

        if let Some(key) = authenticator_public_key {
            body["authenticatorPublicKey"] = Value::String(key.to_string());
        }

        let mut request = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Rpc-Method", method_name);

        // Apply stamp if stamper is available
        if let Some(ref stamper) = self.stamper {
            let body_str = serde_json::to_string(&body).unwrap_or_default();
            match stamper
                .stamp(phantom_sdk_types::StampParams::Pki {
                    data: body_str.as_bytes().to_vec(),
                })
                .await
            {
                Ok(stamp) => {
                    request = request.header("X-Phantom-Stamp", stamp);
                }
                Err(e) => {
                    return Err(ClientError::Api(format!(
                        "Failed to stamp prepare request: {}",
                        e
                    )));
                }
            }
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Api(format!("Prepare request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_body = response
                .json::<PrepareErrorResponse>()
                .await
                .unwrap_or_else(|_| PrepareErrorResponse {
                    error_type: None,
                    title: None,
                    detail: Some("Failed to submit transaction".to_string()),
                    request_id: None,
                    message: None,
                    previous_spend_cents: None,
                    transaction_spend_cents: None,
                    total_spend_cents: None,
                    limit_cents: None,
                    scanner_result: None,
                });

            // Check for wallet service errors
            if let Some(ws_error) = parse_wallet_service_error(&error_body) {
                return Err(ClientError::WalletService(ws_error));
            }

            let message = error_body
                .detail
                .or(error_body.message)
                .unwrap_or_else(|| "Failed to submit transaction".to_string());
            return Err(ClientError::Prepare(message));
        }

        let prepare_response: PrepareResponse = response
            .json()
            .await
            .map_err(|e| ClientError::Api(format!("Failed to parse prepare response: {}", e)))?;

        Ok(prepare_response)
    }

    /// Post a JSON-RPC request to the KMS API.
    async fn post_kms_rpc(
        &self,
        request_body: &Value,
        extra_headers: Option<&[(String, String)]>,
    ) -> Result<Value, ClientError> {
        let url = &self.config.api_base_url;

        let body_str = serde_json::to_string(request_body).unwrap_or_default();

        let mut request = self
            .http
            .post(url)
            .header("Content-Type", "application/json");

        // Add extra headers
        if let Some(headers) = extra_headers {
            for (key, value) in headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        // Apply stamp if stamper is available
        if let Some(ref stamper) = self.stamper {
            match stamper
                .stamp(phantom_sdk_types::StampParams::Pki {
                    data: body_str.as_bytes().to_vec(),
                })
                .await
            {
                Ok(stamp) => {
                    request = request.header("X-Phantom-Stamp", stamp);
                }
                Err(e) => {
                    return Err(ClientError::Api(format!("Failed to stamp request: {}", e)));
                }
            }
        }

        let response = request
            .body(body_str)
            .send()
            .await
            .map_err(ClientError::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClientError::Api(format!(
                "KMS API error (status {}): {}",
                status, error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(ClientError::Http)?;
        Ok(response_json)
    }
}

/// Validate that a name doesn't exceed the maximum length.
#[allow(clippy::result_large_err)]
fn validate_name_length(name: &str, name_type: &str) -> Result<(), ClientError> {
    if name.len() > MAX_NAME_LENGTH {
        return Err(ClientError::Config(format!(
            "{} name cannot exceed {} characters. Current length: {}",
            name_type,
            MAX_NAME_LENGTH,
            name.len()
        )));
    }
    Ok(())
}
