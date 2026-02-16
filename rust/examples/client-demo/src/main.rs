//! Phantom Client Demo
//!
//! Demonstrates:
//! 1. Generating an Ed25519 keypair
//! 2. Creating an API key stamper
//! 3. Initializing a PhantomClient
//! 4. Creating an organization
//! 5. Creating a wallet
//! 6. Persisting demo data to demo-data.json
//! 7. Generating SERVER_SDK_USAGE.md documentation
//!
//! Usage:
//!   cargo run --bin phantom-client-demo
//!
//! Environment variables:
//!   API_BASE_URL  - Phantom API base URL (defaults to staging)
//!   APP_ID        - Application ID

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_base64url::base64url_encode;
use phantom_client::{
    AuthenticatorConfig, ClientAlgorithm, PhantomClient, PhantomClientConfig, UserConfig,
};
use phantom_crypto::generate_key_pair;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Persisted demo data, mirroring the TypeScript DemoData interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoData {
    #[serde(skip_serializing_if = "Option::is_none")]
    key_pair: Option<KeyPairData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<OrganizationData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyPairData {
    public_key: String,
    secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrganizationData {
    organization_id: String,
    name: String,
}

/// Return the path to demo-data.json in the current working directory.
fn demo_data_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("demo-data.json")
}

/// Try to load existing demo data from disk.
fn load_demo_data(path: &PathBuf) -> DemoData {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<DemoData>(&contents) {
            Ok(data) => {
                println!("Loaded existing demo data");
                data
            }
            Err(_) => {
                println!("Creating new demo data file");
                DemoData::default()
            }
        },
        Err(_) => {
            println!("Creating new demo data file");
            DemoData::default()
        }
    }
}

/// Save demo data to disk.
fn save_demo_data(path: &PathBuf, data: &DemoData) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(data).expect("Failed to serialize demo data");
    std::fs::write(path, json)
}

#[tokio::main]
async fn main() {
    println!("Starting Phantom Client Demo\n");

    let data_path = demo_data_path();
    let mut demo_data = load_demo_data(&data_path);

    let api_base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "https://staging-api.phantom.app/v1/wallets".to_string());
    let app_id = std::env::var("APP_ID")
        .unwrap_or_else(|_| "2b4308d3-e072-4f31-b890-4bd14ed6e546".to_string());

    // Step 1: Generate key pair and save to JSON
    println!("Generating key pair...");
    let key_pair = generate_key_pair();

    demo_data.key_pair = Some(KeyPairData {
        public_key: key_pair.public_key.clone(),
        secret_key: key_pair.secret_key.clone(),
    });

    save_demo_data(&data_path, &demo_data).expect("Failed to save demo data");
    println!("  Key pair generated and saved:");
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
            println!(
                "  {}",
                serde_json::to_string_pretty(&org).unwrap_or_default()
            );

            // Persist organization data
            let org_id = org
                .get("organizationId")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let org_name = org
                .get("organizationName")
                .and_then(|v| v.as_str())
                .unwrap_or("Demo Organization")
                .to_string();

            demo_data.organization = Some(OrganizationData {
                organization_id: org_id.clone(),
                name: org_name,
            });

            save_demo_data(&data_path, &demo_data).expect("Failed to save demo data");

            // Step 5: Create wallet
            if !org_id.is_empty() {
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
    println!("All data saved to: {}", data_path.display());

    // Generate server SDK usage documentation
    generate_server_sdk_doc(&demo_data);
}

