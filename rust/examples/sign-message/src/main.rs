//! Phantom Sign Message Demo
//!
//! Signs a message using a Phantom wallet, with optional wallet selection.
//!
//! Usage:
//!   cargo run --bin phantom-sign-message -- "Your message to sign"
//!   cargo run --bin phantom-sign-message -- "Your message" --wallet-id wallet_123
//!   cargo run --bin phantom-sign-message -- "Your message" --wallet-name "My Wallet"
//!
//! Required environment variables:
//!   ORGANIZATION_ID          - Your organization ID
//!   ORGANIZATION_PRIVATE_KEY - Base58-encoded Ed25519 private key
//!   APP_ID                   - Application ID
//!
//! Optional environment variables:
//!   WALLET_API               - API base URL (defaults to staging)

use phantom_base64url::{base64url_decode, string_to_base64url};
use phantom_constants::NetworkId;
use phantom_server_sdk::{ServerSdk, ServerSdkConfig, ServerSignMessageParams};

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
        std::process::exit(1);
    })
}

/// Encode bytes as a hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse command line arguments.
struct CliArgs {
    message: String,
    wallet_id: Option<String>,
    wallet_name: Option<String>,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        eprintln!("No message provided");
        eprintln!("\nUsage:");
        eprintln!("  cargo run --bin phantom-sign-message -- \"Your message to sign\"");
        eprintln!(
            "  cargo run --bin phantom-sign-message -- \"Your message\" --wallet-id wallet_123"
        );
        eprintln!(
            "  cargo run --bin phantom-sign-message -- \"Your message\" --wallet-name \"My Wallet\""
        );
        eprintln!("\nOptions:");
        eprintln!("  --wallet-id     Use an existing wallet by ID");
        eprintln!("  --wallet-name   Create/find a wallet with this name");
        eprintln!("\nIf no wallet is specified, a new wallet will be created.");
        std::process::exit(1);
    }

    let message = args[0].clone();
    let mut wallet_id = None;
    let mut wallet_name = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--wallet-id" && i + 1 < args.len() {
            wallet_id = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--wallet-name" && i + 1 < args.len() {
            wallet_name = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    CliArgs {
        message,
        wallet_id,
        wallet_name,
    }
}

