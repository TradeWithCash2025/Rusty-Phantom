//! transfer_tokens tool — transfers SOL or SPL tokens on Solana.
//!
//! Builds, signs, and sends the transfer transaction using the authenticated
//! embedded wallet.  Handles both native SOL transfers (SystemProgram) and SPL
//! token transfers with automatic Associated Token Account (ATA) handling.

use phantom_base64url::base64url_encode;
use phantom_constants::NetworkId;
use phantom_utils::is_solana_chain;
use serde_json::{json, Value};

use super::types::{ToolContext, ToolHandler, ToolInputSchema};
use crate::utils::amount::{parse_base_unit_amount, parse_ui_amount, require_positive_amount};
use crate::utils::network::normalize_network_id;
use crate::utils::solana::get_solana_address;

// ---------------------------------------------------------------------------
// Solana RPC helpers
// ---------------------------------------------------------------------------

/// Default Solana RPC URLs keyed by canonical CAIP-2 network ID.
fn default_rpc_url(network_id: &str) -> Option<&'static str> {
    match network_id {
        s if s == NetworkId::SolanaMainnet.as_str() => Some("https://api.mainnet-beta.solana.com"),
        s if s == NetworkId::SolanaDevnet.as_str() => Some("https://api.devnet.solana.com"),
        s if s == NetworkId::SolanaTestnet.as_str() => Some("https://api.testnet.solana.com"),
        _ => None,
    }
}

/// Resolve the Solana RPC URL.  User-supplied `rpc_url` takes priority.
fn resolve_rpc_url(network_id: &str, rpc_url: Option<&str>) -> Result<String, String> {
    if let Some(url) = rpc_url {
        if !url.is_empty() {
            return Ok(url.to_string());
        }
    }
    default_rpc_url(network_id)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "rpcUrl is required for networkId \"{}\". Supported defaults: solana:mainnet, solana:devnet, solana:testnet",
                network_id
            )
        })
}

/// Make a Solana JSON-RPC request and return the `result` field.
async fn rpc_request(
    rpc_url: &str,
    method: &str,
    params: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
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

    resp.get("result")
        .cloned()
        .ok_or_else(|| "RPC response missing 'result' field".into())
}

/// Fetch the latest blockhash from the Solana cluster.
async fn get_latest_blockhash(
    rpc_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let result = rpc_request(
        rpc_url,
        "getLatestBlockhash",
        json!([{"commitment": "confirmed"}]),
    )
    .await?;

    result
        .get("value")
        .and_then(|v| v.get("blockhash"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to parse blockhash from RPC response".into())
}

/// Check whether a Solana account exists (non-null).
async fn account_exists(
    rpc_url: &str,
    address: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let result = rpc_request(
        rpc_url,
        "getAccountInfo",
        json!([address, {"encoding": "base64", "commitment": "confirmed"}]),
    )
    .await?;

    Ok(!result.get("value").is_none_or(|v| v.is_null()))
}

/// Fetch SPL token mint decimals from chain.
async fn get_mint_decimals(
    rpc_url: &str,
    mint_address: &str,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let result = rpc_request(
        rpc_url,
        "getAccountInfo",
        json!([mint_address, {"encoding": "base64", "commitment": "confirmed"}]),
    )
    .await?;

    let data_str = result
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .ok_or("Failed to fetch mint account data")?;

    let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_str)?;

    // SPL Mint layout: decimals is at offset 44 (1 byte)
    if data.len() < 82 {
        return Err("Mint account data too short".into());
    }
    Ok(data[44] as u32)
}

// ---------------------------------------------------------------------------
// Solana transaction building helpers (raw binary format)
// ---------------------------------------------------------------------------

/// Well-known Solana program IDs (32 bytes each).
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

// TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
const TOKEN_PROGRAM_ID: [u8; 32] = [
    0x06, 0xdd, 0xf6, 0xe1, 0xd7, 0x65, 0xa1, 0x93, 0xd9, 0xcb, 0xe1, 0x46, 0xce, 0xeb, 0x79, 0xac,
    0x1c, 0xb4, 0x85, 0xed, 0x5f, 0x5b, 0x37, 0x91, 0x3a, 0x8c, 0xf5, 0x85, 0x7e, 0xff, 0x00, 0xa9,
];

// ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL
const ASSOCIATED_TOKEN_PROGRAM_ID: [u8; 32] = [
    0x8c, 0x97, 0x25, 0x8f, 0x4e, 0x24, 0x89, 0xf1, 0xbb, 0x3d, 0x10, 0x29, 0x14, 0x8e, 0x0d, 0x83,
    0x0b, 0x5a, 0x13, 0x99, 0xda, 0xff, 0x10, 0x84, 0x04, 0x8e, 0x7b, 0xd8, 0xdb, 0xe9, 0xf8, 0x59,
];

/// Decode a base58-encoded public key to 32 bytes.
fn decode_pubkey(s: &str) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    let bytes = bs58::decode(s).into_vec()?;
    if bytes.len() != 32 {
        return Err(format!(
            "Invalid public key length: expected 32, got {}",
            bytes.len()
        )
        .into());
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

/// Derive the Associated Token Account (ATA) address.
///
/// ATA = PDA of \[wallet, TOKEN_PROGRAM_ID, mint\] seeded under ASSOCIATED_TOKEN_PROGRAM_ID.
fn derive_ata(
    wallet: &[u8; 32],
    mint: &[u8; 32],
) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    use sha2::{Digest, Sha256};

    // Try nonces from 255 down to 0 until we find a point NOT on the ed25519 curve.
    for nonce in (0..=255u8).rev() {
        let mut hasher = Sha256::new();
        hasher.update(wallet);
        hasher.update(TOKEN_PROGRAM_ID);
        hasher.update(mint);
        hasher.update([nonce]);
        hasher.update(ASSOCIATED_TOKEN_PROGRAM_ID);
        hasher.update(b"ProgramDerivedAddress");
        let hash = hasher.finalize();

        let mut candidate = [0u8; 32];
        candidate.copy_from_slice(&hash[..32]);

        // A valid PDA must NOT be a valid ed25519 public key.
        if !is_on_ed25519_curve(&candidate) {
            return Ok(candidate);
        }
    }
    Err("Failed to derive ATA: all nonces produced on-curve points".into())
}

/// Check if 32 bytes represent a point on the ed25519 curve.
fn is_on_ed25519_curve(bytes: &[u8; 32]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(bytes).is_ok()
}

/// Compact encoding of a u16 for Solana's transaction format.
fn encode_compact_u16(val: u16) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = val;
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
    out
}

