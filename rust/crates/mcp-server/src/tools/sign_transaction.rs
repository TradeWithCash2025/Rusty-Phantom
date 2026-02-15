//! sign_transaction tool — signs a transaction using a wallet.

use phantom_client::SignTransactionParams;
use phantom_utils::is_solana_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::network::normalize_network_id;
use crate::utils::solana::get_solana_address;

/// Create the sign_transaction tool handler.
pub fn sign_transaction_tool() -> ToolHandler {
    ToolHandler {
        name: "sign_transaction",
        description: "Signs a transaction using the authenticated embedded wallet. Supports Solana, Ethereum, Bitcoin, and other chains.",
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "walletId": {
                    "type": "string",
                    "description": "Optional wallet ID to use for signing (defaults to authenticated wallet)"
                },
                "transaction": {
                    "type": "string",
                    "description": "The transaction to sign (format depends on chain: base64url for Solana, RLP-encoded hex for Ethereum)"
                },
                "networkId": {
                    "type": "string",
                    "description": "Network identifier (e.g., \"eip155:1\" for Ethereum mainnet, \"solana:mainnet\" for Solana)"
                },
                "derivationIndex": {
                    "type": "number",
                    "description": "Optional derivation index for the account (default: 0)",
                    "minimum": 0
                },
                "account": {
                    "type": "string",
                    "description": "Optional specific account address to use for simulation/signing"
                }
            }),
            required: Some(vec!["transaction".to_string(), "networkId".to_string()]),
        },
        handler: |params, context| Box::pin(handle_sign_transaction(params, context)),
    }
}

async fn handle_sign_transaction(
    params: Value,
    context: &ToolContext,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let transaction = params
        .get("transaction")
        .and_then(|v| v.as_str())
        .ok_or("transaction must be a string")?;

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

    let derivation_index = params
        .get("derivationIndex")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let network_id = normalize_network_id(network_id_raw);

    let mut account = params
        .get("account")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if account.is_none() && is_solana_chain(&network_id) {
        account = Some(get_solana_address(context, wallet_id, derivation_index).await?);
    }

    context.logger.info(&format!(
        "Signing transaction for wallet {} on network {}",
        wallet_id, network_id
    ));

    let result = context
        .client
        .sign_transaction(&SignTransactionParams {
            wallet_id: wallet_id.to_string(),
            transaction: transaction.to_string(),
            network_id: network_id.clone(),
            derivation_index,
            account,
        })
        .await?;

    context.logger.info(&format!(
        "Successfully signed transaction for wallet {}",
        wallet_id
    ));

    Ok(json!({
        "signedTransaction": result.raw_transaction,
    }))
}
