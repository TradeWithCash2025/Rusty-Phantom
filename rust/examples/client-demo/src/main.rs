//! Phantom Client Demo
//!
//! Demonstrates:
//! 1. Generating an Ed25519 keypair
//! 2. Creating an API key stamper
//! 3. Initializing a PhantomClient
//! 4. Creating an organization
//! 5. Creating a wallet
//!
//! Usage:
//!   cargo run --bin phantom-client-demo
//!
//! Environment variables:
//!   API_BASE_URL  - Phantom API base URL (defaults to staging)
//!   APP_ID        - Application ID

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_base64url::base64url_encode;
use phantom_client::{AuthenticatorConfig, ClientAlgorithm, PhantomClient, PhantomClientConfig, UserConfig};
use phantom_crypto::generate_key_pair;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Starting Phantom Client Demo\n");

    let api_base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "https://staging-api.phantom.app/v1/wallets".to_string());
    let app_id = std::env::var("APP_ID")
        .unwrap_or_else(|_| "2b4308d3-e072-4f31-b890-4bd14ed6e546".to_string());

    // Step 1: Generate key pair
    println!("Generating key pair...");
    let key_pair = generate_key_pair();
    println!("  Public Key: {}", key_pair.public_key);
    println!(
        "  Secret Key: {}...",
        &key_pair.secret_key[..20.min(key_pair.secret_key.len())]
    );

    // Step 2: Create stamper from the generated secret key
    println!("\nInitializing API key stamper...");
    let stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
        api_secret_key: key_pair.secret_key.clone(),
    })
    .expect("Failed to create stamper from generated key");
    println!("  Stamper created successfully");

    // Step 3: Initialize the Phantom client
    println!("\nInitializing Phantom Client...");
    let mut headers = std::collections::HashMap::new();
    headers.insert("x-app-id".to_string(), app_id);

    let client = PhantomClient::new(
        PhantomClientConfig {
            api_base_url: api_base_url.clone(),
            organization_id: None,
            headers: Some(headers),
            wallet_type: "server-wallet".to_string(),
        },
        Some(Arc::new(stamper)),
    );
    println!("  Client initialized (base URL: {})", api_base_url);

    // Step 4: Create organization
    println!("\nCreating organization...");
    let pk_bytes = bs58::decode(&key_pair.public_key)
        .into_vec()
        .expect("Failed to decode public key");
    let base64url_public_key = base64url_encode(&pk_bytes);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let users = vec![UserConfig {
        username: format!("demo-user-{}", timestamp),
        role: Some("ADMIN".to_string()),
        authenticators: vec![AuthenticatorConfig::Keypair {
            authenticator_name: format!("demo-auth-{}", timestamp),
            public_key: base64url_public_key,
            algorithm: ClientAlgorithm::Ed25519,
            expires_in_ms: None,
        }],
    }];

    match client
        .create_organization("Demo Organization", &users, None)
        .await
    {
        Ok(org) => {
            println!("  Organization created:");
            println!("  {}", serde_json::to_string_pretty(&org).unwrap_or_default());

            // Step 5: Create wallet
            if let Some(org_id) = org.get("organizationId").and_then(|v| v.as_str()) {
                println!("\nCreating wallet (org: {})...", org_id);
                let client2 = PhantomClient::new(
                    PhantomClientConfig {
                        api_base_url,
                        organization_id: Some(org_id.to_string()),
                        headers: None,
                        wallet_type: "server-wallet".to_string(),
                    },
                    Some(Arc::new(
                        ApiKeyStamper::new(ApiKeyStamperConfig {
                            api_secret_key: key_pair.secret_key.clone(),
                        })
                        .unwrap(),
                    )),
                );

                match client2.create_wallet(Some("Demo Wallet")).await {
                    Ok(wallet) => {
                        println!("  Wallet created:");
                        println!("    Wallet ID: {}", wallet.wallet_id);
                        for addr in &wallet.addresses {
                            println!("    {} : {}", addr.address_type, addr.address);
                        }
                    }
                    Err(e) => println!("  Failed to create wallet: {}", e),
                }
            }
        }
        Err(e) => {
            println!("  Failed to create organization: {}", e);
            println!("  (This is expected without valid API credentials)");
        }
    }

    println!("\nDemo completed.");
}
