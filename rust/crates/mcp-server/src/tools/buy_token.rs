//! buy_token tool — fetches a swap quote from the Phantom quotes API.

use phantom_utils::is_solana_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::amount::{parse_base_unit_amount, parse_ui_amount, require_positive_amount};
use crate::utils::network::{normalize_network_id, normalize_swapper_chain_id};
use crate::utils::solana::get_solana_address;

const DEFAULT_QUOTES_API_URL: &str = "https://api.phantom.app/swap/v2/quotes";

/// Validate that a URL uses HTTPS protocol.
fn validate_https_url(url: &str, context_name: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!(
            "{} URL must use HTTPS protocol, got: {}",
            context_name, url
        ));
    }
    // Basic hostname check
    let after_scheme = &url["https://".len()..];
    if after_scheme.is_empty() || after_scheme.starts_with('/') || after_scheme.starts_with(':') {
        return Err(format!("{} URL missing hostname: {}", context_name, url));
    }
    Ok(())
}

/// Resolve the quotes API URL.
fn resolve_quotes_api_url(override_url: Option<&str>) -> Result<String, String> {
    let url = if let Some(u) = override_url {
        u.to_string()
    } else if let Ok(env_url) = std::env::var("PHANTOM_QUOTES_API_URL") {
        env_url
    } else {
        DEFAULT_QUOTES_API_URL.to_string()
    };

    validate_https_url(&url, "Quotes API")?;
    Ok(url)
}

/// Create the buy_token tool handler.
pub fn buy_token_tool() -> ToolHandler {
    ToolHandler {
        name: "buy_token",
        description: "Fetches a swap quote from Phantom's quotes API for buying a token (Solana only). By default, this tool only fetches a quote and does not submit a transaction. Pass execute: true to sign and send the transaction.",
        input_schema: ToolInputSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "walletId": {
                    "type": "string",
                    "description": "Optional wallet ID to use for the taker address (defaults to authenticated wallet)"
                },
                "networkId": {
                    "type": "string",
                    "description": "Solana network identifier (e.g., \"solana:mainnet\", \"solana:devnet\")"
                },
                "buyTokenMint": {
                    "type": "string",
                    "description": "Mint address of the token to buy (omit if buying native SOL)"
                },
                "buyTokenIsNative": {
                    "type": "boolean",
                    "description": "Set true to buy native SOL (default: false)"
                },
                "sellTokenMint": {
                    "type": "string",
                    "description": "Mint address of the token to sell (omit if selling native SOL)"
                },
                "sellTokenIsNative": {
                    "type": "boolean",
                    "description": "Set true to sell native SOL (default: true if sellTokenMint not provided)"
                },
                "amount": {
                    "type": "string",
                    "description": "The amount to swap as a string"
                },
                "amountUnit": {
                    "type": "string",
                    "description": "Amount unit: 'ui' for token units, 'base' for atomic units (default: 'base')",
                    "enum": ["ui", "base"]
                },
                "buyTokenDecimals": {
                    "type": "number",
                    "description": "Decimals for the buy token",
                    "minimum": 0
                },
                "sellTokenDecimals": {
                    "type": "number",
                    "description": "Decimals for the sell token",
                    "minimum": 0
                },
                "slippageTolerance": {
                    "type": "number",
                    "description": "Slippage tolerance in percent (0-100)",
                    "minimum": 0,
                    "maximum": 100
                },
                "exactOut": {
                    "type": "boolean",
                    "description": "If true, amount is treated as buy amount instead of sell amount"
                },
                "autoSlippage": {
                    "type": "boolean",
                    "description": "Enable auto slippage calculation"
                },
                "execute": {
                    "type": "boolean",
                    "description": "If true, sign and send the first quote transaction after fetching"
                },
                "taker": {
                    "type": "string",
                    "description": "Taker address (defaults to wallet's Solana address)"
                },
                "rpcUrl": {
                    "type": "string",
                    "description": "Optional Solana RPC URL"
                },
                "quoteApiUrl": {
                    "type": "string",
                    "description": "Optional quotes API URL override"
                },
                "derivationIndex": {
                    "type": "number",
                    "description": "Optional derivation index for the taker address (default: 0)",
                    "minimum": 0
                }
            }),
            required: Some(vec!["amount".to_string()]),
        },
        handler: |params, context| Box::pin(handle_buy_token(params, context)),
    }
}

