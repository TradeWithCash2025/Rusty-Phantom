//! Blockchain chain interface abstractions for the Phantom Connect SDK.
//!
//! Defines the `EthereumChain` and `SolanaChain` traits that abstract
//! interactions with Ethereum/EVM and Solana blockchains respectively.

mod ethereum_chain;
mod solana_chain;

pub use ethereum_chain::{EthTransactionRequest, EthereumChain};
pub use solana_chain::{
    SolanaChain, SolanaConnectOptions, SolanaConnectResult, SolanaNetwork,
    SolanaSignMessageResult, SolanaSendTransactionResult, SolanaSendAllTransactionsResult,
};