/// A Solana instruction for building transactions.
struct SolInstruction {
    program_id: [u8; 32],
    accounts: Vec<SolAccountMeta>,
    data: Vec<u8>,
}

struct SolAccountMeta {
    pubkey: [u8; 32],
    is_signer: bool,
    is_writable: bool,
}

/// Build SystemProgram::Transfer instruction.
fn system_transfer_instruction(from: &[u8; 32], to: &[u8; 32], lamports: u64) -> SolInstruction {
    // SystemProgram Transfer instruction index = 2 (little-endian u32)
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());

    SolInstruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts: vec![
            SolAccountMeta {
                pubkey: *from,
                is_signer: true,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *to,
                is_signer: false,
                is_writable: true,
            },
        ],
        data,
    }
}

/// Build SPL Token `Transfer` instruction (index 3).
fn spl_transfer_instruction(
    source_ata: &[u8; 32],
    destination_ata: &[u8; 32],
    owner: &[u8; 32],
    amount: u64,
) -> SolInstruction {
    let mut data = Vec::with_capacity(9);
    data.push(3u8); // Transfer instruction index
    data.extend_from_slice(&amount.to_le_bytes());

    SolInstruction {
        program_id: TOKEN_PROGRAM_ID,
        accounts: vec![
            SolAccountMeta {
                pubkey: *source_ata,
                is_signer: false,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *destination_ata,
                is_signer: false,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *owner,
                is_signer: true,
                is_writable: false,
            },
        ],
        data,
    }
}

/// Build SPL Token `TransferChecked` instruction (index 12).
fn spl_transfer_checked_instruction(
    source_ata: &[u8; 32],
    mint: &[u8; 32],
    destination_ata: &[u8; 32],
    owner: &[u8; 32],
    amount: u64,
    decimals: u8,
) -> SolInstruction {
    let mut data = Vec::with_capacity(10);
    data.push(12u8); // TransferChecked instruction index
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);

    SolInstruction {
        program_id: TOKEN_PROGRAM_ID,
        accounts: vec![
            SolAccountMeta {
                pubkey: *source_ata,
                is_signer: false,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *mint,
                is_signer: false,
                is_writable: false,
            },
            SolAccountMeta {
                pubkey: *destination_ata,
                is_signer: false,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *owner,
                is_signer: true,
                is_writable: false,
            },
        ],
        data,
    }
}

