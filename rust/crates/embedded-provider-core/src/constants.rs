//! Constants for the embedded provider.

use crate::types::EmbeddedProviderAuthType;

/// Re-export AddressFormat from the client crate.
pub use phantom_client::AddressFormat;

/// How long an authenticator is valid before it expires (in milliseconds).
/// Default: 7 days.
pub const AUTHENTICATOR_EXPIRATION_TIME_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// How long before expiration should we attempt to renew the authenticator (in milliseconds).
/// Default: 2 days before expiration.
pub const AUTHENTICATOR_RENEWAL_WINDOW_MS: u64 = 2 * 24 * 60 * 60 * 1000;

/// Supported embedded provider auth types.
pub const EMBEDDED_PROVIDER_AUTH_TYPES: &[EmbeddedProviderAuthType] = &[
    EmbeddedProviderAuthType::Google,
    EmbeddedProviderAuthType::Apple,
    EmbeddedProviderAuthType::Phantom,
];
