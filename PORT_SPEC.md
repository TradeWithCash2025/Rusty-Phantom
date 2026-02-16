# Port Specification

Per-function specifications for differential testing.

---

## base64urlEncode

- **TS**: `base64urlEncode(data: string | Uint8Array | ArrayLike<number>) -> string`
- **Rust**: `base64url_encode(data: &[u8]) -> String`
- **Input generator**: `vec(any::<u8>(), 0..1024)`
- **Invariants**:
  - Output contains only `[A-Za-z0-9_-]` (no `+`, `/`, `=`)
  - Roundtrip: `decode(encode(x)) == x`
  - Empty input -> empty output
- **Comparison**: Exact string equality
- **Gotchas**: TS version accepts strings (interprets as raw bytes via btoa in browser, Buffer in Node). Oracle always sends byte arrays.

## base64urlDecode

- **TS**: `base64urlDecode(str: string) -> Uint8Array`
- **Rust**: `base64url_decode(s: &str) -> Result<Vec<u8>, DecodeError>`
- **Input generator**: Encode random bytes first -> feed encoded string to both decoders
- **Invariants**:
  - Roundtrip: `decode(encode(x)) == x`
  - Invalid input produces error on both sides
- **Comparison**: Exact byte array equality
- **Gotchas**: TS re-pads with `=` before decoding; Rust `URL_SAFE_NO_PAD` also accepts padded input.

## base64urlDecodeToString

- **TS**: `base64urlDecodeToString(str: string) -> string`
- **Rust**: `base64url_decode_to_string(s: &str) -> Result<String, Base64UrlError>`
- **Input generator**: Encode random UTF-8 strings -> feed encoded string to decode
- **Invariants**:
  - Roundtrip: `decodeToString(stringToBase64url(s)) == s`
- **Comparison**: Exact string equality

## stringToBase64url

- **TS**: `stringToBase64url(str: string) -> string`
- **Rust**: `string_to_base64url(s: &str) -> String`
- **Input generator**: `"\\PC{0,256}"` (printable + unicode chars)
- **Invariants**:
  - Output contains only `[A-Za-z0-9_-]`
  - Roundtrip: `decodeToString(stringToBase64url(s)) == s`
- **Comparison**: Exact string equality

---

## createKeyPairFromSecret

- **TS**: `createKeyPairFromSecret(b58PrivateKey: string) -> Keypair`
- **Rust**: `create_key_pair_from_secret(b58_private_key: &str) -> Result<Keypair, CryptoError>`
- **Input generator**: Generate valid Ed25519 keypair in Rust (32-byte seed -> derive 64-byte key -> base58 encode)
- **Invariants**:
  - publicKey is 32 bytes (base58 decoded)
  - secretKey is 64 bytes (base58 decoded)
  - secretKey[32..64] == publicKey bytes
- **Comparison**: Exact `{publicKey, secretKey}` equality
- **Gotchas**: Both libs (tweetnacl, ed25519-dalek) use same Ed25519 curve, same 64-byte key format.

## signWithSecret

- **TS**: `signWithSecret(secretKey: string | Uint8Array, data: string | Uint8Array | Buffer) -> Uint8Array`
- **Rust**: `sign_with_secret(secret_key: &SecretKeyInput, data: &[u8]) -> Result<Vec<u8>, CryptoError>`
- **Input generator**: Valid keypair (from above) + `vec(any::<u8>(), 0..2048)` for data
- **Invariants**:
  - Signature is always 64 bytes
  - Deterministic: same key + same data = same signature
  - Signature verifies with corresponding public key
- **Comparison**: Exact byte array equality
- **Gotchas**: TS auto-encodes string data as UTF-8. Oracle always receives pre-encoded bytes.

---

## getNetworkConfig

- **TS**: `getNetworkConfig(networkId: NetworkId) -> NetworkConfig | undefined`
- **Rust**: `get_network_config(network_id: NetworkId) -> Option<&'static NetworkConfig>`
- **Input generator**: Enumerate all 18 `NetworkId` variants
- **Invariants**:
  - Every NetworkId returns Some/defined (all are configured)