/// Build CreateAssociatedTokenAccount instruction.
fn create_ata_instruction(
    funding_address: &[u8; 32],
    ata_address: &[u8; 32],
    wallet_address: &[u8; 32],
    mint: &[u8; 32],
) -> SolInstruction {
    SolInstruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: vec![
            SolAccountMeta {
                pubkey: *funding_address,
                is_signer: true,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *ata_address,
                is_signer: false,
                is_writable: true,
            },
            SolAccountMeta {
                pubkey: *wallet_address,
                is_signer: false,
                is_writable: false,
            },
            SolAccountMeta {
                pubkey: *mint,
                is_signer: false,
                is_writable: false,
            },
            SolAccountMeta {
                pubkey: SYSTEM_PROGRAM_ID,
                is_signer: false,
                is_writable: false,
            },
            SolAccountMeta {
                pubkey: TOKEN_PROGRAM_ID,
                is_signer: false,
                is_writable: false,
            },
        ],
        data: vec![],
    }
}

/// Serialize a legacy Solana transaction from a set of instructions, a fee
/// payer, and a recent blockhash.
///
/// Returns raw serialized transaction bytes (unsigned — signature slots are
/// filled with zeros; Phantom's backend performs the actual signing).
fn serialize_transaction(
    instructions: &[SolInstruction],
    fee_payer: &[u8; 32],
    recent_blockhash: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // 1. Collect unique accounts and determine their roles.
    let mut accounts: Vec<([u8; 32], bool, bool)> = Vec::new(); // (key, is_signer, is_writable)

    // Fee payer is always the first account (signer + writable).
    accounts.push((*fee_payer, true, true));

    for ix in instructions {
        for meta in &ix.accounts {
            if let Some(existing) = accounts.iter_mut().find(|(k, _, _)| k == &meta.pubkey) {
                existing.1 |= meta.is_signer;
                existing.2 |= meta.is_writable;
            } else {
                accounts.push((meta.pubkey, meta.is_signer, meta.is_writable));
            }
        }
        // Program IDs are non-signer, non-writable accounts.
        if !accounts.iter().any(|(k, _, _)| k == &ix.program_id) {
            accounts.push((ix.program_id, false, false));
        }
    }

    // 2. Sort accounts: signers-writable, signers-readonly, non-signers-writable,
    //    non-signers-readonly — but fee payer must remain first.
    let fee_payer_entry = accounts.remove(0);
    accounts.sort_by(|a, b| {
        let order = |signer: bool, writable: bool| -> u8 {
            match (signer, writable) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            }
        };
        order(a.1, a.2).cmp(&order(b.1, b.2))
    });
    accounts.insert(0, fee_payer_entry);

    let num_required_signatures = accounts.iter().filter(|(_, s, _)| *s).count() as u8;
    let num_readonly_signed = accounts.iter().filter(|(_, s, w)| *s && !*w).count() as u8;
    let num_readonly_unsigned = accounts.iter().filter(|(_, s, w)| !*s && !*w).count() as u8;

    // 3. Build the message.
    let mut message = Vec::new();
    message.push(num_required_signatures);
    message.push(num_readonly_signed);
    message.push(num_readonly_unsigned);

    // Account addresses.
    message.extend_from_slice(&encode_compact_u16(accounts.len() as u16));
    for (key, _, _) in &accounts {
        message.extend_from_slice(key);
    }

    // Recent blockhash (32 bytes).
    let bh_bytes = bs58::decode(recent_blockhash).into_vec()?;
    if bh_bytes.len() != 32 {
        return Err("Invalid blockhash length".into());
    }
    message.extend_from_slice(&bh_bytes);

    // Instructions.
    message.extend_from_slice(&encode_compact_u16(instructions.len() as u16));
    for ix in instructions {
        // Program ID index.
        let program_idx = accounts
            .iter()
            .position(|(k, _, _)| k == &ix.program_id)
            .ok_or("Program ID not found in account list")? as u8;
        message.push(program_idx);

        // Account indices.
        message.extend_from_slice(&encode_compact_u16(ix.accounts.len() as u16));
        for meta in &ix.accounts {
            let idx = accounts
                .iter()
                .position(|(k, _, _)| k == &meta.pubkey)
                .ok_or("Account not found in account list")? as u8;
            message.push(idx);
        }

        // Instruction data.
        message.extend_from_slice(&encode_compact_u16(ix.data.len() as u16));
        message.extend_from_slice(&ix.data);
    }

    // 4. Build the full transaction: signatures + message.
    let mut tx = Vec::new();
    tx.extend_from_slice(&encode_compact_u16(num_required_signatures as u16));
    // Placeholder signatures (64 zero bytes each — unsigned).
    for _ in 0..num_required_signatures {
        tx.extend_from_slice(&[0u8; 64]);
    }
    tx.extend_from_slice(&message);

    Ok(tx)
}

