//! get_wallet_addresses tool — gets addresses for the authenticated embedded wallet.

use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};

/// Create the get_wallet_addresses tool handler.
pub fn get_wallet_addresses_tool() -> ToolHandler {
    ToolHandler {
        name: "get_wallet_addresses",
        description: "Gets all blockchain addresses for the authenticated embedded wallet (Solana, Ethereum, Bitcoin, Sui)",
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "derivationIndex": {
                    "type": "number",
                    "description": "Optional derivation index for the addresses",
                    "minimum": 0
                }
            }),
            required: None,
        },
        handler: |params, context| Box::pin(handle_get_wallet_addresses(params, context)),
    }
}

async fn handle_get_wallet_addresses(
    params: Value,
    context: &ToolContext,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let derivation_index = params
        .get("derivationIndex")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    context.logger.info("Getting addresses for wallet");

    let addresses = context
        .client
        .get_wallet_addresses(&context.session.wallet_id, None, derivation_index)
        .await?;

    context
        .logger
        .info(&format!("Successfully retrieved {} addresses", addresses.len()));

    let address_list: Vec<Value> = addresses
        .iter()
        .map(|addr| {
            json!({
                "addressType": addr.address_type,
                "address": addr.address,
            })
        })
        .collect();

    Ok(json!({
        "walletId": context.session.wallet_id,
        "organizationId": context.session.organization_id,
        "addresses": address_list,
    }))
}
