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
    /// JSON transaction object (for EVM RLP encoding).
    JsonObject(serde_json::Value),
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
    let network_prefix = network_id.split(':').next().unwrap_or("").to_lowercase();

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
        TransactionInput::HexString(_) | TransactionInput::JsonObject(_) => {
            Err(ParseError::UnsupportedFormat("Solana".to_string()))
        }
    }
}

/// Parse EVM transaction to RLP-encoded hex format.
///
/// - RLP hex strings → returned as-is
/// - Raw bytes → converted to hex
/// - JSON transaction objects → RLP encoded
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
        TransactionInput::JsonObject(obj) => rlp_encode_evm_transaction(&obj),
    }
}

/// RLP encode an EVM transaction from a JSON object.
///
/// Supports both EIP-1559 (type 2) and legacy transaction formats.
/// Mirrors the TS behavior of `ethers.Transaction.from(tx).unsignedSerialized`.
fn rlp_encode_evm_transaction(tx: &serde_json::Value) -> Result<ParsedTransaction, ParseError> {
    use rlp::RlpStream;

    let get_str = |key: &str| -> Option<String> {
        tx.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    let hex_to_bytes = |hex: &str| -> Vec<u8> {
        let h = hex.strip_prefix("0x").unwrap_or(hex);
        if h.is_empty() {
            return vec![];
        }
        hex_decode(h).unwrap_or_default()
    };

    // Check for EIP-1559 (type 2) transaction
    let is_eip1559 = get_str("maxFeePerGas").is_some()
        || get_str("max_fee_per_gas").is_some()
        || get_str("type")
            .as_deref()
            .is_some_and(|t| t == "0x2" || t == "2");

    // Get gas limit: check gasLimit, gas, then default for simple transfers
    let gas_limit = get_str("gasLimit")
        .or_else(|| get_str("gas_limit"))
        .or_else(|| get_str("gas"))
        .unwrap_or_else(|| {
            // Default for simple transfers
            if get_str("to").is_some() && get_str("value").is_some() && get_str("data").is_none() {
                "0x5208".to_string() // 21000
            } else {
                "0x0".to_string()
            }
        });

    // Normalize "to" address to lowercase
    let to = get_str("to").map(|t| t.to_lowercase()).unwrap_or_default();
    let value = get_str("value").unwrap_or_else(|| "0x0".to_string());
    let data = get_str("data").unwrap_or_default();
    let nonce = get_str("nonce").unwrap_or_else(|| "0x0".to_string());
    let chain_id = get_str("chainId")
        .or_else(|| get_str("chain_id"))
        .unwrap_or_else(|| "0x1".to_string());

    if is_eip1559 {
        let max_fee = get_str("maxFeePerGas")
            .or_else(|| get_str("max_fee_per_gas"))
            .unwrap_or_else(|| "0x0".to_string());
        let max_priority_fee = get_str("maxPriorityFeePerGas")
            .or_else(|| get_str("max_priority_fee_per_gas"))
            .unwrap_or_else(|| "0x0".to_string());

        // EIP-1559: 0x02 || RLP([chainId, nonce, maxPriorityFeePerGas, maxFeePerGas, gasLimit, to, value, data, accessList])
        let mut stream = RlpStream::new_list(9);
        stream.append(&hex_to_bytes(&chain_id));
        stream.append(&hex_to_bytes(&nonce));
        stream.append(&hex_to_bytes(&max_priority_fee));
        stream.append(&hex_to_bytes(&max_fee));
        stream.append(&hex_to_bytes(&gas_limit));
        stream.append(&hex_to_bytes(&to));
        stream.append(&hex_to_bytes(&value));
        stream.append(&hex_to_bytes(&data));
        // Access list (empty)
        stream.begin_list(0);

        let rlp_bytes = stream.out();
        // Prepend type byte 0x02
        let mut encoded = vec![0x02];
        encoded.extend_from_slice(&rlp_bytes);

        Ok(ParsedTransaction {
            parsed: Some(format!("0x{}", hex_encode(&encoded))),
            original_format: "json".to_string(),
        })
    } else {
        let gas_price = get_str("gasPrice")
            .or_else(|| get_str("gas_price"))
            .unwrap_or_else(|| "0x0".to_string());

        // Legacy: RLP([nonce, gasPrice, gasLimit, to, value, data, v, r, s])
        // For unsigned, v = chainId, r = 0, s = 0 (EIP-155)
        let mut stream = RlpStream::new_list(9);
        stream.append(&hex_to_bytes(&nonce));
        stream.append(&hex_to_bytes(&gas_price));
        stream.append(&hex_to_bytes(&gas_limit));
        stream.append(&hex_to_bytes(&to));
        stream.append(&hex_to_bytes(&value));
        stream.append(&hex_to_bytes(&data));
        stream.append(&hex_to_bytes(&chain_id)); // v = chainId for EIP-155
        let empty: Vec<u8> = vec![];
        stream.append(&empty); // r = 0
        stream.append(&empty); // s = 0

        let rlp_bytes = stream.out();
        Ok(ParsedTransaction {
            parsed: Some(format!("0x{}", hex_encode(&rlp_bytes))),
            original_format: "json".to_string(),
        })
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
        TransactionInput::Base64String(_) | TransactionInput::JsonObject(_) => {
            Err(ParseError::UnsupportedFormat("Bitcoin".to_string()))
        }
    }
}

/// Convert a @solana/kit-style transaction (with messageBytes) to raw bytes.
///
/// In the TS SDK, `parseSolanaKitTransactionToSolanaWeb3js` wraps a Kit transaction
/// into a web3.js-compatible object. In Rust, this simply extracts the raw bytes
/// from the Kit transaction format for further processing.
pub fn parse_solana_kit_transaction(message_bytes: &[u8]) -> Vec<u8> {
    message_bytes.to_vec()
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
