//! Parsing utilities for Phantom transaction and message formats.
//!
//! This crate handles conversion between chain-native transaction formats
//! and the base64url/hex encoding used by the Phantom KMS API.

pub mod response_parsers;

// Re-export response parsers
pub use response_parsers::{
    deserialize_solana_transaction, parse_sign_message_response, parse_solana_signed_transaction,
    parse_transaction_response, ParsedSignatureResult, ParsedTransactionResult, SolanaTransaction,
};

use phantom_base64url::{base64url_decode, base64url_encode};
use thiserror::Error;

/// Errors that can occur during transaction parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Unsupported network type.
    #[error("Unsupported network: {0}")]
    UnsupportedNetwork(String),
    /// Unsupported transaction format.
    #[error("Unsupported {0} transaction format")]
    UnsupportedFormat(String),
    /// Base64 decoding failed.
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),
}

/// Parsed transaction ready for KMS submission.
pub struct ParsedTransaction {
    /// The parsed transaction string (base64url for Solana/Sui/Bitcoin, RLP-encoded hex for EVM).
    pub parsed: Option<String>,
    /// Original format of the input transaction.
    pub original_format: String,
}

/// Input transaction in various formats that can be parsed for KMS.
pub enum TransactionInput {
    /// Already-serialized bytes (Uint8Array equivalent).
    Bytes(Vec<u8>),
    /// Hex string (with or without 0x prefix).
    HexString(String),
    /// Base64 encoded string.
    Base64String(String),
    /// Pre-serialized bytes with known format.
    Serialized {
        /// The serialized bytes.
        bytes: Vec<u8>,
        /// The original SDK format (e.g., "@solana/web3.js", "ethers").
        format: String,
    },
}

/// Parse a transaction to KMS format based on network type.
///
/// - Solana: base64url encoding
/// - EVM chains: hex encoding
/// - Sui, Bitcoin: base64url encoding
pub fn parse_to_kms_transaction(
    transaction: TransactionInput,
    network_id: &str,
) -> Result<ParsedTransaction, ParseError> {
    let network_prefix = network_id
        .split(':')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match network_prefix.as_str() {
        "solana" => parse_solana_transaction_to_base64url(transaction),
        "ethereum" | "eip155" | "polygon" | "optimism" | "arbitrum" | "base" => {
            parse_evm_transaction_to_hex(transaction)
        }
        "sui" => parse_sui_transaction_to_base64url(transaction),
        "bitcoin" => parse_bitcoin_transaction_to_base64url(transaction),
        _ => Err(ParseError::UnsupportedNetwork(network_prefix)),
    }
}

/// Parse Solana transaction to base64url.
///
/// Supports raw bytes, base64 strings, and pre-serialized formats.
fn parse_solana_transaction_to_base64url(
    transaction: TransactionInput,
) -> Result<ParsedTransaction, ParseError> {
    match transaction {
        TransactionInput::Bytes(bytes) => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: "bytes".to_string(),
        }),
        TransactionInput::Serialized { bytes, format } => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: format,
        }),
        TransactionInput::Base64String(s) => {
            let bytes = base64url_decode(&s).or_else(|_| {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                STANDARD.decode(&s)
            })?;
            Ok(ParsedTransaction {
                parsed: Some(base64url_encode(&bytes)),
                original_format: "base64".to_string(),
            })
        }
        TransactionInput::HexString(_) => {
            Err(ParseError::UnsupportedFormat("Solana".to_string()))
        }
    }
}

/// Parse EVM transaction to RLP-encoded hex format.
///
/// - RLP hex strings → returned as-is
/// - Raw bytes → converted to hex
fn parse_evm_transaction_to_hex(
    transaction: TransactionInput,
) -> Result<ParsedTransaction, ParseError> {
    match transaction {
        TransactionInput::HexString(s) => {
            let hex = if s.starts_with("0x") {
                s
            } else {
                format!("0x{}", s)
            };
            Ok(ParsedTransaction {
                parsed: Some(hex),
                original_format: "hex".to_string(),
            })
        }
        TransactionInput::Bytes(bytes) => {
            let hex = format!("0x{}", hex_encode(&bytes));
            Ok(ParsedTransaction {
                parsed: Some(hex),
                original_format: "bytes".to_string(),
            })
        }
        TransactionInput::Serialized { bytes, format } => {
            let hex = format!("0x{}", hex_encode(&bytes));
            Ok(ParsedTransaction {
                parsed: Some(hex),
                original_format: format,
            })
        }
        TransactionInput::Base64String(s) => {
            let bytes = base64url_decode(&s)?;
            let hex = format!("0x{}", hex_encode(&bytes));
            Ok(ParsedTransaction {
                parsed: Some(hex),
                original_format: "base64".to_string(),
            })
        }
    }
}

/// Parse Sui transaction to base64url.
fn parse_sui_transaction_to_base64url(
    transaction: TransactionInput,
) -> Result<ParsedTransaction, ParseError> {
    match transaction {
        TransactionInput::Bytes(bytes) => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: "bytes".to_string(),
        }),
        TransactionInput::Serialized { bytes, format } => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: format,
        }),
        _ => Err(ParseError::UnsupportedFormat("Sui".to_string())),
    }
}

/// Parse Bitcoin transaction to base64url.
fn parse_bitcoin_transaction_to_base64url(
    transaction: TransactionInput,
) -> Result<ParsedTransaction, ParseError> {
    match transaction {
        TransactionInput::Bytes(bytes) => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: "bytes".to_string(),
        }),
        TransactionInput::Serialized { bytes, format } => Ok(ParsedTransaction {
            parsed: Some(base64url_encode(&bytes)),
            original_format: format,
        }),
        TransactionInput::HexString(s) => {
            let hex_str = s.strip_prefix("0x").unwrap_or(&s);
            let bytes = hex_decode(hex_str)
                .map_err(|_| ParseError::UnsupportedFormat("Bitcoin".to_string()))?;
            Ok(ParsedTransaction {
                parsed: Some(base64url_encode(&bytes)),
                original_format: "hex".to_string(),
            })
        }
        TransactionInput::Base64String(_) => {
            Err(ParseError::UnsupportedFormat("Bitcoin".to_string()))
        }
    }
}

/// Encode bytes to lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode hex string to bytes.
fn hex_decode(hex: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect()
}
