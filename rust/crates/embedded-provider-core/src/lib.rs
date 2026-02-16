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
pub use constants::{
    AUTHENTICATOR_EXPIRATION_TIME_MS, AUTHENTICATOR_RENEWAL_WINDOW_MS, EMBEDDED_PROVIDER_AUTH_TYPES,
};
pub use embedded_chains::{EmbeddedEthereumChain, EmbeddedSolanaChain};
pub use embedded_provider::{EmbeddedProvider, EmbeddedProviderEvent, EventCallback};
pub use interfaces::{
    AuthProvider, AuthResult, DebugLogger, EmbeddedStorage, Keypair, PhantomAppAuthOptions,
    PhantomAppProvider, PhantomConnectOptions, PlatformAdapter, Session, SessionStatus,
    StamperInfo, UrlParamsAccessor,
};
pub use types::{
    AuthOptions, AuthUrlOptions, ConnectErrorEventData, ConnectEventData, ConnectResult,
    ConnectStartEventData, ConnectStatus, DisconnectEventData, EmbeddedProviderAuthType,
    EmbeddedProviderConfig, SignAndSendTransactionParams, SignMessageParams, SignMessageResult,
    SignTransactionParams, SignTypedDataV4Params, SignedTransaction, WalletAddress,
};
pub use utils::{generate_session_id, retry_with_backoff};
