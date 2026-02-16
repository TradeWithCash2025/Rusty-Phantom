#!/usr/bin/env node

/**
 * Fixture Generator for Differential Testing
 *
 * Runs TS functions on known inputs and writes JSON fixtures to fixtures/.
 * These fixtures serve as golden test vectors for both TS and Rust.
 *
 * Usage: node tools/generate_fixtures.mjs
 * Prerequisites: yarn install && yarn build:packages
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Buffer } from "node:buffer";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");
const FIXTURES = join(ROOT, "fixtures");

mkdirSync(FIXTURES, { recursive: true });

// Import built packages
const { base64urlEncode, base64urlDecode, base64urlDecodeToString, stringToBase64url } =
  await import(join(ROOT, "packages/base64url/dist/index.mjs"));
const { generateKeyPair, createKeyPairFromSecret, signWithSecret } =
  await import(join(ROOT, "packages/crypto/dist/index.mjs"));
const {
  getNetworkConfig, getExplorerUrl, getSupportedNetworks,
  getNetworksByChain, chainIdToNetworkId, networkIdToChainId,
  getProviderName, NetworkId,
} = await import(join(ROOT, "packages/constants/dist/index.mjs"));
const { getChainPrefix, isEthereumChain, isSolanaChain } =
  await import(join(ROOT, "packages/utils/dist/index.mjs"));
const { ApiKeyStamper } =
  await import(join(ROOT, "packages/api-key-stamper/dist/index.mjs"));

function writeFixture(name, data) {
  const path = join(FIXTURES, name);
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
  console.log(`  wrote ${name} (${data.length} vectors)`);
}

console.log("Generating fixtures...\n");

// --- base64url ---

const base64urlFixtures = [];

const stringCases = [
  ["empty string", ""],
  ["Hello World", "Hello World"],
  ["special chars", "Special chars: !@#$%^&*()"],
  ["JSON payload", '{"key":"value","nested":{"a":1}}'],
  ["multiline", "Line1\nLine2\tTabbed"],
  ["unicode emoji", "Hello \u{1f30d} World"],
  ["JWT-like", '{"sub":"1234567890","name":"John Doe","iat":1516239022}'],
  ["Phantom string", "Hello from Phantom!"],
  ["URL with params", "https://example.com/path?a=1&b=2"],
];

for (const [desc, input] of stringCases) {
  const encoded = stringToBase64url(input);
  const decoded = base64urlDecodeToString(encoded);
  base64urlFixtures.push({
    description: `stringToBase64url: ${desc}`,
    fn: "base64url.stringToBase64url",
    args: [input],
    expected_encoded: encoded,
    expected_decoded: decoded,
  });
}

const byteCases = [
  ["empty bytes", []],
  ["Hello bytes", [72, 101, 108, 108, 111]],
  ["single byte zero", [0]],
  ["single byte 255", [255]],
  ["boundary bytes", [62, 63, 64]],
  ["sequential 0-255", Array.from({ length: 256 }, (_, i) => i)],
  ["all zeros 16", new Array(16).fill(0)],
  ["all 0xFF 16", new Array(16).fill(255)],
];

for (const [desc, input] of byteCases) {
  const encoded = base64urlEncode(new Uint8Array(input));
  const decoded = Array.from(base64urlDecode(encoded));
  base64urlFixtures.push({
    description: `base64urlEncode bytes: ${desc}`,
    fn: "base64url.base64urlEncode",
    args: [{ __bytes__: input }],
    expected_encoded: encoded,
    expected_decoded_bytes: decoded,
  });
}

writeFixture("base64url.json", base64urlFixtures);

// --- crypto ---

// Generate a valid keypair and use it for all crypto/stamper fixtures
const generatedKp = generateKeyPair();
const knownSecretB58 = generatedKp.secretKey;
console.log(`  Using generated keypair (publicKey: ${generatedKp.publicKey.slice(0, 12)}...)`);

const cryptoFixtures = [];

const kp = createKeyPairFromSecret(knownSecretB58);
cryptoFixtures.push({
  description: "createKeyPairFromSecret with known key",
  fn: "crypto.createKeyPairFromSecret",
  args: [knownSecretB58],
  expected: { publicKey: kp.publicKey, secretKey: kp.secretKey },
});

// Sign several messages with the known key
const messages = [
  ["empty message", []],
  ["Hello World", Array.from(new TextEncoder().encode("Hello, World!"))],
  ["single byte", [42]],
  ["256 bytes", Array.from({ length: 256 }, (_, i) => i % 256)],
];

for (const [desc, dataBytes] of messages) {
  const data = new Uint8Array(dataBytes);
  const sig = signWithSecret(knownSecretB58, data);
  cryptoFixtures.push({
    description: `signWithSecret: ${desc}`,
    fn: "crypto.signWithSecret",
    args: [knownSecretB58, { __bytes__: dataBytes }],
    expected_signature: Array.from(sig),
  });
}

writeFixture("crypto.json", cryptoFixtures);

// --- constants ---

const constantsFixtures = [];

// NetworkId enum values - get all by checking the enum object
const networkIds = Object.values(NetworkId).filter((v) => typeof v === "string");

for (const nid of networkIds) {
  const config = getNetworkConfig(nid);
  if (config) {
    constantsFixtures.push({
      description: `getNetworkConfig: ${nid}`,
      fn: "constants.getNetworkConfig",
      args: [nid],
      expected: config,
    });

    // Explorer URLs
    if (config.explorer) {
      for (const urlType of ["transaction", "address"]) {
        const testValue = urlType === "transaction" ? "abc123txhash" : "abc123address";
        const url = getExplorerUrl(nid, urlType, testValue);
        constantsFixtures.push({
          description: `getExplorerUrl: ${nid} ${urlType}`,
          fn: "constants.getExplorerUrl",
          args: [nid, urlType, testValue],
          expected: url,
        });
      }
    }
  }
}

// getSupportedNetworks
const supported = getSupportedNetworks();
constantsFixtures.push({
  description: "getSupportedNetworks",
  fn: "constants.getSupportedNetworks",
  args: [],
  expected: supported,
});

// getNetworksByChain
for (const chain of ["solana", "ethereum", "polygon", "base", "arbitrum", "monad", "bitcoin", "sui", "nonexistent"]) {
  const networks = getNetworksByChain(chain);
  constantsFixtures.push({
    description: `getNetworksByChain: ${chain}`,
    fn: "constants.getNetworksByChain",
    args: [chain],
    expected: networks,
  });
}

// chainIdToNetworkId
for (const chainId of [1, 11155111, 137, 80002, 8453, 84532, 42161, 421614, 143, 10143, 999999]) {
  const result = chainIdToNetworkId(chainId);
  constantsFixtures.push({
    description: `chainIdToNetworkId: ${chainId}`,
    fn: "constants.chainIdToNetworkId",
    args: [chainId],
    expected: result ?? null,
  });
}

// networkIdToChainId
for (const nid of networkIds) {
  const result = networkIdToChainId(nid);
  constantsFixtures.push({
    description: `networkIdToChainId: ${nid}`,
    fn: "constants.networkIdToChainId",
    args: [nid],
    expected: result ?? null,
  });
}

// getProviderName
for (const provider of ["google", "apple", "phantom", "device", "injected", "deeplink", "unknown_provider"]) {
  constantsFixtures.push({
    description: `getProviderName: ${provider}`,
    fn: "constants.getProviderName",
    args: [provider],
    expected: getProviderName(provider),
  });
}

writeFixture("constants.json", constantsFixtures);

// --- api_key_stamper ---

const stamperFixtures = [];

const stamper = new ApiKeyStamper({ apiSecretKey: knownSecretB58 });

const stampMessages = [
  ["simple message", Array.from(new TextEncoder().encode("test-stamp-data"))],
  ["JSON data", Array.from(new TextEncoder().encode('{"action":"sign","wallet":"abc"}'))],
];

for (const [desc, dataBytes] of stampMessages) {
  const stamp = await stamper.stamp({ data: Buffer.from(dataBytes) });
  stamperFixtures.push({
    description: `ApiKeyStamper.stamp PKI: ${desc}`,
    fn: "apiKeyStamper.stamp",
    args: [knownSecretB58, { __bytes__: dataBytes }],
    expected: stamp,
  });
}

writeFixture("api_key_stamper.json", stamperFixtures);

console.log("\nDone!");
