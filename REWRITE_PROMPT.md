# Phantom Connect SDK: TypeScript-to-Rust Rewrite Prompt

## Mission

You are rewriting the **Phantom Connect SDK** from TypeScript/JavaScript to idiomatic Rust — one file at a time. The original codebase is a monorepo of 19 packages under `packages/` and 8 example apps under `examples/`. Your job is to produce a fully functional Rust workspace that preserves every piece of business logic, every public API contract, and every behavioral nuance from the original.

---

## Project Layout

The Rust workspace lives in `rust/` at the repository root. The original TypeScript source stays untouched for reference.

```
rust/
├── Cargo.toml                  (workspace root)
├── crates/
│   ├── constants/              ← packages/constants
│   ├── utils/                  ← packages/utils
│   ├── base64url/              ← packages/base64url
│   ├── crypto/                 ← packages/crypto
│   ├── sdk-types/              ← packages/sdk-types
│   ├── api-key-stamper/        ← packages/api-key-stamper
│   ├── parsers/                ← packages/parsers
│   ├── chain-interfaces/       ← packages/chain-interfaces
│   ├── client/                 ← packages/client
│   ├── server-sdk/             ← packages/server-sdk
│   ├── embedded-provider-core/ ← packages/embedded-provider-core
│   ├── indexed-db-stamper/     ← packages/indexed-db-stamper
│   ├── browser-injected-sdk/   ← packages/browser-injected-sdk
│   ├── browser-sdk/            ← packages/browser-sdk
│   ├── mcp-server/             ← packages/mcp-server
│   ├── phantom-openclaw-plugin/← packages/phantom-openclaw-plugin
│   ├── ui/                     ← packages/ui (platform abstraction only, no JSX)
│   ├── react-sdk/              ← packages/react-sdk (skip — React-specific)
│   └── react-native-sdk/       ← packages/react-native-sdk (skip — RN-specific)
└── examples/
    ├── client-demo/            ← examples/client-demo-app
    └── server-sdk-demo/        ← examples/server-sdk-examples
```

> **Skip React/React Native packages and browser-only examples entirely.** They have no Rust equivalent. Focus on the core SDK, server-side, and CLI-usable packages. If a skipped file contains reusable logic (types, utils), extract that logic into the appropriate core crate.

---

## Conversion Order (Dependency-First)

Process files **strictly** in this order. Each phase must be completed before the next begins. Within each phase, convert files in the listed order.

### Phase 1: Foundation (zero internal dependencies)

```
1.  packages/constants/src/network-ids.ts
2.  packages/constants/src/networks.ts
3.  packages/constants/src/environments.ts
4.  packages/constants/src/authenticators.ts
5.  packages/constants/src/analytics.ts
6.  packages/constants/src/icons.ts
7.  packages/constants/src/provider-names.ts
8.  packages/constants/src/index.ts
9.  packages/utils/src/uuid.ts
10. packages/utils/src/time.ts
11. packages/utils/src/network.ts
12. packages/utils/src/index.ts
13. packages/base64url/src/index.ts
14. packages/crypto/src/index.ts
```

### Phase 2: Type Contracts & Stamping

```
15. packages/sdk-types/src/index.ts
16. packages/api-key-stamper/src/index.ts
17. packages/chain-interfaces/src/interfaces/IEthereumChain.ts
18. packages/chain-interfaces/src/interfaces/ISolanaChain.ts
19. packages/chain-interfaces/src/interfaces/index.ts
20. packages/chain-interfaces/src/index.ts
```

### Phase 3: Parsing & Client

```
21. packages/parsers/src/response-parsers.ts
22. packages/parsers/src/index.ts
23. packages/client/src/types.ts
24. packages/client/src/errors.ts
25. packages/client/src/constants.ts
26. packages/client/src/caip2-mappings.ts
27. packages/client/src/utils.ts
28. packages/client/src/PhantomClient.ts
29. packages/client/src/index.ts
```

### Phase 4: Providers & Server SDK

