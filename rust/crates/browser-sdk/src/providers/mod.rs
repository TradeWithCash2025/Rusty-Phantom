//! Provider implementations for the browser SDK.

pub mod embedded;
pub mod injected;

pub use embedded::{
    BrowserAuthConfig, BrowserAuthProvider, BrowserEmbeddedProvider, BrowserLogger,
    BrowserPlatformAdapter, BrowserPlatformConfig, BrowserPhantomAppProvider, BrowserStorage,
    BrowserURLParamsAccessor,
};
pub use injected::{
    InjectedProvider, InjectedProviderConfig, InjectedWalletEthereumChain,
    InjectedWalletSolanaChain,
};
