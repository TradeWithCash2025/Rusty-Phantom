//! Provider implementations for the browser SDK.

pub mod embedded;
pub mod injected;

pub use embedded::{
    BrowserAuthConfig, BrowserAuthProvider, BrowserEmbeddedProvider, BrowserLogger,
    BrowserPlatformAdapter, BrowserPlatformConfig, BrowserPhantomAppProvider, BrowserStorage,
    BrowserURLParamsAccessor,
};
pub use injected::{
    ChainCallbacks, InjectedProvider, InjectedProviderConfig, InjectedWalletEthereumChain,
    InjectedWalletSolanaChain, WalletStandardAccount, WalletStandardFeatures,
    WalletStandardSolanaAdapter, WalletStandardWallet, StandardConnectFeature,
    StandardDisconnectFeature, StandardEventsChangeProperties, StandardEventsFeature,
    SolanaSignMessageFeature, SolanaSignTransactionFeature,
    SolanaSignAndSendTransactionFeature,
};