```
30. packages/embedded-provider-core/src/types.ts
31. packages/embedded-provider-core/src/constants.ts
32. packages/embedded-provider-core/src/interfaces/auth.ts
33. packages/embedded-provider-core/src/interfaces/storage.ts
34. packages/embedded-provider-core/src/interfaces/platform.ts
35. packages/embedded-provider-core/src/interfaces/url-params.ts
36. packages/embedded-provider-core/src/interfaces/index.ts
37. packages/embedded-provider-core/src/utils/retry.ts
38. packages/embedded-provider-core/src/utils/session.ts
39. packages/embedded-provider-core/src/chains/EthereumChain.ts
40. packages/embedded-provider-core/src/chains/SolanaChain.ts
41. packages/embedded-provider-core/src/chains/index.ts
42. packages/embedded-provider-core/src/embedded-provider.ts
43. packages/embedded-provider-core/src/index.ts
44. packages/server-sdk/src/types.ts
45. packages/server-sdk/src/index.ts
```

### Phase 5: Browser & Injected SDKs

```
46. packages/browser-injected-sdk/src/types.ts
47. packages/browser-injected-sdk/src/solana/types.ts
48. packages/browser-injected-sdk/src/solana/getProvider.ts
49. packages/browser-injected-sdk/src/solana/connect.ts
50. packages/browser-injected-sdk/src/solana/disconnect.ts
51. packages/browser-injected-sdk/src/solana/getAccount.ts
52. packages/browser-injected-sdk/src/solana/signMessage.ts
53. packages/browser-injected-sdk/src/solana/signTransaction.ts
54. packages/browser-injected-sdk/src/solana/signAllTransactions.ts
55. packages/browser-injected-sdk/src/solana/signAndSendTransaction.ts
56. packages/browser-injected-sdk/src/solana/signAndSendAllTransactions.ts
57. packages/browser-injected-sdk/src/solana/signIn.ts
58. packages/browser-injected-sdk/src/solana/eventListeners.ts
59. packages/browser-injected-sdk/src/solana/strategies/types.ts
60. packages/browser-injected-sdk/src/solana/strategies/injected.ts
61. packages/browser-injected-sdk/src/solana/plugin.ts
62. packages/browser-injected-sdk/src/solana/index.ts
63. packages/browser-injected-sdk/src/ethereum/types.ts
64. packages/browser-injected-sdk/src/ethereum/getProvider.ts
65. packages/browser-injected-sdk/src/ethereum/chainUtils.ts
66. packages/browser-injected-sdk/src/ethereum/connect.ts
67. packages/browser-injected-sdk/src/ethereum/disconnect.ts
68. packages/browser-injected-sdk/src/ethereum/getAccounts.ts
69. packages/browser-injected-sdk/src/ethereum/signMessage.ts
70. packages/browser-injected-sdk/src/ethereum/sendTransaction.ts
71. packages/browser-injected-sdk/src/ethereum/signIn.ts
72. packages/browser-injected-sdk/src/ethereum/siwe.ts
73. packages/browser-injected-sdk/src/ethereum/eventListeners.ts
74. packages/browser-injected-sdk/src/ethereum/strategies/types.ts
75. packages/browser-injected-sdk/src/ethereum/strategies/injected.ts
76. packages/browser-injected-sdk/src/ethereum/plugin.ts
77. packages/browser-injected-sdk/src/ethereum/index.ts
78. packages/browser-injected-sdk/src/extension/isInstalled.ts
79. packages/browser-injected-sdk/src/extension/plugin.ts
80. packages/browser-injected-sdk/src/extension/index.ts
81. packages/browser-injected-sdk/src/auto-confirm/types.ts
82. packages/browser-injected-sdk/src/auto-confirm/getProvider.ts
83. packages/browser-injected-sdk/src/auto-confirm/autoConfirmEnable.ts
84. packages/browser-injected-sdk/src/auto-confirm/autoConfirmDisable.ts
85. packages/browser-injected-sdk/src/auto-confirm/autoConfirmStatus.ts
86. packages/browser-injected-sdk/src/auto-confirm/autoConfirmSupportedChains.ts
87. packages/browser-injected-sdk/src/auto-confirm/plugin.ts
88. packages/browser-injected-sdk/src/auto-confirm/index.ts
89. packages/browser-injected-sdk/src/index.ts
```

### Phase 6: Browser SDK Core