/// Generate a comprehensive SERVER_SDK_USAGE.md file with code examples.
///
/// Mirrors the TypeScript `generateServerSDKDoc()` function, providing
/// Rust-oriented usage examples alongside the original TypeScript examples.
fn generate_server_sdk_doc(demo_data: &DemoData) {
    println!("\nGenerating Server SDK documentation...");

    let org_id = demo_data
        .organization
        .as_ref()
        .map(|o| o.organization_id.as_str())
        .unwrap_or("<ORGANIZATION_ID>");
    let secret_key = demo_data
        .key_pair
        .as_ref()
        .map(|k| k.secret_key.as_str())
        .unwrap_or("<PRIVATE_KEY>");
    let public_key = demo_data
        .key_pair
        .as_ref()
        .map(|k| k.public_key.as_str())
        .unwrap_or("<PUBLIC_KEY>");

    let demo_json = serde_json::to_string_pretty(demo_data).unwrap_or_default();

    let doc_content = format!(
        r#"# Using Phantom Server SDK with Your Credentials

This guide shows how to use the Phantom Server SDK with your generated credentials.

## Your Credentials

You have been provided with the following credentials:
- **Organization ID**: `{org_id}`
- **Private Key**: `{secret_key}`
- **Public Key**: `{public_key}`

## Setup

1. Install the Server SDK:
```bash
npm install @phantom/server-sdk
# or
yarn add @phantom/server-sdk
```

2. Create a `.env` file with your credentials:
```env
ORGANIZATION_ID={org_id}
PRIVATE_KEY={secret_key}
API_URL=https://staging-api.phantom.app/v1/wallets
```

## Basic Usage

```typescript
import {{ ServerSDK, NetworkId }} from "@phantom/server-sdk";
import dotenv from "dotenv";

// Load environment variables
dotenv.config();

// Initialize the SDK
const sdk = new ServerSDK({{
  organizationId: process.env.ORGANIZATION_ID!,
  appId: process.env.APP_ID!,
  apiPrivateKey: process.env.PRIVATE_KEY!,
  apiBaseUrl: process.env.API_URL!,
}});

async function main() {{
  try {{
    // Create a wallet
    const wallet = await sdk.createWallet("My Wallet");
    console.log("Wallet ID:", wallet.walletId);
    console.log("Addresses:", wallet.addresses);

    // Sign a message (ServerSDK accepts plain text)
    const messageResult = await sdk.signMessage({{
      walletId: wallet.walletId,
      message: "Hello from Phantom!",
      networkId: NetworkId.SOLANA_MAINNET,
    }});
    console.log("Message signature result:", messageResult);
    console.log("Human-readable signature:", messageResult.signature);
    console.log("Raw signature:", messageResult.rawSignature);

    // Example: Sign a transaction (you'll need actual transaction data)
    // const signedTx = await sdk.signAndSendTransaction({{
    //   walletId: wallet.walletId,
    //   transaction: yourTransactionObject, // ServerSDK accepts various formats
    //   networkId: NetworkId.SOLANA_MAINNET,
    // }});

    // List all wallets in your organization
    const walletsResult = await sdk.getWallets(10, 0);
    console.log(`Total wallets: ${{walletsResult.totalCount}}`);

  }} catch (error) {{
    console.error("Error:", error);
  }}
}}

main();
```

## Advanced Features

### Working with Multiple Networks

```typescript
// When you create a wallet, it automatically generates addresses for multiple chains
const wallet = await sdk.createWallet("My Multi-chain Wallet");

// Access different chain addresses from the wallet.addresses array
const solanaAddress = wallet.addresses.find(addr => addr.addressType === "Solana")?.address;
const ethereumAddress = wallet.addresses.find(addr => addr.addressType === "Ethereum")?.address;
const bitcoinAddress = wallet.addresses.find(addr => addr.addressType === "BitcoinSegwit")?.address;

console.log("Solana:", solanaAddress);
console.log("Ethereum:", ethereumAddress);
console.log("Bitcoin:", bitcoinAddress);

// Sign messages on different networks (ServerSDK accepts plain text)
const solanaResult = await sdk.signMessage({{
  walletId: wallet.walletId,
  message: "Solana message",
  networkId: NetworkId.SOLANA_MAINNET,
}});

const ethereumResult = await sdk.signMessage({{
  walletId: wallet.walletId,
  message: "Ethereum message",
  networkId: NetworkId.ETHEREUM_MAINNET,
}});
```

### Transaction Signing Examples

The ServerSDK accepts various transaction formats and automatically parses them:

```typescript
import {{ Transaction, SystemProgram, PublicKey }} from "@solana/web3.js";

// Solana transaction example - ServerSDK accepts the transaction object directly
const transaction = new Transaction().add(
  SystemProgram.transfer({{
    fromPubkey: new PublicKey(solanaAddress),
    toPubkey: new PublicKey("RECIPIENT_ADDRESS_HERE"),
    lamports: 1000000, // 0.001 SOL
  }})
);

// Set recent blockhash and fee payer (required for Solana transactions)
// transaction.recentBlockhash = recentBlockhash;
// transaction.feePayer = new PublicKey(solanaAddress);

// ServerSDK accepts the transaction object directly - no need to serialize
const signedTransaction = await sdk.signAndSendTransaction({{
  walletId: wallet.walletId,
  transaction, // Pass the Transaction object directly
  networkId: NetworkId.SOLANA_MAINNET,
}});
```

## Security Notes

**IMPORTANT**:
- Store your private key securely in environment variables
- Never commit the private key to version control
- Use the staging API URL (`https://staging-api.phantom.app/v1/wallets`) for testing
- Switch to production API URL for live applications

## Next Steps

1. Read the full [Server SDK documentation](https://docs.phantom.com/server-sdk)
2. Check out more [examples](https://github.com/phantom/wallet-sdk/tree/main/examples/server-sdk-examples)
3. Integrate with your application's backend services

## Your Credential Data

Your credentials and organization information:
```json
{demo_json}
```
"#
    );

    let doc_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("SERVER_SDK_USAGE.md");

    match std::fs::write(&doc_path, doc_content) {
        Ok(()) => println!("Server SDK documentation generated: {}", doc_path.display()),
        Err(e) => eprintln!("Failed to write documentation: {}", e),
    }
}
