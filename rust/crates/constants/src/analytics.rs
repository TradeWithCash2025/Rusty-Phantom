//! Common analytics header names and types that SDKs can use.

use serde::{Deserialize, Serialize};

/// Analytics HTTP header name constants.
pub mod headers {
    /// SDK type header (server, browser-sdk, react-native-sdk).
    pub const SDK_TYPE: &str = "x-phantom-sdk-type";
    /// SDK version header (e.g., "1.0.0").
    pub const SDK_VERSION: &str = "x-phantom-sdk-version";
    /// Platform header (firefox, chrome, safari, ios, android, etc.).
    pub const PLATFORM: &str = "x-phantom-platform";
    /// Wallet type header (app-wallet, user-wallet).
    pub const WALLET_TYPE: &str = "x-phantom-wallet-type";
    /// Application ID header for identifying your app in analytics.
    pub const APP_ID: &str = "x-app-id";
    /// Platform version header (OS version, device model, etc.).
    pub const PLATFORM_VERSION: &str = "x-phantom-platform-version";
}

/// SDK type identifier for analytics headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SdkType {
    /// Server-side SDK.
    Server,
    /// Browser SDK.
    Browser,
    /// React Native SDK.
    #[serde(rename = "react-native")]
    ReactNative,
}

/// Wallet type for analytics headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WalletType {
    /// Application-managed wallet.
    AppWallet,
    /// User-managed wallet.
    UserWallet,
}

/// Base analytics headers required for all SDKs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAnalyticsHeaders {
    /// Type of SDK sending the request.
    pub sdk_type: SdkType,
    /// Version of the SDK.
    pub sdk_version: String,
    /// Application ID (optional).
    pub app_id: Option<String>,
}

/// Server SDK specific analytics headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSdkHeaders {
    /// Base analytics headers.
    #[serde(flatten)]
    pub base: BaseAnalyticsHeaders,
    /// Platform identifier (optional).
    pub platform: Option<String>,
    /// Platform version (optional).
    pub platform_version: Option<String>,
}

/// Browser SDK specific analytics headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSdkHeaders {
    /// Base analytics headers.
    #[serde(flatten)]
    pub base: BaseAnalyticsHeaders,
    /// Wallet type (optional).
    pub wallet_type: Option<WalletType>,
    /// Platform identifier (chrome, firefox, safari, edge, etc.).
    pub platform: Option<String>,
    /// Platform version (full user agent for detailed info).
    pub platform_version: Option<String>,
}

/// React Native SDK specific analytics headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactNativeSdkHeaders {
    /// Base analytics headers.
    #[serde(flatten)]
    pub base: BaseAnalyticsHeaders,
    /// Wallet type (optional).
    pub wallet_type: Option<WalletType>,
    /// Platform identifier (ios, android, etc.).
    pub platform: Option<String>,
    /// Platform version (OS version, device model, etc.).
    pub platform_version: Option<String>,
}

/// Client-side SDK headers (browser or React Native).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientSideSdkHeaders {
    /// Browser SDK headers.
    Browser(BrowserSdkHeaders),
    /// React Native SDK headers.
    ReactNative(ReactNativeSdkHeaders),
}

/// Union type of all possible SDK analytics headers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SdkAnalyticsHeaders {
    /// Server SDK headers.
    Server(ServerSdkHeaders),
    /// Browser SDK headers.
    Browser(BrowserSdkHeaders),
    /// React Native SDK headers.
    ReactNative(ReactNativeSdkHeaders),
}
