//! Error types for the Phantom client.
//!
//! Mirrors the TypeScript error classes from `packages/client/src/errors.ts`.

use crate::types::{PrepareErrorResponse, WalletServiceErrorType};
use thiserror::Error;

/// Errors that can occur during Phantom client operations.
#[derive(Debug, Error)]
pub enum ClientError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Wallet service error (spending limit or transaction blocked).
    #[error("{0}")]
    WalletService(#[from] WalletServiceError),

    /// Configuration error (e.g., missing organization ID).
    #[error("{0}")]
    Config(String),

    /// Unsupported network.
    #[error("Unsupported network ID: {0}")]
    UnsupportedNetwork(String),

    /// Prepare endpoint error.
    #[error("{0}")]
    Prepare(String),

    /// General API error.
    #[error("{0}")]
    Api(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Wallet service error — returned by the API for spending limits and blocked transactions.
#[derive(Debug, Error)]
pub enum WalletServiceError {
    /// Spending limit has been exceeded.
    #[error("Spending limit exceeded: {detail}")]
    SpendingLimitExceeded {
        /// Error title.
        title: String,
        /// Error detail message.
        detail: String,
        /// Request ID.
        request_id: String,
        /// Previous spend in cents.
        previous_spend_cents: Option<u64>,
        /// Transaction spend in cents.
        transaction_spend_cents: Option<u64>,
        /// Total spend in cents.
        total_spend_cents: Option<u64>,
        /// Limit in cents.
        limit_cents: Option<u64>,
    },

    /// Transaction was blocked by security policy.
    #[error("Transaction blocked: {detail}")]
    TransactionBlocked {
        /// Error title.
        title: String,
        /// Error detail message.
        detail: String,
        /// Request ID.
        request_id: String,
        /// Full simulation result when transaction is blocked.
        scanner_result: Option<serde_json::Value>,
    },
}

/// Extract error message from various error types.
///
/// Tries to extract a meaningful message from the error,
/// falling back to the provided fallback message.
pub fn get_error_message(error: &ClientError, fallback_message: &str) -> String {
    match error {
        ClientError::WalletService(ws) => match ws {
            WalletServiceError::SpendingLimitExceeded { detail, title, .. } => {
                if !detail.is_empty() {
                    detail.clone()
                } else if !title.is_empty() {
                    title.clone()
                } else {
                    fallback_message.to_string()
                }
            }
            WalletServiceError::TransactionBlocked { detail, title, .. } => {
                if !detail.is_empty() {
                    detail.clone()
                } else if !title.is_empty() {
                    title.clone()
                } else {
                    fallback_message.to_string()
                }
            }
        },
        ClientError::Http(e) => e.to_string(),
        ClientError::Api(msg) => msg.clone(),
        _ => error.to_string(),
    }
}

/// Extract typed error data from a reqwest error response body.
///
/// Equivalent to the TS `getAxiosErrorData<T>(error)` function.
/// Attempts to deserialize the error body as type `T`.
pub fn get_error_data<T: serde::de::DeserializeOwned>(body: &str) -> Option<T> {
    serde_json::from_str(body).ok()
}

/// Parse a prepare error response into a typed wallet service error.
///
/// Returns `None` if the response doesn't contain a recognized error type.
pub fn parse_wallet_service_error(data: &PrepareErrorResponse) -> Option<WalletServiceError> {
    let error_type = data.error_type.as_ref()?;
    let title = data.title.clone().unwrap_or_default();
    let detail = data.detail.clone().unwrap_or_default();
    let request_id = data.request_id.clone().unwrap_or_default();

    match error_type {
        WalletServiceErrorType::SpendingLimitExceeded => {
            Some(WalletServiceError::SpendingLimitExceeded {
                title,
                detail,
                request_id,
                previous_spend_cents: data.previous_spend_cents,
                transaction_spend_cents: data.transaction_spend_cents,
                total_spend_cents: data.total_spend_cents,
                limit_cents: data.limit_cents,
            })
        }
        WalletServiceErrorType::TransactionBlocked => {
            Some(WalletServiceError::TransactionBlocked {
                title,
                detail,
                request_id,
                scanner_result: data.scanner_result.clone(),
            })
        }
    }
}
