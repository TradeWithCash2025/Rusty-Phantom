//! Core embedded provider functionality for the Phantom Connect SDK.
//!
//! This crate provides the platform-agnostic core of the embedded wallet provider,
//! including authentication flows, session management, transaction signing,
//! and chain-specific adapters.

pub mod constants;
pub mod embedded_chains;
pub mod embedded_provider;
pub mod interfaces;
pub mod types;
pub mod utils;

// Re-export main types
pub use embedded_provider::{EmbeddedProvider, EmbeddedProviderEvent, EventCallback};
pub use interfaces::{
    AuthProvider, AuthResult, DebugLogger, EmbeddedStorage, Keypair, PlatformAdapter,
    PhantomAppAuthOptions, PhantomAppProvider, PhantomConnectOptions, Session, SessionStatus,
    StamperInfo, UrlParamsAccessor,
};
pub use types::{
    AuthOptions, AuthUrlOptions, ConnectResult, ConnectStatus, EmbeddedProviderAuthType,
    EmbeddedProviderConfig, SignAndSendTransactionParams, SignMessageParams, SignMessageResult,
    SignTransactionParams, SignTypedDataV4Params, SignedTransaction, WalletAddress,
};
pub use embedded_chains::{EmbeddedEthereumChain, EmbeddedSolanaChain};
pub use utils::{generate_session_id, retry_with_backoff};
pub use constants::{
    AUTHENTICATOR_EXPIRATION_TIME_MS, AUTHENTICATOR_RENEWAL_WINDOW_MS,
    EMBEDDED_PROVIDER_AUTH_TYPES,
};
