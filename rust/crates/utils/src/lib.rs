//! Utility functions for the Phantom Connect SDK.
//!
//! Provides UUID generation, secure timestamps, and network helper functions.

pub mod network;
pub mod time;
pub mod uuid;

// Re-export commonly used functions
pub use network::{get_chain_prefix, is_ethereum_chain, is_solana_chain};
pub use time::{get_secure_timestamp, get_secure_timestamp_sync};
pub use uuid::{random_string, random_uuid};
