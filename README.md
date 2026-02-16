# Rusty-Phantom

**An unofficial, experimental Rust port of [Phantom's Connect SDK](https://github.com/nicholasgasior/phantom-connect-sdk).**

> **WARNING: This is an independent development project and has NOT been tested in production. It is not affiliated with, endorsed by, or maintained by Phantom Technologies, Inc. Use at your own risk.**

## What Is This?

This repository is a TypeScript-to-Rust rewrite of the [Phantom Connect SDK](https://github.com/nicholasgasior/phantom-connect-sdk) — a comprehensive suite of SDKs for integrating Phantom wallet across different platforms. The original SDK is written in TypeScript and maintained by the [Phantom](https://phantom.app) team.

The goal of this project is to provide a native Rust implementation of the same SDK surface, enabling Rust-native server applications, CLI tools, and WASM targets to interact with Phantom wallets without depending on a Node.js runtime.

### Original Project

- **Repository**: [github.com/nicholasgasior/phantom-connect-sdk](https://github.com/nicholasgasior/phantom-connect-sdk)
- **Author**: [Phantom Technologies, Inc](https://phantom.app)
- **License**: MIT (see [LICENSE](./LICENSE))
- **Original language**: TypeScript

All credit for the SDK design, API surface, and business logic belongs to the Phantom team. This port aims to faithfully reproduce their work in Rust.

## Project Status

**Status: Experimental / In Development**

This project is a work in progress. While significant portions of the SDK have been ported and verified against the TypeScript reference implementation, it has **not** been:

- Audited for security
- Tested against live Phantom APIs in production
- Reviewed by the original Phantom team
- Used in any production application

**Do not use this in production without thorough independent review and testing.**

## Repository Structure

The repository contains both the original TypeScript source (for reference) and the Rust port:

```
Rusty-Phantom/
├── packages/           # Original TypeScript SDK (19 packages, untouched)
├── rust/               # Rust workspace (20 crates)
│   ├── Cargo.toml      # Workspace root
│   └── crates/
│       ├── base64url/              # URL-safe base64 encoding
│       ├── crypto/                 # Ed25519 key generation and signing
│       ├── constants/              # Network configs, chain IDs, providers
│       ├── utils/                  # Chain prefix parsing, network detection
│       ├── sdk-types/              # Shared types (Stamper trait, StampParams)
│       ├── api-key-stamper/        # Ed25519 request signing
│       ├── chain-interfaces/       # Solana/Ethereum chain interfaces
│       ├── parsers/                # API response parsers
│       ├── client/                 # HTTP API client
│       ├── server-sdk/             # Server-side SDK
│       ├── embedded-provider-core/ # Embedded wallet orchestration
│       ├── indexed-db-stamper/     # Browser IndexedDB stamper (WASM)
│       ├── browser-injected-sdk/   # Browser extension integration
│       ├── browser-sdk/            # Browser SDK
│       ├── mcp-server/             # MCP server implementation
│       ├── phantom-openclaw-plugin/# OpenClaw plugin
│       ├── ui/                     # Platform UI abstractions
│       └── differential-tests/     # Cross-language verification tests
├── tools/              # Test infrastructure (TS oracle, fixture generator)
├── fixtures/           # Shared golden test vectors (JSON)
├── FAILURES/           # Mismatch artifacts from differential tests
├── PORT_SCOPE.md       # TS export → Rust mapping status
├── MAPPING.json        # Machine-readable mapping table
└── PORT_SPEC.md        # Per-function test specifications
```

## Verification: Cross-Language Differential Testing

A key contribution of this project is a rigorous cross-language differential testing system that proves behavioral parity between the TypeScript and Rust implementations. This goes beyond simple unit testing — it uses property-based testing to discover edge-case divergences.

### How It Works

1. **TS Oracle**: A persistent Node.js process (`tools/ts_oracle.mjs`) accepts NDJSON requests over stdin, calls the original TypeScript functions, and returns results over stdout.

2. **Rust Test Harness**: The `phantom-differential-tests` crate uses [proptest](https://crates.io/crates/proptest) to generate randomized inputs, feeds them to both the Rust implementation (natively) and the TypeScript implementation (via the oracle), and asserts the outputs match.

3. **Shared Fixtures**: Golden test vectors in `fixtures/` provide a deterministic baseline. Both implementations are tested against these vectors.

4. **Failure Artifacts**: Any mismatch is written to `FAILURES/` as a JSON artifact with full context for reproduction.

### What's Covered

| Module | Functions Tested | Method |
|--------|-----------------|--------|
| base64url | `encode`, `decode`, `decodeToString`, `stringToBase64url` | proptest (256 cases) + fixtures |
| crypto | `createKeyPairFromSecret`, `signWithSecret` | proptest (64 cases) + fixtures |
| constants | `getNetworkConfig`, `getExplorerUrl`, `getSupportedNetworks`, `getNetworksByChain`, `chainIdToNetworkId`, `networkIdToChainId`, `getProviderName` | exhaustive enumeration + fixtures |
| utils | `getChainPrefix`, `isEthereumChain`, `isSolanaChain` | proptest (256 cases) + known IDs |
| api-key-stamper | `ApiKeyStamper.stamp` (PKI) | proptest (32 cases) + fixtures |

### Test Results

**33 tests across 7 suites, all passing:**

```
test result: ok. 3 passed  (api_key_stamper_diff)
test result: ok. 7 passed  (base64url_diff)
test result: ok. 7 passed  (constants_diff)
test result: ok. 4 passed  (crypto_diff)
test result: ok. 4 passed  (shared_fixtures)
test result: ok. 8 passed  (utils_diff)
```

Additionally, all 61+ existing Rust unit tests across the workspace pass with zero regressions.

### Bugs Found by Differential Testing

The differential testing system caught two real porting bugs:

1. **Algorithm serialization mismatch**: The Rust `Algorithm` enum serialized as `"ed25519"` (lowercase), but the TypeScript SDK uses `"Ed25519"` (PascalCase). Fixed in `constants/src/authenticators.rs` and `api-key-stamper/src/lib.rs`.

2. **NetworkConfig JSON key mismatch**: Rust serialized struct fields as `snake_case` (e.g., `transaction_url`) and included `null` for `None` fields. TypeScript uses `camelCase` (`transactionUrl`) and omits `undefined` fields. Fixed with serde attributes in `constants/src/networks.rs`.

## Running the Tests

### Prerequisites

```bash
# Install Node.js dependencies and build TypeScript packages
corepack enable && corepack install
yarn install && yarn build:packages
```

### Run Differential Tests

```bash
# Must be single-threaded (shared oracle process)
cd rust && cargo test -p phantom-differential-tests -- --test-threads=1
```

### Run All Rust Tests

```bash
cd rust && cargo test
```

### Regenerate Fixtures

```bash
node tools/generate_fixtures.mjs
```

## Contributing

This is an experimental project. If you're interested in contributing:

1. Read `PORT_SCOPE.md` for the full TS-to-Rust mapping status
2. Read `PORT_SPEC.md` for per-function test specifications
3. Run the differential tests to verify your changes don't break parity
4. Any new Rust implementations should have corresponding differential tests

## License

This project is licensed under the MIT License — see [LICENSE](./LICENSE) for details.

The original Phantom Connect SDK is Copyright (c) 2024 Phantom Technologies, Inc, also under the MIT License.

## Acknowledgments

All credit for the SDK architecture, API design, and business logic belongs to the **Phantom** team. This project would not exist without their excellent open-source work on the [Phantom Connect SDK](https://github.com/nicholasgasior/phantom-connect-sdk).

## Disclaimer

This software is provided "as is", without warranty of any kind. This is an independent project not affiliated with Phantom Technologies, Inc. The authors are not responsible for any damages or losses arising from the use of this software. Always verify the correctness of any cryptographic operations independently before deploying to production.
