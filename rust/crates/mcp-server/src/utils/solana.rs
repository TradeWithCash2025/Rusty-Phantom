//! Solana utility functions.

use crate::tools::types::ToolContext;

/// Retrieves the Solana address for a given wallet.
///
/// # Errors
/// Returns an error if no Solana address is found for the wallet.
pub async fn get_solana_address(
    context: &ToolContext,
    wallet_id: &str,
    derivation_index: Option<u32>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let addresses = context
        .client
        .get_wallet_addresses(wallet_id, None, derivation_index)
        .await?;

    let solana_address = addresses
        .iter()
        .find(|addr| addr.address_type == "solana" || addr.address_type.to_lowercase() == "solana");

    match solana_address {
        Some(addr) => Ok(addr.address.clone()),
        None => Err("No Solana address found for this wallet".into()),
    }
}
