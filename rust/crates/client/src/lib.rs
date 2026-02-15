//! Phantom HTTP client for wallet API communication.
//!
//! This crate provides the `PhantomClient` struct for interacting with the
//! Phantom wallet service API, including wallet creation, transaction signing,
//! organization management, and authenticator management.

pub mod caip2_mappings;
pub mod constants;
pub mod errors;
pub mod phantom_client;
pub mod types;

// Re-export main types
pub use phantom_client::PhantomClient;

// Re-export types
pub use types::{
    AuthenticatorConfig, CreateAuthenticatorParams, CreateWalletResult,
    DeleteAuthenticatorParams, DerivationInfo, GetWalletWithTagParams, GetWalletsResult,
    IdTokenClaims, PhantomClientConfig, PrepareErrorResponse, PrepareResponse,
    SignAndSendTransactionParams, SignMessageParams, SignTransactionParams, SignTypedDataParams,
    SignedTransaction, SignedTransactionResult, SimulationConfig, SpendingLimitConfig,
    SubmissionConfig, UserConfig, Wallet, WalletAddress, WalletServiceErrorData,
    WalletServiceErrorType,
};

// Re-export errors
pub use errors::{ClientError, WalletServiceError};

// Re-export constants
pub use constants::{
    get_client_network_config, get_derivation_path_for_network, AddressFormat, ClientAlgorithm,
    ClientNetworkConfig, Curve, DerivationPath,
};

// Re-export CAIP-2 mappings
pub use caip2_mappings::{
    derive_submission_config, get_network_description, get_network_ids_by_chain,
    get_supported_network_ids, supports_transaction_submission,
};

// Re-export from dependencies
pub use phantom_constants::NetworkId;
