//! buy_token tool — fetches a swap quote from the Phantom quotes API.

use phantom_utils::is_solana_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::amount::{parse_base_unit_amount, parse_ui_amount, require_positive_amount};
use crate::utils::network::{normalize_network_id, normalize_swapper_chain_id};
use crate::utils::solana::get_solana_address;

const DEFAULT_QUOTES_API_URL: &str = "https://api.phantom.app/swap/v2/quotes";

/// Default Solana RPC URLs keyed by swapper chain ID.
const DEFAULT_SOLANA_RPC_URLS: &[(&str, &str)] = &[
    ("solana:101", "https://api.mainnet-beta.solana.com"),
    ("solana:103", "https://api.devnet.solana.com"),
    ("solana:102", "https://api.testnet.solana.com"),
];

/// Resolve the Solana RPC URL for on-chain lookups.
///
/// Priority: override parameter > default URL for chain ID.
fn resolve_solana_rpc_url(chain_id: &str, override_url: Option<&str>) -> Result<String, String> {
    if let Some(url) = override_url {
        if !url.is_empty() {
            validate_https_url(url, "Solana RPC")?;
            return Ok(url.to_string());
        }
    }
    for &(id, url) in DEFAULT_SOLANA_RPC_URLS {
        if id == chain_id {
            return Ok(url.to_string());
        }
    }
    let supported: Vec<&str> = DEFAULT_SOLANA_RPC_URLS.iter().map(|(id, _)| *id).collect();
    Err(format!(
        "rpcUrl is required for chainId \"{}\". Supported defaults: {}",
        chain_id,
        supported.join(", ")
    ))
}

/// Fetch SPL token mint decimals from chain via getAccountInfo RPC call.
async fn get_mint_decimals(
    rpc_url: &str,
    mint_address: &str,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [mint_address, {"encoding": "base64", "commitment": "confirmed"}],
    });

    let resp = client
        .post(rpc_url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .json::<Value>()
        .await?;

    if let Some(err) = resp.get("error") {
        return Err(format!("RPC error: {}", err).into());
    }

    let result = resp
        .get("result")
        .ok_or("RPC response missing 'result' field")?;

    let data_str = result
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .ok_or("Failed to fetch mint account data")?;

    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD
        .decode(data_str)
        .map_err(|e| format!("Failed to decode mint account data: {}", e))?;

    // SPL Mint layout: decimals is at offset 44 (1 byte)
    if data.len() < 82 {
        return Err("Mint account data too short".into());
    }
    Ok(data[44] as u32)
}

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
                "base64EncodedTx": {
                    "type": "boolean",
                    "description": "Request base64-encoded transaction data in the quote response"
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

    let derivation_index = match params.get("derivationIndex") {
        Some(serde_json::Value::Null) | None => None,
        Some(v) => {
            let idx = v.as_u64().ok_or_else(|| {
                format!("derivationIndex must be a non-negative integer, got: {}", v)
            })?;
            Some(idx as u32)
        }
    };

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

    if buy_token_is_native && buy_token_mint.is_some() {
        return Err("buyTokenMint must be omitted when buyTokenIsNative is true".into());
    }

    if !sell_token_is_native && sell_token_mint.is_none() {
        return Err("sellTokenMint is required unless sellTokenIsNative is true".into());
    }

    if sell_token_is_native && sell_token_mint.is_some() {
        return Err("sellTokenMint must be omitted when sellTokenIsNative is true".into());
    }

    // Validate mint addresses are valid Solana public keys (base58, 32 bytes)
    if let Some(mint) = buy_token_mint {
        if bs58::decode(mint).into_vec().map(|v| v.len()).unwrap_or(0) != 32 {
            return Err("buyTokenMint must be a valid Solana address".into());
        }
    }

    if let Some(mint) = sell_token_mint {
        if bs58::decode(mint).into_vec().map(|v| v.len()).unwrap_or(0) != 32 {
            return Err("sellTokenMint must be a valid Solana address".into());
        }
    }

    let taker = if let Some(t) = params.get("taker").and_then(|v| v.as_str()) {
        t.to_string()
    } else {
        get_solana_address(context, wallet_id, derivation_index).await?
    };

    // Validate taker is a valid Solana address
    if bs58::decode(&taker).into_vec().map(|v| v.len()).unwrap_or(0) != 32 {
        return Err("taker must be a valid Solana address".into());
    }

    let exact_out = params
        .get("exactOut")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Parse amount to base units
    let amount_base_units = if amount_unit == "base" {
        parse_base_unit_amount(amount_str)?
    } else {
        let decimals: u32 = if exact_out {
            if buy_token_is_native {
                9
            } else if let Some(d) = params
                .get("buyTokenDecimals")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
            {
                d
            } else if let Some(mint) = buy_token_mint {
                // Auto-fetch decimals from chain
                let rpc_url = resolve_solana_rpc_url(
                    &swapper_chain_id,
                    params.get("rpcUrl").and_then(|v| v.as_str()),
                )?;
                get_mint_decimals(&rpc_url, mint).await?
            } else {
                return Err("buyTokenMint is required to lookup decimals".into());
            }
        } else if sell_token_is_native {
            9
        } else if let Some(d) = params
            .get("sellTokenDecimals")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
        {
            d
        } else if let Some(mint) = sell_token_mint {
            // Auto-fetch decimals from chain
            let rpc_url = resolve_solana_rpc_url(
                &swapper_chain_id,
                params.get("rpcUrl").and_then(|v| v.as_str()),
            )?;
            get_mint_decimals(&rpc_url, mint).await?
        } else {
            return Err("sellTokenMint is required to lookup decimals".into());
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
        if !slippage.is_finite() || slippage < 0.0 || slippage > 100.0 {
            return Err("slippageTolerance must be a number between 0 and 100".into());
        }
        body["slippageTolerance"] = json!(slippage);
    }

    if let Some(exact_out_val) = params.get("exactOut").and_then(|v| v.as_bool()) {
        body["exactOut"] = json!(exact_out_val);
    }

    if let Some(auto_slippage) = params.get("autoSlippage").and_then(|v| v.as_bool()) {
        body["autoSlippage"] = json!(auto_slippage);
    }

    if let Some(base64_encoded_tx) = params.get("base64EncodedTx").and_then(|v| v.as_bool()) {
        body["base64EncodedTx"] = json!(base64_encoded_tx);
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

    // Decode transaction data based on encoding format
    let base64_encoded_tx = params.get("base64EncodedTx").and_then(|v| v.as_bool()).unwrap_or(false);
    let decoded: Vec<u8> = if base64_encoded_tx {
        // If base64EncodedTx is true, decode as base64
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(transaction_data)
            .map_err(|e| format!("Failed to decode base64 transaction data: {}", e))?;
        if bytes.is_empty() {
            return Err("Failed to decode base64 transaction data".into());
        }
        bytes
    } else {
        // Try base58 first, then fall back to base64
        if let Ok(bytes) = bs58::decode(transaction_data).into_vec() {
            bytes
        } else {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(transaction_data)
                .map_err(|e| format!("Failed to decode transaction data: {}", e))?;
            if bytes.is_empty() {
                return Err("Failed to decode transaction data".into());
            }
            bytes
        }
    };

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
