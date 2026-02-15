//! sign_message tool — signs a message using a wallet.

use phantom_client::SignMessageParams;
use phantom_utils::is_ethereum_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::network::normalize_network_id;

/// Create the sign_message tool handler.
pub fn sign_message_tool() -> ToolHandler {
    ToolHandler {
        name: "sign_message",
        description: "Signs a UTF-8 message using the authenticated embedded wallet. Automatically routes to the correct signing method based on the network (Ethereum vs other chains).",
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "walletId": {
                    "type": "string",
                    "description": "Optional wallet ID to use for signing (defaults to authenticated wallet)"
                },
                "message": {
                    "type": "string",
                    "description": "The UTF-8 message to sign"
                },
                "networkId": {
                    "type": "string",
                    "description": "Network identifier (e.g., \"eip155:1\" for Ethereum mainnet, \"solana:mainnet\" for Solana)"
                },
                "derivationIndex": {
                    "type": "integer",
                    "description": "Optional derivation index for the account (default: 0)",
                    "minimum": 0
                }
            }),
            required: Some(vec!["message".to_string(), "networkId".to_string()]),
        },
        handler: |params, context| Box::pin(handle_sign_message(params, context)),
    }
}

async fn handle_sign_message(
    params: Value,
    context: &ToolContext,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let message = params
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or("message must be a string")?;

    let network_id_raw = params
        .get("networkId")
        .and_then(|v| v.as_str())
        .ok_or("networkId must be a string")?;

    let wallet_id = params
        .get("walletId")
        .and_then(|v| v.as_str())
        .unwrap_or(&context.session.wallet_id);

    if wallet_id.is_empty() {
        return Err("walletId is required (missing from session and not provided)".into());
    }

    let derivation_index = match params.get("derivationIndex") {
        Some(serde_json::Value::Null) | None => None,
        Some(v) => {
            let idx = v.as_u64().ok_or_else(|| {
                format!("derivationIndex must be a non-negative integer, got: {}", v)
            })?;
            Some(idx as u32)
        }
    };

    let network_id = normalize_network_id(network_id_raw);

    context.logger.info(&format!(
        "Signing message for wallet {} on network {}",
        wallet_id, network_id
    ));

    let signature = if is_ethereum_chain(&network_id) {
        // For Ethereum chains, convert message to base64url and use ethereumSignMessage
        let base64_message = phantom_base64url::string_to_base64url(message);
        context.logger.debug("Using Ethereum message signing");

        context
            .client
            .ethereum_sign_message(&SignMessageParams {
                wallet_id: wallet_id.to_string(),
                message: base64_message,
                network_id: network_id.clone(),
                derivation_index,
            })
            .await?
    } else {
        // For non-Ethereum chains (Solana, etc.), use signUtf8Message
        context.logger.debug("Using UTF-8 message signing");

        context
            .client
            .sign_utf8_message(&SignMessageParams {
                wallet_id: wallet_id.to_string(),
                message: message.to_string(),
                network_id: network_id.clone(),
                derivation_index,
            })
            .await?
    };

    context.logger.info(&format!(
        "Successfully signed message for wallet {}",
        wallet_id
    ));

    Ok(json!({ "signature": signature }))
}
