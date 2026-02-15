//! Chain wrappers for injected wallet providers.
//!
//! These wrappers add debug logging and event forwarding around external
//! [`EthereumChain`] and [`SolanaChain`] providers (e.g. EIP-6963 wallets
//! and Wallet Standard providers).

mod ethereum_chain;
mod solana_chain;

pub use ethereum_chain::InjectedWalletEthereumChain;
pub use solana_chain::InjectedWalletSolanaChain;
