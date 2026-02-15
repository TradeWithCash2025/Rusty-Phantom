//! Provider implementations for the browser SDK.

pub mod embedded;
pub mod injected;

pub use embedded::BrowserEmbeddedProvider;
pub use injected::{InjectedProvider, InjectedProviderConfig};