```
90.  packages/browser-sdk/src/types.ts
91.  packages/browser-sdk/src/debug.ts
92.  packages/browser-sdk/src/polyfills.ts
93.  packages/browser-sdk/src/utils/browser-detection.ts
94.  packages/browser-sdk/src/utils/auth-callback.ts
95.  packages/browser-sdk/src/utils/deeplink.ts
96.  packages/browser-sdk/src/wallets/custom-wallets.ts
97.  packages/browser-sdk/src/wallets/registry.ts
98.  packages/browser-sdk/src/wallets/discovery.ts
99.  packages/browser-sdk/src/providers/embedded/adapters/auth.ts
100. packages/browser-sdk/src/providers/embedded/adapters/logger.ts
101. packages/browser-sdk/src/providers/embedded/adapters/storage.ts
102. packages/browser-sdk/src/providers/embedded/adapters/phantom-app.ts
103. packages/browser-sdk/src/providers/embedded/adapters/url-params.ts
104. packages/browser-sdk/src/providers/embedded/adapters/index.ts
105. packages/browser-sdk/src/providers/embedded/index.ts
106. packages/browser-sdk/src/providers/injected/chains/ChainCallbacks.ts
107. packages/browser-sdk/src/providers/injected/chains/walletStandardTypes.ts
108. packages/browser-sdk/src/providers/injected/chains/WalletStandardSolanaAdapter.ts
109. packages/browser-sdk/src/providers/injected/chains/InjectedWalletSolanaChain.ts
110. packages/browser-sdk/src/providers/injected/chains/InjectedWalletEthereumChain.ts
111. packages/browser-sdk/src/providers/injected/index.ts
112. packages/browser-sdk/src/ProviderManager.ts
113. packages/browser-sdk/src/isPhantomLoginAvailable.ts
114. packages/browser-sdk/src/waitForPhantomExtension.ts
115. packages/browser-sdk/src/BrowserSDK.ts
116. packages/browser-sdk/src/index.ts
```

### Phase 7: MCP Server & Plugin

```
117. packages/mcp-server/src/utils/logger.ts
118. packages/mcp-server/src/utils/amount.ts
119. packages/mcp-server/src/utils/network.ts
120. packages/mcp-server/src/utils/solana.ts
121. packages/mcp-server/src/session/types.ts
122. packages/mcp-server/src/session/storage.ts
123. packages/mcp-server/src/session/manager.ts
124. packages/mcp-server/src/auth/callback-server.ts
125. packages/mcp-server/src/auth/dcr.ts
126. packages/mcp-server/src/auth/oauth.ts
127. packages/mcp-server/src/tools/types.ts
128. packages/mcp-server/src/tools/get-wallet-addresses.ts
129. packages/mcp-server/src/tools/sign-message.ts
130. packages/mcp-server/src/tools/sign-transaction.ts
131. packages/mcp-server/src/tools/transfer-tokens.ts
132. packages/mcp-server/src/tools/buy-token.ts
133. packages/mcp-server/src/tools/index.ts
134. packages/mcp-server/src/server.ts
135. packages/mcp-server/src/index.ts
136. packages/phantom-openclaw-plugin/src/client/types.ts
137. packages/phantom-openclaw-plugin/src/session.ts
138. packages/phantom-openclaw-plugin/src/tools/register-tools.ts
139. packages/phantom-openclaw-plugin/src/index.ts
```

### Phase 8: IndexedDB Stamper & UI Abstractions

```
140. packages/indexed-db-stamper/src/index.ts
141. packages/ui/src/utils/index.ts
142. packages/ui/src/themes/index.ts
143. packages/ui/src/hooks/useTheme.ts
144. packages/ui/src/index.ts
```

### Phase 9: Tests (convert alongside or after each source file)

For every `*.test.ts` / `*.test.tsx` file, create a corresponding `#[cfg(test)] mod tests { ... }` block inside the Rust source file, **or** a separate `tests/` integration test file within the crate. Convert test logic to use standard Rust testing with `#[test]`, `#[tokio::test]`, and assertion macros.

### Phase 10: Examples

```
145. examples/client-demo-app/src/index.ts
146. examples/client-demo-app/src/multi-auth-demo.ts
147. examples/server-sdk-examples/src/server-sdk-demo.ts
148. examples/server-sdk-examples/src/list-wallets.ts
149. examples/server-sdk-examples/src/sign-message.ts
```

---

## Per-File Conversion Procedure

For **every single file**, execute these steps in order:

### Step 1: Read & Analyze the TypeScript Source

```
Read the full contents of the TypeScript file.
Identify and document:
  - All exported types, interfaces, enums, constants, and functions
  - All imports (internal cross-package and external npm packages)
  - Async patterns (Promise, async/await, callbacks, event emitters)
  - Error handling patterns (try/catch, custom error types, Result-like returns)
  - Nullable/optional patterns (T | null, T | undefined, optional chaining)
  - Generic type parameters and constraints
  - Side effects (global mutations, module-level initialization)
  - Platform-specific code (browser APIs, Node.js APIs)
```