- **Comparison**: Canonicalize sorted keys (TS uses camelCase, Rust uses serde rename)
- **Gotchas**: TS `NetworkConfig.explorer` has `transactionUrl`/`addressUrl`; Rust has `transaction_url`/`address_url` with serde rename.

## getExplorerUrl

- **TS**: `getExplorerUrl(networkId: NetworkId, type: "transaction" | "address", value: string) -> string | undefined`
- **Rust**: `get_explorer_url(network_id: NetworkId, url_type: ExplorerUrlType, value: &str) -> Option<String>`
- **Input generator**: All NetworkId x {"transaction", "address"} x sample values
- **Comparison**: Exact string equality

## getSupportedNetworks

- **TS**: `getSupportedNetworks() -> NetworkId[]`
- **Rust**: `get_supported_networks() -> Vec<NetworkId>`
- **Input generator**: Single call (no args)
- **Comparison**: Compare as sorted string sets

## getNetworksByChain

- **TS**: `getNetworksByChain(chain: string) -> NetworkId[]`
- **Rust**: `get_networks_by_chain(chain: &str) -> Vec<NetworkId>`
- **Input generator**: Known chains: "solana", "ethereum", "polygon", "base", "arbitrum", "monad", "bitcoin", "sui", plus unknown: "nonexistent"
- **Comparison**: Compare as sorted string sets

## chainIdToNetworkId

- **TS**: `chainIdToNetworkId(chainId: number) -> NetworkId | undefined`
- **Rust**: `chain_id_to_network_id(chain_id: u64) -> Option<NetworkId>`
- **Input generator**: Known chain IDs (1, 11155111, 137, 80002, 8453, 84532, 42161, 421614, 143, 10143) + unknown (999999)
- **Comparison**: Exact

## networkIdToChainId

- **TS**: `networkIdToChainId(networkId: NetworkId) -> number | undefined`
- **Rust**: `network_id_to_chain_id(network_id: NetworkId) -> Option<u64>`
- **Input generator**: Enumerate all NetworkId variants
- **Comparison**: Exact

## getProviderName

- **TS**: `getProviderName(provider: ProviderNameKey | string) -> string`
- **Rust**: `get_provider_name(provider: &str) -> &str`
- **Input generator**: Known keys: "google", "apple", "phantom", "device", "injected", "deeplink" + random `[a-z]{1,20}` strings
- **Comparison**: Exact

---

## isEthereumChain

- **TS**: `isEthereumChain(networkId: string) -> boolean`
- **Rust**: `is_ethereum_chain(network_id: &str) -> bool`
- **Input generator**: Known network IDs + random strings
- **Comparison**: Exact

## isSolanaChain

- **TS**: `isSolanaChain(networkId: string) -> boolean`
- **Rust**: `is_solana_chain(network_id: &str) -> bool`
- **Input generator**: Known network IDs + random strings
- **Comparison**: Exact

## getChainPrefix

- **TS**: `getChainPrefix(networkId: string) -> string`
- **Rust**: `get_chain_prefix(network_id: &str) -> String`
- **Input generator**: Known network IDs + random strings
- **Comparison**: Exact

---

## ApiKeyStamper.stamp (PKI)

- **TS**: `new ApiKeyStamper({apiSecretKey}).stamp({data: Buffer, type: "PKI"}) -> Promise<string>`
- **Rust**: `ApiKeyStamper::new(config).stamp(StampParams::Pki{data}) -> Result<String, ...>`
- **Input generator**: Generate Ed25519 keypair, random `vec(any::<u8>(), 1..512)` for data
- **Invariants**:
  - Stamp is valid base64url
  - Decoded stamp is valid JSON with keys: publicKey, signature, kind, algorithm
  - kind == "PKI"
  - Signature verifies against publicKey
- **Comparison**: Decode base64url -> parse JSON -> canonicalize sorted keys -> exact
- **Gotchas**: JSON key ordering may differ between `JSON.stringify` and `serde_json::to_string`. Always canonicalize.
