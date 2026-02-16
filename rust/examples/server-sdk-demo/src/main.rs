//! Phantom Server SDK Demo
//!
//! Demonstrates the full Solana wallet lifecycle:
//! 1. Initializing the Server SDK
//! 2. Creating a wallet
//! 3. Checking wallet balance via Solana RPC
//! 4. Building a self-transfer transaction (raw binary format)
//! 5. Signing and sending the transaction via the SDK
//! 6. Polling for transaction confirmation
//! 7. Verifying the final balance
//!
//! Usage:
//!   cargo run --bin phantom-server-sdk-demo
//!
//! Required environment variables:
//!   ORGANIZATION_ID          - Your organization ID
//!   ORGANIZATION_PRIVATE_KEY - Base58-encoded Ed25519 private key
//!   APP_ID                   - Application ID
//!
//! Optional environment variables:
//!   WALLET_API               - API base URL (defaults to staging)
//!   SOLANA_RPC_URL           - Solana RPC endpoint (defaults to devnet)
//!   NETWORK                  - "devnet" or "mainnet" (defaults to "devnet")

use phantom_base64url::base64url_encode;
use phantom_constants::NetworkId;
use phantom_server_sdk::{
    ServerSdk, ServerSdkConfig, ServerSignAndSendTransactionParams, ServerSignMessageParams,
};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Configuration helpers
// ---------------------------------------------------------------------------

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        eprintln!("Missing required environment variable: {}", key);
        eprintln!("\nRequired variables:");
        eprintln!("  ORGANIZATION_ID          - Your organization ID");
        eprintln!("  ORGANIZATION_PRIVATE_KEY - Base58-encoded Ed25519 private key");
        eprintln!("  APP_ID                   - Application ID");
        eprintln!("\nOptional:");
        eprintln!("  WALLET_API               - API base URL");
        eprintln!("  SOLANA_RPC_URL           - Solana RPC endpoint");
        eprintln!("  NETWORK                  - \"devnet\" or \"mainnet\"");
        std::process::exit(1);
    })
}

// ---------------------------------------------------------------------------
// Solana RPC helpers (using reqwest directly)
// ---------------------------------------------------------------------------

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