### Step 2: Write the Rust Equivalent

Apply the **Type Mapping Table** and **Idiom Translation Rules** below. Write the complete `.rs` file.

### Step 3: Verification Diff

After writing the Rust file, perform a **structured logic diff**. Produce a table comparing every exported symbol:

```markdown
| # | TypeScript Symbol              | Rust Symbol                        | Logic Match | Notes                              |
|---|--------------------------------|------------------------------------|-------------|------------------------------------|
| 1 | export function foo(x: string) | pub fn foo(x: &str)                | YES         |                                    |
| 2 | export interface Bar           | pub struct Bar / pub trait Bar      | YES         | Interface → trait (has methods)    |
| 3 | export const BAZ = 42          | pub const BAZ: i32 = 42            | YES         |                                    |
| 4 | private method _helper()       | fn helper(&self)                   | YES         | Private by default in Rust         |
| 5 | export type Result = A | B     | pub enum Result { A(A), B(B) }     | ADAPTED     | Union → enum, idiomatic Rust       |
```

Rules for the diff:
- **Logic Match = YES**: Identical behavior, possibly different syntax.
- **Logic Match = ADAPTED**: Behavior preserved but approach changed for Rust idioms (e.g., `Option` instead of `null`, `Result` instead of `throw`). Explain why in Notes.
- **Logic Match = SKIPPED**: Symbol intentionally omitted (e.g., TypeScript-only type guard, JSX component). Explain why.
- **Logic Match = NO**: This should **never** appear. If logic cannot be preserved, stop and flag the issue.

Every function body must be compared for:
- Control flow (branches, loops, early returns)
- Error propagation paths
- Side effects and mutation
- Return value semantics

### Step 4: Compile Check

After writing the file, mentally verify (or actually run):
```bash
cargo check -p <crate-name>
```
If it won't compile due to missing dependencies from later phases, add `// TODO: depends on crate X (Phase N)` stubs with the correct type signatures so the file compiles in isolation.

---

## Type Mapping Table

| TypeScript                          | Rust                                              |
|-------------------------------------|---------------------------------------------------|
| `string`                            | `String` (owned) or `&str` (borrowed)             |
| `number` (integer context)          | `u32`, `u64`, `i32`, `i64` (pick smallest safe)   |
| `number` (float context)            | `f64`                                             |
| `boolean`                           | `bool`                                            |
| `T \| null` / `T \| undefined`     | `Option<T>`                                       |
| `T[]` / `Array<T>`                  | `Vec<T>`                                          |
| `Record<string, T>` / `{ [k]: T }` | `HashMap<String, T>` or `BTreeMap<String, T>`     |
| `Promise<T>`                        | `async fn ... -> Result<T, Error>`                |
| `void`                              | `()` or `-> Result<(), Error>`                    |
| `any`                               | `serde_json::Value`                               |
| `unknown`                           | `serde_json::Value` or generic `T`                |
| `Buffer` / `Uint8Array`             | `Vec<u8>` or `&[u8]`                              |
| `interface` (data only)             | `pub struct` with `#[derive(Debug, Clone, Serialize, Deserialize)]` |
| `interface` (has methods)           | `pub trait` + impl structs                        |
| `type A = B \| C` (union)           | `pub enum` with variants                          |
| `enum` (string enum)                | `pub enum` with `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = ...)]` |
| `enum` (numeric)                    | `pub enum` with `#[repr(u32)]` or similar         |
| `class`                             | `pub struct` + `impl` block                       |
| `extends` (class)                   | Composition or trait + blanket impl               |
| `implements` (interface)            | `impl Trait for Struct`                           |
| `export`                            | `pub`                                             |
| `private`                           | no `pub` (private by default)                     |
| `readonly`                          | No `mut` / immutable by default                   |
| `as const`                          | `const` or `static`                               |
| `typeof x`                          | Match on enum variant or trait object              |
| `keyof T`                           | No direct equivalent — use enum of field names     |
| `Partial<T>`                        | Struct with all `Option<T>` fields                |
| `Pick<T, K>` / `Omit<T, K>`        | Separate struct with subset of fields             |

## Idiom Translation Rules

