//! Solana integration for the browser-injected SDK.

pub mod events;
pub mod operations;
pub mod plugin;
pub mod strategy;
pub mod types;

pub use events::{PhantomEventCallback, SolanaEventListeners};
pub use plugin::{create_solana_plugin, Solana};
pub use strategy::{
    InjectedSolanaStrategy, PhantomSolanaProvider, SolanaProviderFactory, SolanaStrategy,
};
pub use types::{
    ConnectOptions, DisplayEncoding, PhantomEventType, SendOptions, SignAndSendAllResult,
    SignAndSendResult, SignInResult, SignMessageResult, SolanaSignInData,
};
