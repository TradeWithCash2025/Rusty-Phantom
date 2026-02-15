//! High-level Solana operations that wrap the strategy.
//!
//! These functions mirror the individual TS files: connect.ts, disconnect.ts,
//! getAccount.ts, signMessage.ts, signTransaction.ts, etc.

use super::events::SolanaEventListeners;
use super::strategy::SolanaStrategy;
use super::types::*;
use std::sync::Arc;

/// Connect to the Solana wallet.
///
/// Attempts eager connection first, then prompts user if `only_if_trusted` is false.
pub async fn connect(
    strategy: &dyn SolanaStrategy,
    events: &SolanaEventListeners,
    only_if_trusted: bool,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if strategy.is_connected() {
        if let Some(account) = strategy.get_account().await {
            return Ok(account);
        }
    }

    // First try eager connecting without prompting user
    if let Ok(Some(address)) = strategy
        .connect(ConnectOptions {
            only_if_trusted: true,
        })
        .await
    {
        events.trigger_connect(&address);
        return Ok(address);
    }

    if only_if_trusted {
        return Err("No trusted connection available.".into());
    }

    // Prompt user to connect
    if let Ok(Some(address)) = strategy
        .connect(ConnectOptions {
            only_if_trusted: false,
        })
        .await
    {
        events.trigger_connect(&address);
        return Ok(address);
    }

    Err("Failed to connect to Phantom.".into())
}

/// Disconnect from the Solana wallet.
pub async fn disconnect(
    strategy: &dyn SolanaStrategy,
    events: &SolanaEventListeners,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    strategy.disconnect().await?;
    events.trigger_disconnect();
    Ok(())
}

/// Get the currently connected account address.
pub async fn get_account(strategy: &dyn SolanaStrategy) -> Option<String> {
    strategy.get_account().await
}

/// Sign a message.
pub async fn sign_message(
    strategy: &dyn SolanaStrategy,
    message: &[u8],
    display: Option<DisplayEncoding>,
) -> Result<SignMessageResult, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy
            .connect(ConnectOptions {
                only_if_trusted: false,
            })
            .await?;
    }
    strategy.sign_message(message, display).await
}

/// Sign a transaction without sending.
pub async fn sign_transaction(
    strategy: &dyn SolanaStrategy,
    transaction: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy
            .connect(ConnectOptions {
                only_if_trusted: false,
            })
            .await?;
    }
    strategy.sign_transaction(transaction).await
}

/// Sign all transactions without sending.
pub async fn sign_all_transactions(
    strategy: &dyn SolanaStrategy,
    transactions: &[Vec<u8>],
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy
            .connect(ConnectOptions {
                only_if_trusted: false,
            })
            .await?;
    }
    strategy.sign_all_transactions(transactions).await
}

/// Sign and send a transaction.
pub async fn sign_and_send_transaction(
    strategy: &dyn SolanaStrategy,
    transaction: &[u8],
) -> Result<SignAndSendResult, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy
            .connect(ConnectOptions {
                only_if_trusted: false,
            })
            .await?;
    }
    strategy.sign_and_send_transaction(transaction).await
}

/// Sign and send all transactions.
pub async fn sign_and_send_all_transactions(
    strategy: &dyn SolanaStrategy,
    transactions: &[Vec<u8>],
) -> Result<SignAndSendAllResult, Box<dyn std::error::Error + Send + Sync>> {
    if !strategy.is_connected() {
        strategy
            .connect(ConnectOptions {
                only_if_trusted: false,
            })
            .await?;
    }
    strategy.sign_and_send_all_transactions(transactions).await
}

/// Sign in with Solana.
pub async fn sign_in(
    strategy: &dyn SolanaStrategy,
    events: &SolanaEventListeners,
    sign_in_data: &SolanaSignInData,
) -> Result<SignInResult, Box<dyn std::error::Error + Send + Sync>> {
    let result = strategy.sign_in(sign_in_data).await?;
    if !result.address.is_empty() {
        events.trigger_connect(&result.address);
    }
    Ok(result)
}

/// Get a Solana strategy by type.
///
/// In the TS SDK, this creates an `InjectedSolanaStrategy` and calls `load()`.
/// In Rust, we accept a pre-loaded strategy since provider factories are injected.
pub fn get_provider(strategy: Arc<dyn SolanaStrategy>) -> Arc<dyn SolanaStrategy> {
    strategy
}
