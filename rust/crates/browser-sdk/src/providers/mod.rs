//! Provider implementations for the browser SDK.

pub mod embedded;
pub mod injected;

pub use embedded::{
    BrowserAuthConfig, BrowserAuthProvider, BrowserEmbeddedProvider, BrowserLogger,
    BrowserPhantomAppProvider, BrowserPlatformAdapter, BrowserPlatformConfig, BrowserStorage,
    BrowserURLParamsAccessor,
};
pub use injected::{
    ChainCallbacks, InjectedProvider, InjectedProviderConfig, InjectedWalletEthereumChain,
    InjectedWalletSolanaChain, SolanaSignAndSendTransactionFeature, SolanaSignMessageFeature,
    SolanaSignTransactionFeature, StandardConnectFeature, StandardDisconnectFeature,
    StandardEventsChangeProperties, StandardEventsFeature, WalletStandardAccount,
    WalletStandardFeatures, WalletStandardSolanaAdapter, WalletStandardWallet,
};
