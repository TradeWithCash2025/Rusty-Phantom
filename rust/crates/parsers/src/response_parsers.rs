//! Chain-specific transaction and message response parsing.
//!
//! This module contains the specific parsing logic for each blockchain network,
//! converting base64url-encoded responses into human-readable formats.

use phantom_base64url::base64url_decode;
use phantom_constants::{get_explorer_url, ExplorerUrlType, NetworkId};
use phantom_utils::is_ethereum_chain;

/// Parsed signature result from a sign-message response.
pub struct ParsedSignatureResult {
    /// Human-readable signature (hex/base58 depending on chain).
    pub signature: String,
    /// Original base64url signature from server.
    pub raw_signature: String,
    /// Explorer link (if supported).
    pub block_explorer: Option<String>,
}

/// Parsed transaction result from a sign-transaction response.
pub struct ParsedTransactionResult {
    /// Transaction hash/signature.
    pub hash: Option<String>,
    /// Original base64url transaction from server.
    pub raw_transaction: String,
    /// Explorer link to transaction.
    pub block_explorer: Option<String>,
}

/// Parse a signed message response from base64url to human-readable format.
pub fn parse_sign_message_response(
    base64_response: &str,
    network_id: NetworkId,
) -> ParsedSignatureResult {
    let network_str = network_id.as_str();
    let network_prefix = network_str
        .split(':')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match network_prefix.as_str() {
        "solana" => parse_solana_signature_response(base64_response),
        "eip155" | "ethereum" => parse_evm_signature_response(base64_response),
        "sui" => parse_sui_signature_response(base64_response),
        "bip122" | "bitcoin" => parse_bitcoin_signature_response(base64_response),
        _ => {
            // Fallback: return the signature as-is
            ParsedSignatureResult {
                signature: base64_response.to_string(),
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
    }
}

/// Parse a transaction response from base64url rawTransaction to extract hash.
///
/// For Ethereum chains, converts base64url to hex format.
pub fn parse_transaction_response(
    base64_raw_transaction: &str,
    network_id: NetworkId,
    hash: Option<&str>,
) -> ParsedTransactionResult {
    let network_str = network_id.as_str();
    let mut raw_transaction = base64_raw_transaction.to_string();

    // For Ethereum chains, decode base64url to hex format
    if is_ethereum_chain(network_str) {
        match base64url_decode(base64_raw_transaction) {
            Ok(tx_bytes) => {
                raw_transaction = format!("0x{}", hex_encode(&tx_bytes));
            }
            Err(_) => {
                // Fallback: assume it's already hex format
                if base64_raw_transaction.starts_with("0x") {
                    raw_transaction = base64_raw_transaction.to_string();
                } else {
                    raw_transaction = format!("0x{}", base64_raw_transaction);
                }
            }
        }
    }

    if let Some(h) = hash {
        ParsedTransactionResult {
            hash: Some(h.to_string()),
            raw_transaction,
            block_explorer: get_explorer_url(network_id, ExplorerUrlType::Transaction, h),
        }
    } else {
        ParsedTransactionResult {
            hash: None,
            raw_transaction,
            block_explorer: None,
        }
    }
}

/// Parse Solana signature response.
///
/// Solana signatures are typically 64 bytes, base58 encoded.
fn parse_solana_signature_response(base64_response: &str) -> ParsedSignatureResult {
    match base64url_decode(base64_response) {
        Ok(signature_bytes) => {
            let signature = bs58::encode(&signature_bytes).into_string();
            ParsedSignatureResult {
                signature,
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
        Err(_) => {
            // Fallback: assume it's already a base58 signature
            ParsedSignatureResult {
                signature: base64_response.to_string(),
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
    }
}

/// Parse EVM signature response.
///
/// EVM signatures are hex-encoded with a 0x prefix.
fn parse_evm_signature_response(base64_response: &str) -> ParsedSignatureResult {
    match base64url_decode(base64_response) {
        Ok(signature_bytes) => {
            let signature = format!("0x{}", hex_encode(&signature_bytes));
            ParsedSignatureResult {
                signature,
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
        Err(_) => ParsedSignatureResult {
            signature: base64_response.to_string(),
            raw_signature: base64_response.to_string(),
            block_explorer: None,
        },
    }
}

/// Parse Sui signature response.
///
/// Sui uses standard base64 encoded signatures.
fn parse_sui_signature_response(base64_response: &str) -> ParsedSignatureResult {
    match base64url_decode(base64_response) {
        Ok(signature_bytes) => {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let signature = STANDARD.encode(&signature_bytes);
            ParsedSignatureResult {
                signature,
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
        Err(_) => ParsedSignatureResult {
            signature: base64_response.to_string(),
            raw_signature: base64_response.to_string(),
            block_explorer: None,
        },
    }
}

/// Parse Bitcoin signature response.
///
/// Bitcoin signatures are DER encoded, represented as hex.
fn parse_bitcoin_signature_response(base64_response: &str) -> ParsedSignatureResult {
    match base64url_decode(base64_response) {
        Ok(signature_bytes) => {
            let signature = hex_encode(&signature_bytes);
            ParsedSignatureResult {
                signature,
                raw_signature: base64_response.to_string(),
                block_explorer: None,
            }
        }
        Err(_) => ParsedSignatureResult {
            signature: base64_response.to_string(),
            raw_signature: base64_response.to_string(),
            block_explorer: None,
        },
    }
}

/// Solana transaction type (legacy or versioned).
#[derive(Debug)]
pub enum SolanaTransaction {
    /// Legacy transaction format.
    Legacy(Vec<u8>),
    /// Versioned transaction format (v0+).
    Versioned(Vec<u8>),
}

/// Parse Solana signed transaction from base64url encoded transaction bytes.
///
/// Supports both legacy Transaction and VersionedTransaction formats.
pub fn parse_solana_signed_transaction(base64_raw_transaction: &str) -> Option<SolanaTransaction> {
    match base64url_decode(base64_raw_transaction) {
        Ok(transaction_bytes) => Some(deserialize_solana_transaction(&transaction_bytes)),
        Err(_) => None,
    }
}

/// Deserialize Solana transaction from raw bytes.
///
/// Supports both legacy Transaction and VersionedTransaction formats.
/// Versioned transactions have a prefix byte where the high bit is set (>= 0x80).
pub fn deserialize_solana_transaction(transaction_bytes: &[u8]) -> SolanaTransaction {
    if transaction_bytes.is_empty() {
        return SolanaTransaction::Legacy(transaction_bytes.to_vec());
    }

    // Versioned transactions have a version byte with the high bit set
    // (the first byte is >= 0x80 for versioned messages)
    if transaction_bytes[0] & 0x80 != 0 {
        SolanaTransaction::Versioned(transaction_bytes.to_vec())
    } else {
        SolanaTransaction::Legacy(transaction_bytes.to_vec())
    }
}

/// Encode bytes to lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
