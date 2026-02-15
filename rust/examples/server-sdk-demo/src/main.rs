//! Phantom Server SDK Demo
//!
//! Demonstrates:
//! 1. Initializing the Server SDK
//! 2. Creating a wallet
//! 3. Listing wallets
//! 4. Signing a message
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
        eprintln!("\nOptional:");
        eprintln!("  WALLET_API               - API base URL");
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() {
    println!("Phantom Server SDK Demo\n");

    // Load configuration from environment
    let organization_id = env_required("ORGANIZATION_ID");
    let api_private_key = env_required("ORGANIZATION_PRIVATE_KEY");
    let app_id = env_required("APP_ID");
    let api_base_url = env_or("WALLET_API", "https://staging-api.phantom.app/v1/wallets");

    // Step 1: Initialize SDK
    println!("Initializing Server SDK...");
    let sdk = ServerSdk::new(ServerSdkConfig {
        organization_id: organization_id.clone(),
        app_id,
        api_base_url: Some(api_base_url.clone()),
        api_private_key,
        solana_rpc_url: None,
    });
    println!("  Connected to: {}", api_base_url);
    println!("  Organization: {}\n", organization_id);

    // Step 2: Create a wallet
    println!("Creating a new wallet...");
    let wallet_name = format!(
        "Demo Wallet {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    match sdk.create_wallet(&wallet_name).await {
        Ok(wallet) => {
            println!("  Wallet created:");
            println!("    ID: {}", wallet.wallet_id);
            println!("    Name: {}", wallet_name);
            for addr in &wallet.addresses {
                println!("    {} : {}", addr.address_type, addr.address);
            }

            // Find Solana address
            let solana_address = wallet
                .addresses
                .iter()
                .find(|a| a.address_type == "Solana")
                .map(|a| a.address.clone());

            if let Some(sol_addr) = &solana_address {
                println!("\n  Solana address: {}", sol_addr);
            }

            // Step 3: Sign a message
            println!("\nSigning a message...");
            let network_id_str = serde_json::to_value(NetworkId::SolanaMainnet)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp".to_string());

            match sdk
                .sign_message(ServerSignMessageParams {
                    wallet_id: wallet.wallet_id.clone(),
                    message: "Hello from Phantom Server SDK (Rust)!".to_string(),
                    network_id: network_id_str,
                    derivation_index: None,
                })
                .await
            {
                Ok(result) => {
                    println!("  Message signed successfully!");
                    println!("    Signature: {}", result.signature);
                    println!("    Raw (base64url): {}", result.raw_signature);
                }
                Err(e) => println!("  Failed to sign message: {}", e),
            }
        }
        Err(e) => {
            println!("  Failed to create wallet: {}", e);
            println!("  (This is expected without valid API credentials)");
        }
    }

    // Step 4: List wallets
    println!("\nListing wallets...");
    match sdk.get_wallets(Some(10), Some(0)).await {
        Ok(result) => {
            println!("  Total wallets: {}", result.total_count);
            for w in &result.wallets {
                println!("    {} ({})", w.wallet_id, w.wallet_name);
            }
        }
        Err(e) => {
            println!("  Failed to list wallets: {}", e);
        }
    }

    println!("\nDemo completed.");
}
