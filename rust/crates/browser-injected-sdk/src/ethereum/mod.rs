//! Ethereum integration for the browser-injected SDK.

pub mod events;
pub mod operations;
pub mod plugin;
pub mod siwe;
pub mod strategy;
pub mod types;

pub use events::{EthereumEventCallback, EthereumEventListeners};
pub use plugin::{create_ethereum_plugin, Ethereum};
pub use siwe::{create_siwe_message, is_uri};
pub use strategy::{
    EthereumProviderFactory, EthereumStrategy, InjectedEthereumStrategy,
    PhantomEthereumProvider,
};
pub use types::{
    EthereumEventType, EthereumSignInData, EthereumSignInResult, EthereumTransaction,
    ProviderRpcError,
};