async fn handle_buy_token(
    params: Value,
    context: &ToolContext,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let network_id = params
        .get("networkId")
        .and_then(|v| v.as_str())
        .unwrap_or("solana:mainnet");

    let _normalized_network_id = normalize_network_id(network_id);
    let swapper_chain_id = normalize_swapper_chain_id(network_id);

    if !is_solana_chain(network_id) && !is_solana_chain(&swapper_chain_id) {
        return Err("buy_token currently supports Solana networks only".into());
    }

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
        .unwrap_or("base");

    if amount_unit != "ui" && amount_unit != "base" {
        return Err("amountUnit must be 'ui' or 'base'".into());
    }

    let buy_token_is_native = params
        .get("buyTokenIsNative")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let sell_token_mint = params.get("sellTokenMint").and_then(|v| v.as_str());
    let sell_token_is_native = params
        .get("sellTokenIsNative")
        .and_then(|v| v.as_bool())
        .unwrap_or(sell_token_mint.is_none());

    let buy_token_mint = params.get("buyTokenMint").and_then(|v| v.as_str());

    if !buy_token_is_native && buy_token_mint.is_none() {
        return Err("buyTokenMint is required unless buyTokenIsNative is true".into());
    }

    if !sell_token_is_native && sell_token_mint.is_none() {
        return Err("sellTokenMint is required unless sellTokenIsNative is true".into());
    }

    let taker = if let Some(t) = params.get("taker").and_then(|v| v.as_str()) {
        t.to_string()
    } else {
        get_solana_address(context, wallet_id, derivation_index).await?
    };

    let exact_out = params
        .get("exactOut")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse amount to base units
    let amount_base_units = if amount_unit == "base" {
        parse_base_unit_amount(amount_str)?
    } else {
        let decimals = if exact_out {
            if buy_token_is_native {
                9
            } else {
                params
                    .get("buyTokenDecimals")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .ok_or("buyTokenDecimals is required for UI amount with exactOut")?
            }
        } else if sell_token_is_native {
            9
        } else {
            params
                .get("sellTokenDecimals")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or("sellTokenDecimals is required for UI amount")?
        };
        parse_ui_amount(amount_str, decimals)?
    };

    require_positive_amount(amount_base_units)?;

    let quote_api_url = resolve_quotes_api_url(
        params.get("quoteApiUrl").and_then(|v| v.as_str()),
    )?;

    // Build quote request body
    let buy_token = if buy_token_is_native {
        json!({ "chainId": swapper_chain_id, "resourceType": "nativeToken", "slip44": "501" })
    } else {
        json!({ "chainId": swapper_chain_id, "resourceType": "address", "address": buy_token_mint })
    };

    let sell_token = if sell_token_is_native {
        json!({ "chainId": swapper_chain_id, "resourceType": "nativeToken", "slip44": "501" })
    } else {
        json!({ "chainId": swapper_chain_id, "resourceType": "address", "address": sell_token_mint })
    };

    let mut body = json!({
        "taker": { "chainId": swapper_chain_id, "resourceType": "address", "address": taker },
        "buyToken": buy_token,
        "sellToken": sell_token,
    });

    if exact_out {
        body["buyAmount"] = json!(amount_base_units.to_string());
    } else {
        body["sellAmount"] = json!(amount_base_units.to_string());
    }

    if let Some(slippage) = params.get("slippageTolerance").and_then(|v| v.as_f64()) {
        body["slippageTolerance"] = json!(slippage);
    }

    if let Some(exact_out_val) = params.get("exactOut").and_then(|v| v.as_bool()) {
        body["exactOut"] = json!(exact_out_val);
    }

    if let Some(auto_slippage) = params.get("autoSlippage").and_then(|v| v.as_bool()) {
        body["autoSlippage"] = json!(auto_slippage);
    }

    context.logger.info("Requesting quote from API");

    // Send quote request
    let client = reqwest::Client::new();
    let response = client
        .post(&quote_api_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Quote API request timed out after 10 seconds".to_string()
            } else {
                format!("Quote API request failed: {}", e)
            }
        })?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();

    let response_json: Value = serde_json::from_str(&response_text).unwrap_or(Value::Null);

    if !status.is_success() {
        let message = if response_json.is_null() {
            response_text
        } else {
            response_json.to_string()
        };
        return Err(format!("Quote API error ({}): {}", status, message).into());
    }

    let execute = params
        .get("execute")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !execute {
        return Ok(json!({
            "quoteRequest": body,
            "quoteResponse": response_json,
        }));
    }

    // Execute mode: sign and send the first transaction
    let quotes = response_json
        .get("quotes")
        .and_then(|v| v.as_array())
        .ok_or("Quote response has unexpected format: missing quotes array")?;

    let transaction_data = quotes
        .first()
        .and_then(|q| q.get("transactionData"))
        .and_then(|td| td.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .ok_or("Quote response missing transaction data in first quote")?;

    // Decode and re-encode as base64url for the client
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(transaction_data)
        .or_else(|_| {
            bs58::decode(transaction_data)
                .into_vec()
                .map_err(|e| base64::DecodeError::InvalidByte(0, e.to_string().as_bytes()[0]))
        })
        .map_err(|e| format!("Failed to decode transaction data: {}", e))?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&decoded);

    let result = context
        .client
        .sign_and_send_transaction(&phantom_client::SignAndSendTransactionParams {
            wallet_id: wallet_id.to_string(),
            transaction: encoded,
            network_id: _normalized_network_id,
            derivation_index,
            account: Some(taker),
        })
        .await?;

    Ok(json!({
        "quoteRequest": body,
        "quoteResponse": response_json,
        "execution": {
            "signature": result.hash,
            "rawTransaction": result.raw_transaction,
        },
    }))
}
