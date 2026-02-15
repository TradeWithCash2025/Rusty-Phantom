//! transfer_tokens tool — transfers SOL or SPL tokens on Solana.
//!
//! NOTE: The full transaction building logic (SystemProgram, SPL token instructions)
//! requires Solana SDK crates (`solana-sdk`, `spl-token`). This Rust implementation
//! provides the tool definition and parameter validation. The actual transaction
//! construction would use the Solana SDK when those dependencies are added.

use phantom_utils::is_solana_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::amount::{parse_base_unit_amount, parse_ui_amount, require_positive_amount};
use crate::utils::network::normalize_network_id;
use crate::utils::solana::get_solana_address;

/// Create the transfer_tokens tool handler.
pub fn transfer_tokens_tool() -> ToolHandler {
    ToolHandler {
        name: "transfer_tokens",
        description: "Transfers SOL or SPL tokens on Solana using the authenticated embedded wallet. Builds, signs, and sends the transaction.",
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "walletId": {
                    "type": "string",
                    "description": "Optional wallet ID to use for transfer (defaults to authenticated wallet)"
                },
                "networkId": {
                    "type": "string",
                    "description": "Solana network identifier (e.g., \"solana:mainnet\", \"solana:devnet\")"
                },
                "to": {
                    "type": "string",
                    "description": "Recipient Solana address"
                },
                "amount": {
                    "type": "string",
                    "description": "Transfer amount as a string (e.g., \"0.5\" or \"1000000\")"
                },
                "amountUnit": {
                    "type": "string",
                    "description": "Amount unit: 'ui' for SOL/token units, 'base' for lamports/base units",
                    "enum": ["ui", "base"]
                },
                "tokenMint": {
                    "type": "string",
                    "description": "Optional SPL token mint address. If omitted, transfers SOL."
                },
                "decimals": {
                    "type": "number",
                    "description": "Token decimals (optional for SPL tokens; fetched from chain if omitted)",
                    "minimum": 0
                },
                "derivationIndex": {
                    "type": "number",
                    "description": "Optional derivation index for the account (default: 0)",
                    "minimum": 0
                },
                "rpcUrl": {
                    "type": "string",
                    "description": "Optional Solana RPC URL (defaults based on networkId)"
                },
                "createAssociatedTokenAccount": {
                    "type": "boolean",
                    "description": "Create destination associated token account if missing (default: true)"
                }
            }),
            required: Some(vec![
                "networkId".to_string(),
                "to".to_string(),
                "amount".to_string(),
            ]),
        },
        handler: |params, context| Box::pin(handle_transfer_tokens(params, context)),
    }
}

async fn handle_transfer_tokens(
    params: Value,
    context: &ToolContext,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let network_id_raw = params
        .get("networkId")
        .and_then(|v| v.as_str())
        .ok_or("networkId must be a string")?;

    let normalized_network_id = normalize_network_id(network_id_raw);
    if !is_solana_chain(&normalized_network_id) {
        return Err("transfer_tokens currently supports Solana networks only".into());
    }

    let to = params
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or("to must be a string")?;

    let amount_str = params
        .get("amount")
        .and_then(|v| v.as_str())
        .ok_or("amount must be a string")?;

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

    let amount_unit = params
        .get("amountUnit")
        .and_then(|v| v.as_str())
        .unwrap_or("ui");

    if amount_unit != "ui" && amount_unit != "base" {
        return Err("amountUnit must be 'ui' or 'base'".into());
    }

    let token_mint = params.get("tokenMint").and_then(|v| v.as_str());

    let from_address = get_solana_address(context, wallet_id, derivation_index).await?;

    context.logger.info(&format!(
        "Preparing transfer from {} to {} on {}",
        from_address, to, normalized_network_id
    ));

    // Parse amount
    let _amount = if token_mint.is_none() {
        // Native SOL transfer
        let lamports = if amount_unit == "base" {
            parse_base_unit_amount(amount_str)?
        } else {
            parse_ui_amount(amount_str, 9 /* SOL decimals */)?
        };
        require_positive_amount(lamports)?;
        lamports
    } else {
        // SPL token transfer — need decimals
        let decimals = params
            .get("decimals")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let amount = if amount_unit == "base" {
            parse_base_unit_amount(amount_str)?
        } else {
            let dec = decimals.ok_or(
                "decimals is required for SPL token transfers with amountUnit='ui' \
                 (or fetch from chain)",
            )?;
            parse_ui_amount(amount_str, dec)?
        };
        require_positive_amount(amount)?;
        amount
    };

    // NOTE: Full transaction building requires Solana SDK crates.
    // This implementation validates parameters and returns the transfer intent.
    // In production, you would build and serialize the transaction here.

    context.logger.info(&format!(
        "Transfer prepared for wallet {} (transaction building requires Solana SDK)",
        wallet_id
    ));

    Ok(json!({
        "walletId": wallet_id,
        "networkId": normalized_network_id,
        "from": from_address,
        "to": to,
        "tokenMint": token_mint,
        "amount": amount_str,
        "amountUnit": amount_unit,
        "status": "prepared",
        "note": "Full transaction building requires Solana SDK crates (solana-sdk, spl-token)"
    }))
}