### Error Handling
```
TypeScript: throw new Error("msg")       → Rust: return Err(Error::msg("msg"))
TypeScript: try { } catch (e) { }        → Rust: match result { Ok(v) => ..., Err(e) => ... }  OR  .map_err()?
TypeScript: Promise.reject(e)            → Rust: Err(e)
TypeScript: x?.method()                  → Rust: x.as_ref().map(|v| v.method())  OR  if let Some(x) = x { x.method() }
TypeScript: x ?? default                 → Rust: x.unwrap_or(default)  OR  x.unwrap_or_else(|| default)
```

### Async Patterns
```
TypeScript: async function foo()         → Rust: async fn foo() -> Result<T, Error>
TypeScript: await promise                → Rust: future.await?
TypeScript: Promise.all([a, b])          → Rust: tokio::try_join!(a, b)  OR  futures::try_join!(a, b)
TypeScript: setTimeout(fn, ms)          → Rust: tokio::time::sleep(Duration::from_millis(ms)).await
TypeScript: new EventEmitter()           → Rust: tokio::sync::broadcast::channel()  OR custom event system with callbacks
```

### Class → Struct Translation
```typescript
// TypeScript
class Foo {
  private bar: string;
  constructor(bar: string) { this.bar = bar; }
  async doThing(): Promise<Result> { ... }
}
```
```rust
// Rust
pub struct Foo {
    bar: String,
}

impl Foo {
    pub fn new(bar: String) -> Self {
        Self { bar }
    }

    pub async fn do_thing(&self) -> Result<MyResult, Error> {
        // ...
    }
}
```

### Event Emitter → Callback/Channel System
```typescript
// TypeScript
this.emit("connect", data);
this.on("connect", (data) => { ... });
```
```rust
// Rust — use a typed event enum + broadcast channel
pub enum SdkEvent {
    Connect(ConnectData),
    Disconnect(DisconnectData),
    Error(SdkError),
}

// In struct:
event_tx: broadcast::Sender<SdkEvent>,

// Emit:
let _ = self.event_tx.send(SdkEvent::Connect(data));

// Subscribe:
let mut rx = event_tx.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event {
            SdkEvent::Connect(data) => { /* handle */ }
            _ => {}
        }
    }
});
```

### Method Overloading → Enum Parameter or Separate Methods
```typescript
// TypeScript
stamp(params: { type?: "PKI" }): Promise<string>;
stamp(params: { type: "OIDC"; idToken: string }): Promise<string>;
```
```rust
// Rust
pub enum StampParams {
    Pki { data: Vec<u8> },
    Oidc { data: Vec<u8>, id_token: String, salt: String },
}

pub async fn stamp(&self, params: StampParams) -> Result<String, Error> { ... }
```

---

## Crate Dependency Mapping (Cargo.toml)

| NPM Package                    | Rust Crate                                         |
|---------------------------------|----------------------------------------------------|
| `axios`                         | `reqwest` (with `rustls-tls` feature)              |
| `tweetnacl`                     | `ed25519-dalek` + `rand`                           |
| `bs58`                          | `bs58`                                             |
| `buffer`                        | native `Vec<u8>` / `bytes` crate                   |
| `eventemitter3`                 | `tokio::sync::broadcast` or custom                 |
| `@solana/web3.js`               | `solana-sdk` + `solana-client`                     |
| `ethers`                        | `ethers` (ethers-rs) or `alloy`                    |
| `uuid`                          | `uuid`                                             |
| `jest`                          | built-in `#[test]` + `tokio::test`                 |
| `@modelcontextprotocol/sdk`     | `mcp-sdk` or `rmcp`                               |
| `express` / HTTP server         | `axum` or `actix-web`                              |

---

## Naming Conventions

| TypeScript Convention            | Rust Convention                   |
|----------------------------------|-----------------------------------|
| `camelCase` functions/variables  | `snake_case`                      |
| `PascalCase` classes/interfaces  | `PascalCase` structs/enums/traits |
| `UPPER_CASE` constants           | `UPPER_CASE` constants            |
| `I` prefix on interfaces         | Drop the `I` prefix               |
| `.ts` / `.tsx` extension         | `.rs` extension                   |
| `index.ts` barrel exports        | `mod.rs` or `lib.rs` with `pub use` re-exports |
| `foo.test.ts`                    | `#[cfg(test)] mod tests` inside `foo.rs` OR `tests/foo.rs` |

---

## Quality Requirements

1. **Every public function, type, constant, and trait must have a `///` doc comment** explaining what it does, mirroring the original TypeScript JSDoc or inline comments.

