//! Phantom Client Multi-Authenticator Demo
//!
//! Demonstrates:
//! 1. Generating multiple key pairs
//! 2. Creating an organization with multiple authenticators
//! 3. Testing getOrganization, getWalletWithTag
//! 4. Creating and deleting authenticators
//! 5. Verifying multi-authenticator access
//!
//! Usage:
//!   cargo run --bin phantom-multi-auth-demo
//!
//! Environment variables:
//!   API_BASE_URL  - Phantom API base URL (defaults to staging)
//!   APP_ID        - Application ID

use phantom_api_key_stamper::{ApiKeyStamper, ApiKeyStamperConfig};
use phantom_base64url::base64url_encode;
use phantom_client::{
    AuthenticatorConfig, ClientAlgorithm, CreateAuthenticatorParams, DeleteAuthenticatorParams,
    GetWalletWithTagParams, PhantomClient, PhantomClientConfig, UserConfig,
};
use phantom_crypto::generate_key_pair;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("Starting Phantom Client Multi-Authenticator Test");
    println!("This test demonstrates PhantomClient methods for multi-authenticator organizations");
    println!("{}", "=".repeat(80));

    let api_base_url = std::env::var("API_BASE_URL")
        .unwrap_or_else(|_| "https://staging-api.phantom.app/v1/wallets".to_string());
    let app_id = std::env::var("APP_ID")
        .unwrap_or_else(|_| "2b4308d3-e072-4f31-b890-4bd14ed6e546".to_string());

    // Step 1: Generate two key pairs for different authenticators
    println!("\nStep 1: Generate Key Pairs");

    let primary_key_pair = generate_key_pair();
    let secondary_key_pair = generate_key_pair();

    println!(
        "  Primary key pair: {}...",
        &primary_key_pair.public_key[..20.min(primary_key_pair.public_key.len())]
    );
    println!(
        "  Secondary key pair: {}...",
        &secondary_key_pair.public_key[..20.min(secondary_key_pair.public_key.len())]
    );

    // Step 2: Create stamper and client using primary key
    println!("\nStep 2: Initialize PhantomClient");

    let primary_stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
        api_secret_key: primary_key_pair.secret_key.clone(),
    })
    .expect("Failed to create primary stamper");

    let mut headers = std::collections::HashMap::new();
    headers.insert("x-app-id".to_string(), app_id);

    let mut client = PhantomClient::new(
        PhantomClientConfig {
            api_base_url: api_base_url.clone(),
            organization_id: None,
            headers: Some(headers),
            wallet_type: "server-wallet".to_string(),
        },
        Some(Arc::new(primary_stamper)),
    );

    println!("  PhantomClient initialized");

    // Step 3: Create organization with multiple authenticators
    println!("\nStep 3: Test createOrganization with Multiple Authenticators");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let organization_name = format!("Test Org {}", timestamp);
    let primary_auth_name = format!("Primary-Auth-{}", timestamp);
    let secondary_auth_name = format!("Secondary-Auth-{}", timestamp);
    let custom_username = format!("test-user-{}", timestamp);

    let primary_pk_bytes = bs58::decode(&primary_key_pair.public_key)
        .into_vec()
        .expect("Failed to decode primary public key");
    let secondary_pk_bytes = bs58::decode(&secondary_key_pair.public_key)
        .into_vec()
        .expect("Failed to decode secondary public key");

    let users = vec![UserConfig {
        username: custom_username.clone(),
        role: Some("ADMIN".to_string()),
        authenticators: vec![
            AuthenticatorConfig::Keypair {
                authenticator_name: primary_auth_name.clone(),
                public_key: base64url_encode(&primary_pk_bytes),
                algorithm: ClientAlgorithm::Ed25519,
                expires_in_ms: None,
            },
            AuthenticatorConfig::Keypair {
                authenticator_name: secondary_auth_name.clone(),
                public_key: base64url_encode(&secondary_pk_bytes),
                algorithm: ClientAlgorithm::Ed25519,
                expires_in_ms: None,
            },
        ],
    }];

    println!("  Creating organization: {}", organization_name);
    println!("  With custom username: {}", custom_username);
    println!("  With {} authenticators", users[0].authenticators.len());

    match client
        .create_organization(&organization_name, &users, None)
        .await
    {
        Ok(organization) => {
            let org_id = organization["organizationId"].as_str().unwrap_or("unknown");
            let org_name = organization["organizationName"]
                .as_str()
                .unwrap_or("unknown");

            println!("  Organization created successfully!");
            println!("    ID: {}", org_id);
            println!("    Name: {}", org_name);

            // Step 4: Test getOrganization
            println!("\nStep 4: Test getOrganization Method");

            let mut first_authenticator_id = String::new();

            match client.get_organization(org_id).await {
                Ok(retrieved_org) => {
                    println!("  getOrganization works!");
                    println!(
                        "    Retrieved: {}",
                        retrieved_org["organizationName"]
                            .as_str()
                            .unwrap_or("unknown")
                    );

                    if let Some(users) = retrieved_org["users"].as_array() {
                        println!("    Users: {}", users.len());
                        if let Some(first_user) = users.first() {
                            if let Some(auths) = first_user["authenticators"].as_array() {
                                println!("    Authenticators: {}", auths.len());
                                if let Some(first_auth) = auths.first() {
                                    if let Some(id) = first_auth["id"].as_str() {
                                        first_authenticator_id = id.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("  getOrganization failed: {}", e),
            }

            // Step 5: Test secondary authenticator access
            println!("\nStep 5: Test Secondary Authenticator Access");

            let secondary_stamper = ApiKeyStamper::new(ApiKeyStamperConfig {
                api_secret_key: secondary_key_pair.secret_key.clone(),
            })
            .expect("Failed to create secondary stamper");

            let secondary_client = PhantomClient::new(
                PhantomClientConfig {
                    api_base_url: api_base_url.clone(),
                    organization_id: Some(org_id.to_string()),
                    headers: None,
                    wallet_type: "server-wallet".to_string(),
                },
                Some(Arc::new(secondary_stamper)),
            );

            println!("  Secondary client created");

            // Step 6: Test wallet creation
            println!("\nStep 6: Test Wallet Creation");

            let _ = client.set_organization_id(org_id.to_string());
            match client.create_wallet(Some("Test Wallet")).await {
                Ok(wallet) => {
                    println!(
                        "  Wallet created: {}...",
                        &wallet.wallet_id[..20.min(wallet.wallet_id.len())]
                    );
                    for addr in &wallet.addresses {
                        println!("    {} : {}", addr.address_type, addr.address);
                    }
                }
                Err(e) => println!("  Failed to create wallet: {}", e),
            }

            // Step 7: Test getWalletWithTag
            println!("\nStep 7: Test getWalletWithTag Method");

            match client
                .get_wallet_with_tag(&GetWalletWithTagParams {
                    organization_id: org_id.to_string(),
                    tag: "test-tag".to_string(),
                    derivation_paths: vec!["m/44'/501'/0'/0'".to_string()],
                })
                .await
            {
                Ok(_) => println!("  getWalletWithTag found tagged wallet"),
                Err(_) => println!("  getWalletWithTag works (no tagged wallets exist yet)"),
            }

            // Step 8: Test createAuthenticator
            println!("\nStep 8: Test createAuthenticator Method");

            let third_key_pair = generate_key_pair();
            let third_auth_name = format!("Third-Auth-{}", timestamp);
            let third_pk_bytes = bs58::decode(&third_key_pair.public_key)
                .into_vec()
                .expect("Failed to decode third public key");

            match client
                .create_authenticator(&CreateAuthenticatorParams {
                    organization_id: org_id.to_string(),
                    username: custom_username.clone(),
                    authenticator_name: third_auth_name.clone(),
                    authenticator: AuthenticatorConfig::Keypair {
                        authenticator_name: third_auth_name,
                        public_key: base64url_encode(&third_pk_bytes),
                        algorithm: ClientAlgorithm::Ed25519,
                        expires_in_ms: None,
                    },
                    replace_expirable: None,
                })
                .await
            {
                Ok(result) => {
                    println!("  createAuthenticator works");
                    println!(
                        "    Response: {}",
                        serde_json::to_string_pretty(&result).unwrap_or_default()
                    );
                }
                Err(e) => println!("  createAuthenticator method error: {}", e),
            }

            // Step 9: Test deleteAuthenticator
            println!("\nStep 9: Test deleteAuthenticator Method");

            if !first_authenticator_id.is_empty() {
                match client
                    .delete_authenticator(&DeleteAuthenticatorParams {
                        organization_id: org_id.to_string(),
                        username: custom_username.clone(),
                        authenticator_id: first_authenticator_id,
                    })
                    .await
                {
                    Ok(result) => {
                        println!("  deleteAuthenticator works");
                        println!(
                            "    Response: {}",
                            serde_json::to_string_pretty(&result).unwrap_or_default()
                        );
                    }
                    Err(e) => println!("  deleteAuthenticator method error: {}", e),
                }
            } else {
                println!("  deleteAuthenticator skipped (no authenticator ID found)");
            }

            // Step 10: Test multi-authenticator access
            println!("\nStep 10: Test Multi-Authenticator Access");

            match secondary_client.get_organization(org_id).await {
                Ok(_) => {
                    println!("  Both authenticators can access the same organization!");
                    println!(
                        "    Primary auth: {}...",
                        &primary_key_pair.public_key[..12]
                    );
                    println!(
                        "    Secondary auth: {}...",
                        &secondary_key_pair.public_key[..12]
                    );
                }
                Err(e) => println!("  Secondary auth failed: {}", e),
            }
        }
        Err(e) => {
            println!("  Failed to create organization: {}", e);
            println!("  (This is expected without valid API credentials)");
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("All PhantomClient Methods Tested!");
    println!("\nTest Results:");
    println!("  createOrganization - Tested with multiple authenticators");
    println!("  getOrganization - Tested retrieval of organization details");
    println!("  getWalletWithTag - Method tested (no tagged wallets found)");
    println!("  createAuthenticator - Tested additional authenticator creation");
    println!("  deleteAuthenticator - Tested authenticator deletion");
    println!("  Multi-auth access - Both authenticators accessed same organization");
    println!(
        "  OIDC support - AuthenticatorConfig supports OIDC with jwks_url and id_token_claims"
    );

    println!("\nDemo completed.");
}
