//! Phantom List Wallets Demo
//!
//! Lists all wallets in an organization with pagination and summary statistics.
//!
//! Usage:
//!   cargo run --bin phantom-list-wallets
//!
//! Required environment variables:
//!   ORGANIZATION_ID          - Your organization ID
//!   ORGANIZATION_PRIVATE_KEY - Base58-encoded Ed25519 private key
//!   APP_ID                   - Application ID
//!
//! Optional environment variables:
//!   WALLET_API               - API base URL (defaults to staging)

use phantom_server_sdk::{ServerSdk, ServerSdkConfig};
use std::collections::HashMap;

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

#[tokio::main]
async fn main() {
    println!("Phantom Wallet Lister\n");

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
    });

    println!("Fetching wallets...\n");

    let page_size: u64 = 50;
    let mut offset: u64 = 0;
    let mut total_wallets: u64 = 0;
    let mut all_wallets = Vec::new();
    let mut first_page = true;

    loop {
        match sdk.get_wallets(Some(page_size), Some(offset)).await {
            Ok(result) => {
                if first_page {
                    total_wallets = result.total_count;
                    println!("Total wallets in organization: {}\n", total_wallets);

                    if total_wallets == 0 {
                        println!("No wallets found in this organization.");
                        return;
                    }
                    first_page = false;
                }

                let fetched_count = result.wallets.len() as u64;
                all_wallets.extend(result.wallets);
                offset += fetched_count;

                if offset >= total_wallets || fetched_count == 0 {
                    println!("Fetched all {} wallets!\n", total_wallets);
                    break;
                }

                print!("Fetched {} of {} wallets...\r", offset, total_wallets);
            }
            Err(e) => {
                println!("\nFailed to list wallets: {}", e);
                println!("(This is expected without valid API credentials)");
                return;
            }
        }
    }

    // Display wallet information
    println!("Wallet Details:\n");
    println!("{}", "-".repeat(100));

    for (index, wallet) in all_wallets.iter().enumerate() {
        println!("\nWallet #{}", index + 1);
        println!("   ID: {}", wallet.wallet_id);
        println!("   Name: {}", wallet.wallet_name);
        println!("{}", "-".repeat(100));
    }

    // Summary statistics
    let address_types: HashMap<String, u64> = HashMap::new();
    // Note: Wallet type in the list API returns wallet_id and wallet_name only.
    // To get addresses, use get_wallet_addresses for each wallet.

    println!("\nSummary:");
    println!("   Total wallets: {}", total_wallets);

    if !address_types.is_empty() {
        println!("   Address types:");
        for (addr_type, count) in &address_types {
            println!("     {} : {} addresses", addr_type, count);
        }
    }

    println!("\nTip: You can modify this script to filter wallets by:");
    println!("   - Creation date");
    println!("   - Wallet name pattern");
    println!("   - Address type");
    println!("   - Or export to CSV/JSON for further analysis\n");

    println!("Demo completed.");
}