/// Get or create a wallet, returning (wallet_id, solana_address, is_new).
async fn get_or_create_wallet(
    sdk: &ServerSdk,
    wallet_id: Option<&str>,
    wallet_name: Option<&str>,
) -> Result<(String, String, bool), Box<dyn std::error::Error>> {
    // If wallet ID is provided, verify it exists and get Solana address
    if let Some(id) = wallet_id {
        println!("Using wallet ID: {}", id);
        match sdk.get_wallet_addresses(id, None, None).await {
            Ok(addresses) => {
                let solana_address = addresses
                    .iter()
                    .find(|a| a.address_type.to_lowercase() == "solana")
                    .map(|a| a.address.clone())
                    .ok_or("Wallet does not have a Solana address")?;
                return Ok((id.to_string(), solana_address, false));
            }
            Err(e) => {
                eprintln!("Failed to get wallet addresses: {}", e);
                std::process::exit(1);
            }
        }
    }

    // If wallet name is provided, try to find existing wallet
    if let Some(name) = wallet_name {
        println!("Looking for wallet with name: {}", name);
        if let Ok(result) = sdk.get_wallets(Some(100), Some(0)).await {
            if let Some(existing) = result.wallets.iter().find(|w| w.wallet_name == name) {
                println!("Found existing wallet: {}", existing.wallet_id);
                let addresses = sdk
                    .get_wallet_addresses(&existing.wallet_id, None, None)
                    .await?;
                let solana_address = addresses
                    .iter()
                    .find(|a| a.address_type.to_lowercase() == "solana")
                    .map(|a| a.address.clone())
                    .ok_or("Wallet does not have a Solana address")?;
                return Ok((existing.wallet_id.clone(), solana_address, false));
            }
        }
    }

    // Create new wallet
    let name = wallet_name.unwrap_or("Message Signing Wallet");
    let wallet_name = if wallet_name.is_some() {
        name.to_string()
    } else {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{} {}", name, ts)
    };

    println!("Creating new wallet: {}", wallet_name);

    match sdk.create_wallet(&wallet_name).await {
        Ok(wallet) => {
            let solana_address = wallet
                .addresses
                .iter()
                .find(|a| a.address_type.to_lowercase() == "solana")
                .map(|a| a.address.clone())
                .ok_or("Created wallet does not have a Solana address")?;
            println!("Wallet created: {}", wallet.wallet_id);
            Ok((wallet.wallet_id, solana_address, true))
        }
        Err(e) => {
            eprintln!("Failed to create wallet: {}", e);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Phantom Message Signer\n");

    // Parse arguments
    let cli_args = parse_args();

    // Load config
    let organization_id = env_required("ORGANIZATION_ID");
    let api_private_key = env_required("ORGANIZATION_PRIVATE_KEY");
    let app_id = env_required("APP_ID");
    let api_base_url = env_or("WALLET_API", "https://staging-api.phantom.app/v1/wallets");

    // Initialize SDK
    println!("Initializing Server SDK...");
    let sdk = ServerSdk::new(ServerSdkConfig {
        organization_id,
        app_id,
        api_base_url: Some(api_base_url),
        api_private_key,
        solana_rpc_url: None,
    });

    // Get or create wallet
    let (wallet_id, solana_address, is_new) = match get_or_create_wallet(
        &sdk,
        cli_args.wallet_id.as_deref(),
        cli_args.wallet_name.as_deref(),
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to get/create wallet: {}", e);
            std::process::exit(1);
        }
    };

    println!("\nMessage Details:");
    println!("   Message: \"{}\"", cli_args.message);
    println!("   Length: {} characters", cli_args.message.len());
    println!("   UTF-8 bytes: {}", cli_args.message.len());
    println!(
        "   Base64url encoded: {}",
        string_to_base64url(&cli_args.message)
    );
    println!("\nWallet Details:");
    println!("   Wallet ID: {}", wallet_id);
    println!("   Solana Address: {}", solana_address);
    println!(
        "   Status: {}",
        if is_new {
            "Newly created"
        } else {
            "Existing wallet"
        }
    );

    // Resolve network ID string for Solana mainnet
    let network_id_str = serde_json::to_value(NetworkId::SolanaMainnet)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp".to_string());

    // Sign the message
    println!("\nSigning message with Solana network...");

    match sdk
        .sign_message(ServerSignMessageParams {
            wallet_id: wallet_id.clone(),
            message: cli_args.message.clone(),
            network_id: network_id_str,
            derivation_index: None,
        })
        .await
    {
        Ok(result) => {
            println!("\nMessage signed successfully!");
            println!("\nSignature Details:");
            println!("   Human-readable: {}", result.signature);
            println!("   Raw (base64url): {}", result.raw_signature);
            println!("   Length: {} characters", result.raw_signature.len());
            if let Some(ref explorer) = result.block_explorer {
                println!("   Block Explorer: {}", explorer);
            }

            // Convert raw signature to other formats
            if let Ok(signature_bytes) = base64url_decode(&result.raw_signature) {
                let signature_hex = hex_encode(&signature_bytes);
                let first_8: Vec<String> = signature_bytes
                    .iter()
                    .take(8)
                    .map(|b| b.to_string())
                    .collect();

                println!("   Hex: {}", signature_hex);
                println!(
                    "   Bytes: [{}...] ({} bytes)",
                    first_8.join(", "),
                    signature_bytes.len()
                );
            }

            // Verification info
            println!("\nVerification Info:");
            println!("   To verify this signature:");
            println!("   - Public Key: {}", solana_address);
            println!("   - Message: \"{}\"", cli_args.message);
            println!("   - Signature (base64): {}", result.raw_signature);

            println!("\nTips:");
            println!("   - This signature was created using the Solana network context");
            println!("   - You can modify the script to use other networks (Ethereum, etc.)");
            println!(
                "   - The signature can be verified using the public key and original message"
            );
            println!("   - Save the wallet ID to reuse the same wallet for future signatures");

            if is_new {
                println!(
                    "\n  Remember to save this wallet ID for future use: {}",
                    wallet_id
                );
            }
        }
        Err(e) => {
            println!("\nFailed to sign message: {}", e);
            println!("(This is expected without valid API credentials)");
        }
    }

    println!("\nDemo completed.");
}