/// Make a Solana JSON-RPC request and return the `result` field.
async fn rpc_request(
    client: &reqwest::Client,
    rpc_url: &str,
    method: &str,
    params: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
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

/// Fetch the SOL balance (in lamports) for an address.
async fn get_balance(
    client: &reqwest::Client,
    rpc_url: &str,
    address: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let result = rpc_request(
        client,
        rpc_url,
        "getBalance",
        json!([address, {"commitment": "confirmed"}]),
    )
    .await?;

    result
        .get("value")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Failed to parse balance from RPC response".into())
}

/// Fetch the latest blockhash from the Solana cluster.
async fn get_latest_blockhash(
    client: &reqwest::Client,
    rpc_url: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let result = rpc_request(
        client,
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

/// Fetch signature status for transaction confirmation polling.
async fn get_signature_status(
    client: &reqwest::Client,
    rpc_url: &str,
    signature: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let result = rpc_request(
        client,
        rpc_url,
        "getSignatureStatuses",
        json!([[signature]]),
    )
    .await?;

    let statuses = result
        .get("value")
        .and_then(|v| v.as_array())
        .ok_or("Failed to parse signature statuses")?;

    Ok(statuses
        .first()
        .and_then(|v| if v.is_null() { None } else { Some(v.clone()) }))
}

// ---------------------------------------------------------------------------
// Raw Solana transaction building (same approach as mcp-server)
// ---------------------------------------------------------------------------

/// Well-known Solana System Program ID (all zeros).
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

/// Decode a base58-encoded public key to 32 bytes.
fn decode_pubkey(s: &str) -> Result<[u8; 32], Box<dyn std::error::Error>> {
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

/// Build a SystemProgram::Transfer instruction.
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

/// Build a ComputeBudgetProgram::SetComputeUnitPrice instruction.
fn set_compute_unit_price_instruction(micro_lamports: u64) -> SolInstruction {
    // ComputeBudgetProgram ID: ComputeBudget111111111111111111111111111111
    let program_id: [u8; 32] = {
        let bytes = bs58::decode("ComputeBudget111111111111111111111111111111")
            .into_vec()
            .expect("valid base58 for ComputeBudget program ID");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    };

    // SetComputeUnitPrice instruction: discriminator byte 3, then u64 LE
    let mut data = Vec::with_capacity(9);
    data.push(3u8);
    data.extend_from_slice(&micro_lamports.to_le_bytes());

    SolInstruction {
        program_id,
        accounts: vec![],
        data,
    }
}

/// Serialize a legacy Solana transaction from a set of instructions, a fee
/// payer, and a recent blockhash.
///
/// Returns raw serialized transaction bytes (unsigned -- signature slots are
/// filled with zeros; Phantom's backend performs the actual signing).
fn serialize_transaction(
    instructions: &[SolInstruction],
    fee_payer: &[u8; 32],
    recent_blockhash: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
    //    non-signers-readonly -- but fee payer must remain first.
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
    // Placeholder signatures (64 zero bytes each -- unsigned).
    for _ in 0..num_required_signatures {
        tx.extend_from_slice(&[0u8; 64]);
    }
    tx.extend_from_slice(&message);

    Ok(tx)
}

// ---------------------------------------------------------------------------
// Explorer URL helper
// ---------------------------------------------------------------------------

fn explorer_url(signature: &str, network: &str) -> String {
    format!(
        "https://explorer.solana.com/tx/{}?cluster={}",
        signature, network
    )
}

// ---------------------------------------------------------------------------
// Main demo
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    println!("Phantom Server SDK Demo\n");

    // Load configuration from environment
    let organization_id = env_required("ORGANIZATION_ID");
    let api_private_key = env_required("ORGANIZATION_PRIVATE_KEY");
    let app_id = env_required("APP_ID");
    let api_base_url = env_or("WALLET_API", "https://staging-api.phantom.app/v1/wallets");
    let solana_rpc_url = env_or("SOLANA_RPC_URL", "https://api.devnet.solana.com");
    let network = env_or("NETWORK", "devnet");

    let network_id = if network == "mainnet" {
        NetworkId::SolanaMainnet
    } else {
        NetworkId::SolanaDevnet
    };

    // Step 1: Initialize SDK
    println!("1. Initializing Server SDK...");
    let sdk = ServerSdk::new(ServerSdkConfig {
        organization_id: organization_id.clone(),
        app_id,
        api_base_url: Some(api_base_url.clone()),
        api_private_key,
        solana_rpc_url: Some(solana_rpc_url.clone()),
    });
    println!("   Connected to: {}", api_base_url);
    println!("   Organization: {}", organization_id);
    println!("   Solana RPC: {}", solana_rpc_url);
    println!("   Network: {}\n", network);

    let http = reqwest::Client::new();

    // Step 2: Create a wallet
    println!("2. Creating a new wallet...");
    let wallet_name = format!(
        "Demo Wallet {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let wallet = match sdk.create_wallet(&wallet_name).await {
        Ok(w) => w,
        Err(e) => {
            eprintln!("   Failed to create wallet: {}", e);
            eprintln!("   (This is expected without valid API credentials)");
            std::process::exit(1);
        }
    };

    println!("   Wallet created:");
    println!("   ID: {}", wallet.wallet_id);
    println!("   Name: {}", wallet_name);
    for addr in &wallet.addresses {
        println!("   {} : {}", addr.address_type, addr.address);
    }

    // Find Solana address
    let solana_address = wallet
        .addresses
        .iter()
        .find(|a| a.address_type == "Solana")
        .map(|a| a.address.clone());

    let sol_addr = match solana_address {
        Some(addr) => {
            println!("\n   Solana address: {}", addr);
            addr
        }
        None => {
            eprintln!("   No Solana address found in wallet");
            std::process::exit(1);
        }
    };

    // Step 3: Sign a message
    println!("\n3. Signing a message...");
    let network_id_str = serde_json::to_value(network_id)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| network_id.as_str().to_string());

    match sdk
        .sign_message(ServerSignMessageParams {
            wallet_id: wallet.wallet_id.clone(),
            message: "Hello from Phantom Server SDK (Rust)!".to_string(),
            network_id: network_id_str.clone(),
            derivation_index: None,
        })
        .await
    {
        Ok(result) => {
            println!("   Message signed successfully!");
            println!("   Signature: {}", result.signature);
            println!("   Raw (base64url): {}", result.raw_signature);
        }
        Err(e) => println!("   Failed to sign message: {}", e),
    }

    // Step 4: Check wallet balance
    println!("\n4. Checking wallet balance...");
    let mut balance = match get_balance(&http, &solana_rpc_url, &sol_addr).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("   Failed to check balance: {}", e);
            std::process::exit(1);
        }
    };
    println!(
        "   Balance: {} SOL ({} lamports)",
        balance as f64 / LAMPORTS_PER_SOL,
        balance
    );

    if balance == 0 {
        if network == "devnet" {
            println!("\n   Wallet has 0 balance. Please fund the wallet to continue:");
            println!("   1. Request devnet SOL from: https://faucet.solana.com/");
            println!("   2. Use address: {}", sol_addr);
        } else {
            println!("\n   Wallet has 0 balance. Please fund the wallet to continue:");
            println!("   1. Send some SOL to address: {}", sol_addr);
        }
        println!("\n   Waiting for funds...");

        let mut check_count = 0u32;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            match get_balance(&http, &solana_rpc_url, &sol_addr).await {
                Ok(b) => balance = b,
                Err(e) => {
                    eprintln!("   Error checking balance: {}", e);
                    continue;
                }
            }
            check_count += 1;

            if balance > 0 {
                println!(
                    "\n\n   Funds received! Balance: {} SOL",
                    balance as f64 / LAMPORTS_PER_SOL
                );
                break;
            }

            if check_count.is_multiple_of(12) {
                println!("   Still waiting... (checked {} times)", check_count);
            } else {
                eprint!(".");
            }
        }
    }

    // Step 5: Create and send a self-transfer transaction
    println!("\n5. Creating a self-transfer transaction...");
    let transfer_amount_sol = 0.000001_f64;
    let lamports = (transfer_amount_sol * LAMPORTS_PER_SOL) as u64;
    let priority_fee_micro_lamports = 1000_u64;

    println!(
        "   Amount: {} SOL ({} lamports)",
        transfer_amount_sol, lamports
    );
    println!("   From/To: {}", sol_addr);
    println!(
        "   Priority fee: {} micro-lamports per compute unit",
        priority_fee_micro_lamports
    );

    let fee_payer = match decode_pubkey(&sol_addr) {
        Ok(pk) => pk,
        Err(e) => {
            eprintln!("   Failed to decode public key: {}", e);
            std::process::exit(1);
        }
    };

    // Build instructions: priority fee + self-transfer
    let instructions = vec![
        set_compute_unit_price_instruction(priority_fee_micro_lamports),
        system_transfer_instruction(&fee_payer, &fee_payer, lamports),
    ];

    let blockhash = match get_latest_blockhash(&http, &solana_rpc_url).await {
        Ok(bh) => bh,
        Err(e) => {
            eprintln!("   Failed to get latest blockhash: {}", e);
            std::process::exit(1);
        }
    };
    println!("   Recent blockhash: {}", blockhash);

    let serialized = match serialize_transaction(&instructions, &fee_payer, &blockhash) {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("   Failed to serialize transaction: {}", e);
            std::process::exit(1);
        }
    };
    let encoded = base64url_encode(&serialized);

    println!("   Transaction created ({} bytes)\n", serialized.len());

    // Step 6: Sign and send transaction
    println!("6. Signing and sending transaction...");
    let signed_result = match sdk
        .sign_and_send_transaction(ServerSignAndSendTransactionParams {
            wallet_id: wallet.wallet_id.clone(),
            transaction: encoded,
            network_id: network_id_str.clone(),
            derivation_index: None,
            account: Some(sol_addr.clone()),
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("   Failed to sign and send transaction: {}", e);
            std::process::exit(1);
        }
    };

    println!("   Transaction signed and sent!");
    if let Some(ref hash) = signed_result.hash {
        println!("   Transaction Hash: {}", hash);
    }
    println!(
        "   Raw transaction (base64url): {}",
        signed_result.raw_transaction
    );

    let signature = match &signed_result.hash {
        Some(h) => h.clone(),
        None => {
            eprintln!("   No signature/hash found in signed result");
            std::process::exit(1);
        }
    };

    if let Some(ref explorer) = signed_result.block_explorer {
        println!("   Explorer: {}", explorer);
    }

    // Step 7: Wait for confirmation
    println!("\n7. Waiting for transaction confirmation...");
    let start = std::time::Instant::now();
    let mut confirmed = false;
    let max_attempts = 30u32;

    for attempt in 0..max_attempts {
        match get_signature_status(&http, &solana_rpc_url, &signature).await {
            Ok(Some(status)) => {
                if let Some(err) = status.get("err") {
                    if !err.is_null() {
                        eprintln!("\n   Transaction failed: {}", err);
                        break;
                    }
                }

                let confirmation_status = status
                    .get("confirmationStatus")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                if confirmation_status == "confirmed" || confirmation_status == "finalized" {
                    confirmed = true;
                    let elapsed = start.elapsed().as_secs_f64();
                    println!("\n   Transaction confirmed in {:.1} seconds!", elapsed);
                    println!("   Status: {}", confirmation_status);
                    if let Some(slot) = status.get("slot").and_then(|v| v.as_u64()) {
                        println!("   Slot: {}", slot);
                    }
                    break;
                }
            }
            Ok(None) => {
                // Not yet visible to the cluster
            }
            Err(e) => {
                eprintln!("\n   Error checking transaction status: {}", e);
                break;
            }
        }

        if !confirmed {
            eprint!(".");
            if attempt > 0 && attempt % 10 == 0 {
                eprintln!(" (attempt {}/{})", attempt, max_attempts);
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    if !confirmed {
        println!("\n   Transaction confirmation timeout. Check manually:");
        println!("   {}", explorer_url(&signature, &network));
    } else {
        println!(
            "\n   View on Explorer: {}",
            explorer_url(&signature, &network)
        );
    }

    // Step 8: Final balance check
    println!("\n8. Final balance check...");
    match get_balance(&http, &solana_rpc_url, &sol_addr).await {
        Ok(final_balance) => {
            println!(
                "   Balance: {} SOL",
                final_balance as f64 / LAMPORTS_PER_SOL
            );
            let change = balance as i64 - final_balance as i64;
            println!(
                "   Change: {} SOL (includes base fee + priority fee)",
                change as f64 / LAMPORTS_PER_SOL
            );
        }
        Err(e) => {
            eprintln!("   Failed to check final balance: {}", e);
        }
    }

    // List wallets
    println!("\n9. Listing wallets...");
    match sdk.get_wallets(Some(10), Some(0)).await {
        Ok(result) => {
            println!("   Total wallets: {}", result.total_count);
            for w in &result.wallets {
                println!("   {} ({})", w.wallet_id, w.wallet_name);
            }
        }
        Err(e) => {
            println!("   Failed to list wallets: {}", e);
        }
    }

    println!("\nDemo completed successfully!");
}
