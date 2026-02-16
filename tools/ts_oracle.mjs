#!/usr/bin/env node

/**
 * TypeScript Oracle for Differential Testing
 *
 * A long-lived Node.js process that reads NDJSON from stdin, dispatches to
 * built TypeScript package functions, and writes NDJSON results to stdout.
 *
 * Protocol:
 *   Request:  {"fn": "base64url.base64urlEncode", "args": [{"__bytes__": [72,101,108]}]}
 *   Response: {"ok": true, "value": "SGVs"} | {"ok": false, "error": "..."}
 *
 * Prerequisites: yarn install && yarn build:packages
 */

import * as readline from "node:readline";
import { Buffer } from "node:buffer";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");

// --- Dynamic imports from built packages ---

const base64urlMod = await import(join(ROOT, "packages/base64url/dist/index.mjs"));
const cryptoMod = await import(join(ROOT, "packages/crypto/dist/index.mjs"));
const constantsMod = await import(join(ROOT, "packages/constants/dist/index.mjs"));
const utilsMod = await import(join(ROOT, "packages/utils/dist/index.mjs"));
const stamperMod = await import(join(ROOT, "packages/api-key-stamper/dist/index.mjs"));

// --- Argument helpers ---

/**
 * Decode a JSON argument into the type expected by TS functions.
 * - {"__bytes__": [1,2,3]} -> Uint8Array
 * - Strings/numbers pass through unchanged
 */
function decodeArg(arg) {
  if (arg && typeof arg === "object" && Array.isArray(arg.__bytes__)) {
    return new Uint8Array(arg.__bytes__);
  }
  return arg;
}

/**
 * Serialize a TS return value into JSON-safe form.
 * - Uint8Array/Buffer -> number array
 * - undefined -> null
 * - NaN -> "__NaN__"
 * - Infinity -> "__Inf__"
 * - -Infinity -> "__NegInf__"
 * - -0 -> 0
 * - Objects -> recursively serialize with sorted keys
 */
function serializeValue(val) {
  if (val === undefined) return null;
  if (val === null) return null;

  if (typeof val === "number") {
    if (Number.isNaN(val)) return "__NaN__";
    if (val === Infinity) return "__Inf__";
    if (val === -Infinity) return "__NegInf__";
    if (Object.is(val, -0)) return 0;
    return val;
  }

  if (typeof val === "string" || typeof val === "boolean") return val;

  if (val instanceof Uint8Array || Buffer.isBuffer(val)) {
    return Array.from(val);
  }

  if (Array.isArray(val)) {
    return val.map(serializeValue);
  }

  if (typeof val === "object") {
    const sorted = {};
    for (const key of Object.keys(val).sort()) {
      sorted[key] = serializeValue(val[key]);
    }
    return sorted;
  }

  return val;
}

// --- Dispatch table ---

const DISPATCH = {
  // base64url
  "base64url.base64urlEncode": (args) => {
    const data = decodeArg(args[0]);
    return base64urlMod.base64urlEncode(data);
  },
  "base64url.base64urlDecode": (args) => {
    const result = base64urlMod.base64urlDecode(args[0]);
    return Array.from(result);
  },
  "base64url.base64urlDecodeToString": (args) => {
    return base64urlMod.base64urlDecodeToString(args[0]);
  },
  "base64url.stringToBase64url": (args) => {
    return base64urlMod.stringToBase64url(args[0]);
  },

  // crypto
  "crypto.createKeyPairFromSecret": (args) => {
    return cryptoMod.createKeyPairFromSecret(args[0]);
  },
  "crypto.signWithSecret": (args) => {
    const secretKey = args[0]; // base58 string
    const data = decodeArg(args[1]);
    const sig = cryptoMod.signWithSecret(secretKey, data);
    return Array.from(sig);
  },

  // constants - networks
  "constants.getNetworkConfig": (args) => {
    return constantsMod.getNetworkConfig(args[0]);
  },
  "constants.getExplorerUrl": (args) => {
    return constantsMod.getExplorerUrl(args[0], args[1], args[2]);
  },
  "constants.getSupportedNetworks": (_args) => {
    return constantsMod.getSupportedNetworks();
  },
  "constants.getNetworksByChain": (args) => {
    return constantsMod.getNetworksByChain(args[0]);
  },
  "constants.chainIdToNetworkId": (args) => {
    return constantsMod.chainIdToNetworkId(args[0]);
  },
  "constants.networkIdToChainId": (args) => {
    return constantsMod.networkIdToChainId(args[0]);
  },

  // constants - provider names
  "constants.getProviderName": (args) => {
    return constantsMod.getProviderName(args[0]);
  },

  // utils - network
  "utils.getChainPrefix": (args) => {
    return utilsMod.getChainPrefix(args[0]);
  },
  "utils.isEthereumChain": (args) => {
    return utilsMod.isEthereumChain(args[0]);
  },
  "utils.isSolanaChain": (args) => {
    return utilsMod.isSolanaChain(args[0]);
  },

  // api-key-stamper
  "apiKeyStamper.stamp": async (args) => {
    const secretKey = args[0]; // base58 string
    const data = decodeArg(args[1]);
    const stamper = new stamperMod.ApiKeyStamper({ apiSecretKey: secretKey });
    const result = await stamper.stamp({ data: Buffer.from(data) });
    return result;
  },
};

// --- Main loop ---

const rl = readline.createInterface({ input: process.stdin, terminal: false });

for await (const line of rl) {
  let req;
  try {
    req = JSON.parse(line);
  } catch (e) {
    process.stdout.write(JSON.stringify({ ok: false, error: `Invalid JSON: ${e.message}` }) + "\n");
    continue;
  }

  const handler = DISPATCH[req.fn];
  if (!handler) {
    process.stdout.write(JSON.stringify({ ok: false, error: `Unknown function: ${req.fn}` }) + "\n");
    continue;
  }

  try {
    const result = await handler(req.args || []);
    process.stdout.write(JSON.stringify({ ok: true, value: serializeValue(result) }) + "\n");
  } catch (err) {
    process.stdout.write(JSON.stringify({ ok: false, error: err.message || String(err) }) + "\n");
  }
}