// ---------------------------------------------------------------------------
// Tool definition and handler
// ---------------------------------------------------------------------------

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

    // Validate derivationIndex if provided.
    if let Some(deriv_val) = params.get("derivationIndex") {
        if !deriv_val.is_null() {
            match deriv_val.as_f64() {
                Some(f) => {
                    if f.fract() != 0.0 || f < 0.0 {
                        return Err("derivationIndex must be a non-negative integer".into());
                    }
                }
                None => {
                    return Err("derivationIndex must be a non-negative integer".into());
                }
            }
        }
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
    let create_ata = params
        .get("createAssociatedTokenAccount")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Resolve RPC URL.
    let rpc_url = resolve_rpc_url(
        &normalized_network_id,
        params.get("rpcUrl").and_then(|v| v.as_str()),
    )?;

    // Get sender address.
    let from_address = get_solana_address(context, wallet_id, derivation_index).await?;
    let from_pubkey = decode_pubkey(&from_address)?;
    let to_pubkey = decode_pubkey(to)?;

    context.logger.info(&format!(
        "Preparing transfer from {} to {} on {}",
        from_address, to, normalized_network_id
    ));

    // Build transaction instructions.
    let mut instructions: Vec<SolInstruction> = Vec::new();

    if let Some(mint_str) = token_mint {
        // ---------------------------------------------------------------
        // SPL token transfer
        // ---------------------------------------------------------------
        let mint_pubkey = decode_pubkey(mint_str)?;

        let source_ata = derive_ata(&from_pubkey, &mint_pubkey)?;
        let destination_ata = derive_ata(&to_pubkey, &mint_pubkey)?;

        // Verify source ATA exists.
        let source_ata_b58 = bs58::encode(&source_ata).into_string();
        if !account_exists(&rpc_url, &source_ata_b58).await? {
            return Err("Source associated token account not found for this wallet".into());
        }

        // Check destination ATA and optionally create it.
        let dest_ata_b58 = bs58::encode(&destination_ata).into_string();
        if !account_exists(&rpc_url, &dest_ata_b58).await? {
            if create_ata {
                instructions.push(create_ata_instruction(
                    &from_pubkey,
                    &destination_ata,
                    &to_pubkey,
                    &mint_pubkey,
                ));
            } else {
                return Err("Destination associated token account does not exist".into());
            }
        }

        // Determine decimals.
        let mut decimals: Option<u32> = params
            .get("decimals")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        if let Some(d) = decimals {
            if d > 255 {
                return Err("decimals must be a non-negative integer <= 255".into());
            }
        }

        if amount_unit == "ui" && decimals.is_none() {
            decimals = Some(get_mint_decimals(&rpc_url, mint_str).await?);
        }

        if amount_unit == "ui" && decimals.is_none() {
            return Err("Unable to determine token decimals".into());
        }

        let amount_base_units = if amount_unit == "base" {
            parse_base_unit_amount(amount_str)?
        } else {
            parse_ui_amount(amount_str, decimals.unwrap())?
        };
        require_positive_amount(amount_base_units)?;

        // Use TransferChecked when we have decimals, otherwise plain Transfer.
        if amount_unit == "base" && decimals.is_none() {
            instructions.push(spl_transfer_instruction(
                &source_ata,
                &destination_ata,
                &from_pubkey,
                amount_base_units as u64,
            ));
        } else {
            instructions.push(spl_transfer_checked_instruction(
                &source_ata,
                &mint_pubkey,
                &destination_ata,
                &from_pubkey,
                amount_base_units as u64,
                decimals.unwrap() as u8,
            ));
        }
    } else {
        // ---------------------------------------------------------------
        // Native SOL transfer via SystemProgram
        // ---------------------------------------------------------------
        let lamports = if amount_unit == "base" {
            parse_base_unit_amount(amount_str)?
        } else {
            parse_ui_amount(amount_str, 9 /* SOL decimals */)?
        };
        require_positive_amount(lamports)?;

        instructions.push(system_transfer_instruction(
            &from_pubkey,
            &to_pubkey,
            lamports as u64,
        ));
    }

    // Fetch recent blockhash and build the transaction.
    let blockhash = get_latest_blockhash(&rpc_url).await?;
    let serialized = serialize_transaction(&instructions, &from_pubkey, &blockhash)?;
    let encoded = base64url_encode(&serialized);

    // Sign and send via Phantom client.
    let result = context
        .client
        .sign_and_send_transaction(&phantom_client::SignAndSendTransactionParams {
            wallet_id: wallet_id.to_string(),
            transaction: encoded,
            network_id: normalized_network_id.clone(),
            derivation_index,
            account: Some(from_address.clone()),
        })
        .await?;

    context
        .logger
        .info(&format!("Transfer submitted for wallet {}", wallet_id));

    Ok(json!({
        "walletId": wallet_id,
        "networkId": normalized_network_id,
        "from": from_address,
        "to": to,
        "tokenMint": token_mint,
        "signature": result.hash,
        "rawTransaction": result.raw_transaction,
    }))
}