2. **All structs that cross serialization boundaries** (API requests/responses, storage, config) must derive `Serialize` and `Deserialize` from serde.

3. **Use `thiserror`** for all custom error types. Mirror the original error hierarchy.

4. **Use `tracing`** instead of `console.log` / `debug()` calls.

5. **No `unwrap()` or `expect()` in library code.** Use `?` propagation. `unwrap()` is only acceptable in tests and examples.

6. **Feature flags** for optional functionality:
   - `wasm` — for browser/WASM targets (IndexedDB, browser APIs)
   - `server` — for server-only features (file system storage, MCP server)
   - `solana` — for Solana chain support
   - `ethereum` — for Ethereum chain support

7. **The Rust code must compile.** After each file, run `cargo check`. After each phase, run `cargo test`.

---

## Files to SKIP (no Rust equivalent needed)

These file types have no meaningful Rust translation. Do not convert them:

- `*.css` — stylesheets
- `*.html` — markup
- `*.json` (package.json, tsconfig.json, app.json, etc.) — JS tooling config
- `*.js` config files (jest.config.js, babel.config.js, metro.config.js, vite.config.ts, tsup.config.ts, .eslintrc.js, .prettierrc.js)
- `*.d.ts` type declaration files (vite-env.d.ts, global.d.ts, sdk-version.d.ts, jest.d.ts, expo.d.ts) — these inform the type mappings but produce no runtime code
- `.changeset/`, `.github/`, `.yarnrc.yml` — CI/tooling
- All `*.tsx` files under `examples/` that are React apps (browser-sdk-demo-app, react-sdk-demo-app, react-native-sdk-demo-app, with-modal, with-nextjs, with-wagmi)
- All `*.tsx` files under `packages/react-sdk/` and `packages/react-native-sdk/` — React component code
- `packages/ui/` component files (`*.native.tsx`, `*.web.tsx`) — React UI components
- Test mock files (`__mocks__/`, `test/mocks/`)
- `sharedJestConfig.js`, root `jest.config.js`

---

## Output Format Per File

For each file you convert, produce exactly this output structure:

```
═══════════════════════════════════════════════════════════════
FILE [N/149]: packages/foo/src/bar.ts → rust/crates/foo/src/bar.rs
═══════════════════════════════════════════════════════════════

## 1. Source Analysis

- Exports: [list of exported symbols]
- Imports: [list of imports with sources]
- Patterns: [async, events, generics, etc.]
- External deps: [npm packages used]

## 2. Rust Source

```rust
// Complete file contents here
```

## 3. Verification Diff

| # | TypeScript Symbol | Rust Symbol | Logic Match | Notes |
|---|---|---|---|---|
| ... | ... | ... | ... | ... |

## 4. Compile Status

- [ ] `cargo check` passes
- [ ] All public symbols documented
- [ ] No `unwrap()` in library code
- [ ] serde derives on serializable types
- [ ] Error types use `thiserror`

═══════════════════════════════════════════════════════════════
```

---

## Critical Rules

1. **ONE file at a time.** Never batch multiple files into a single response.
2. **Read the TypeScript source FIRST.** Do not guess at implementations.
3. **Preserve ALL business logic.** If the TypeScript checks `x > 5`, the Rust must check `x > 5`. If it retries 3 times, the Rust retries 3 times.
4. **The diff table is mandatory.** It is your proof that nothing was lost.
5. **If you are unsure about a mapping, flag it** in the Notes column as `NEEDS_REVIEW` — do not silently make assumptions.
6. **Do not add functionality.** Do not "improve" the logic. Translate it faithfully.
7. **Do not remove functionality.** If the TS has a code path, the Rust must have it.
8. **Platform-specific code** (browser APIs like `window`, `document`, `localStorage`, `IndexedDB`) should be behind `#[cfg(target_arch = "wasm32")]` or feature flags, with trait-based abstractions for cross-platform use.
9. **When the original uses `any`**, prefer `serde_json::Value` unless the actual type can be inferred from usage context — then use that concrete type.
10. **Maintain the same module visibility.** If something is internal/private in TS, keep it private in Rust. If it's exported, make it `pub`.

---

## Getting Started

Begin with file **#1**: `packages/constants/src/network-ids.ts`

Read the file, convert it, produce the diff, and confirm it compiles. Then proceed to file #2. Continue until all 149 files are converted.
