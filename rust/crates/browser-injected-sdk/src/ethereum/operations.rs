//! High-level Ethereum operations that wrap the strategy.
//!
//! These functions mirror the individual TS files: connect.ts, disconnect.ts,
//! getAccounts.ts, signMessage.ts, sendTransaction.ts, chainUtils.ts, etc.

use super::events::EthereumEventListeners;
use super::strategy::EthereumStrategy;
use super::types::*;

/// Connect to the Ethereum wallet.
pub async fn connect(
    strategy: &dyn EthereumStrategy,
    events: &EthereumEventListeners,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    if strategy.is_connected() {
        return strategy.get_accounts().await;
    }

    // First try eager connecting
    if let Ok(Some(accounts)) = strategy.connect(true).await {
        if !accounts.is_empty() {
            events.trigger_event(
                EthereumEventType::Connect,
                serde_json::to_value(&accounts)?,
            );
            return Ok(accounts);
        }
    }

    // Prompt user to connect
    if let Ok(Some(accounts)) = strategy.connect(false).await {
        if !accounts.is_empty() {
            events.trigger_event(
                EthereumEventType::Connect,
                serde_json::to_value(&accounts)?,
            );
            return Ok(accounts);
        }
    }

    Err("Failed to connect to Phantom.".into())
}

/// Disconnect from the Ethereum wallet.
pub async fn disconnect(
    strategy: &dyn EthereumStrategy,
    events: &EthereumEventListeners,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    strategy.disconnect().await?;
    events.trigger_event(
        EthereumEventType::Disconnect,
        serde_json::json!([]),
    );
    Ok(())
}

/// Get connected accounts.
pub async fn get_accounts(
    strategy: &dyn EthereumStrategy,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    strategy.get_accounts().await
}

/// Sign a message (eth_sign).
pub async fn sign_message(
    strategy: &dyn EthereumStrategy,
    message: &str,
    address: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.sign_message(message, address).await
}

/// Sign a personal message (personal_sign).
pub async fn sign_personal_message(
    strategy: &dyn EthereumStrategy,
    message: &str,
    address: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.sign_personal_message(message, address).await
}

/// Sign EIP-712 typed data.
pub async fn sign_typed_data(
    strategy: &dyn EthereumStrategy,
    typed_data: &serde_json::Value,
    address: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.sign_typed_data(typed_data, address).await
}

/// Send a transaction.
pub async fn send_transaction(
    strategy: &dyn EthereumStrategy,
    transaction: &EthereumTransaction,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.send_transaction(transaction).await
}

/// Sign a transaction without sending.
pub async fn sign_transaction(
    strategy: &dyn EthereumStrategy,
    transaction: &EthereumTransaction,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.sign_transaction(transaction).await
}

/// Sign in with Ethereum (SIWE).
pub async fn sign_in(
    strategy: &dyn EthereumStrategy,
    sign_in_data: &EthereumSignInData,
) -> Result<EthereumSignInResult, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy.connect(false).await?;
    }
    strategy.sign_in(sign_in_data).await
}

/// Get the current chain ID.
pub async fn get_chain_id(
    strategy: &dyn EthereumStrategy,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    strategy.get_chain_id().await
}

/// Switch to a different chain.
pub async fn switch_chain(
    strategy: &dyn EthereumStrategy,
    chain_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    strategy.switch_chain(chain_id).await
}
