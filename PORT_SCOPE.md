# Port Scope

Reference commit: HEAD (current)

## @phantom/base64url (packages/base64url/src/index.ts) → phantom-base64url (rust/crates/base64url/src/lib.rs)

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `base64urlEncode` | function | `base64url_encode` | PORTED |
| `base64urlDecode` | function | `base64url_decode` | PORTED |
| `base64urlDecodeToString` | function | `base64url_decode_to_string` | PORTED |
| `stringToBase64url` | function | `string_to_base64url` | PORTED |

## @phantom/crypto (packages/crypto/src/index.ts) → phantom-crypto (rust/crates/crypto/src/lib.rs)

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `Keypair` | interface | `Keypair` (struct) | PORTED |
| `generateKeyPair` | function | `generate_key_pair` | PORTED |
| `createKeyPairFromSecret` | function | `create_key_pair_from_secret` | PORTED |
| `signWithSecret` | function | `sign_with_secret` | PORTED |

## @phantom/constants (packages/constants/src/) → phantom-constants (rust/crates/constants/src/)

### authenticators

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `DEFAULT_AUTHENTICATOR_ALGORITHM` | const | `DEFAULT_AUTHENTICATOR_ALGORITHM` | PORTED |

### network-ids

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `NetworkId` | enum | `NetworkId` (enum) | PORTED |

### networks

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `InternalNetworkCaip` | type | `InternalNetworkCaip` (enum) | PORTED |
| `NetworkConfig` | interface | `NetworkConfig` (struct) | PORTED |
| `NETWORK_CONFIGS` | const | static `NETWORK_CONFIGS` (via `get_network_config`) | PORTED |
| `getNetworkConfig` | function | `get_network_config` | PORTED |
| `getExplorerUrl` | function | `get_explorer_url` | PORTED |
| `getSupportedNetworks` | function | `get_supported_networks` | PORTED |
| `getNetworksByChain` | function | `get_networks_by_chain` | PORTED |
| `chainIdToNetworkId` | function | `chain_id_to_network_id` | PORTED |
| `networkIdToChainId` | function | `network_id_to_chain_id` | PORTED |
| `networkIdToInternalCaip` | function | `network_id_to_internal_caip` | PORTED |
| `internalCaipToNetworkId` | function | `internal_caip_to_network_id` | PORTED |

### environments

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `DEFAULT_AUTH_URL` | const | `DEFAULT_AUTH_URL` | PORTED |
| `DEFAULT_WALLET_API_URL` | const | `DEFAULT_WALLET_API_URL` | PORTED |
| `DEFAULT_EMBEDDED_WALLET_TYPE` | const | `DEFAULT_EMBEDDED_WALLET_TYPE` | PORTED |

### provider-names

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `ProviderNameKey` | type | `ProviderNameKey` (enum) | PORTED |
| `PROVIDER_NAMES` | const | (inline in `get_provider_name`) | PORTED |
| `getProviderName` | function | `get_provider_name` | PORTED |

### analytics

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `ANALYTICS_HEADERS` | const | `analytics::headers` (module of consts) | PORTED |
| `BaseAnalyticsHeaders` | interface | `BaseAnalyticsHeaders` (struct) | PORTED |
| `ServerSdkHeaders` | interface | `ServerSdkHeaders` (struct) | PORTED |
| `BrowserSdkHeaders` | interface | `BrowserSdkHeaders` (struct) | PORTED |
| `ReactNativeSdkHeaders` | interface | `ReactNativeSdkHeaders` (struct) | PORTED |
| `ClientSideSdkHeaders` | type | `ClientSideSdkHeaders` (enum) | PORTED |
| `SdkAnalyticsHeaders` | type | `SdkAnalyticsHeaders` (enum) | PORTED |

### icons

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `PHANTOM_ICON` | const | `PHANTOM_ICON` | PORTED |

## @phantom/utils (packages/utils/src/) → phantom-utils (rust/crates/utils/src/)

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `randomUUID` | function | `random_uuid` | PORTED |
| `randomString` | function | `random_string` | PORTED |
| `getSecureTimestamp` | function | `get_secure_timestamp` | PORTED |
| `getSecureTimestampSync` | function | `get_secure_timestamp_sync` | PORTED |
| `getChainPrefix` | function | `get_chain_prefix` | PORTED |
| `isEthereumChain` | function | `is_ethereum_chain` | PORTED |
| `isSolanaChain` | function | `is_solana_chain` | PORTED |

## @phantom/sdk-types (packages/sdk-types/src/) → phantom-sdk-types (rust/crates/sdk-types/src/)

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `Stamper` | interface | `Stamper` (trait) | PORTED |
| `StamperKeyInfo` | interface | `StamperKeyInfo` (struct) | PORTED |
| `StamperWithKeyManagement` | interface | `StamperWithKeyManagement` (trait) | PORTED |

## @phantom/api-key-stamper (packages/api-key-stamper/src/) → phantom-api-key-stamper (rust/crates/api-key-stamper/src/)

| TS Export | Kind | Rust Equivalent | Status |
|---|---|---|---|
| `ApiKeyStamperConfig` | interface | `ApiKeyStamperConfig` (struct) | PORTED |
| `ApiKeyStamper` | class | `ApiKeyStamper` (struct + Stamper impl) | PORTED |
