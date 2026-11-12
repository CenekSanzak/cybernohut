/**
 * The benchmark's pipelines. Each one is described twice: as a chain of engine
 * operations, and as the fastest equivalent the browser offers without the engine.
 * The two must produce identical bytes, which the runner checks.
 */
import type { OpConfigMap, OpId } from "@/engine/wasm";

export type Step = { [K in OpId]: { op: K; config?: OpConfigMap[K] } }[OpId];

export interface BenchCase {
  name: string;
  /** Which prepared input the pipeline starts from. */
  source: "raw" | "base64";
  steps: Step[];
  /** What the native side uses, shown next to its timing. */
  nativeLabel: string;
  native: (input: Uint8Array) => Promise<Uint8Array | string> | Uint8Array | string;
}

// Uint8Array base64/hex helpers are recent additions; older browsers lack them.
type ModernBytes = Uint8Array & { toBase64?: () => string; toHex?: () => string };
type ModernBytesStatic = typeof Uint8Array & { fromBase64?: (text: string) => Uint8Array };

const hasToBase64 = typeof (Uint8Array.prototype as ModernBytes).toBase64 === "function";
const hasFromBase64 = typeof (Uint8Array as ModernBytesStatic).fromBase64 === "function";
const hasToHex = typeof (Uint8Array.prototype as ModernBytes).toHex === "function";

const CHUNK = 0x8000;

function base64Encode(bytes: Uint8Array): string {
  if (hasToBase64) return (bytes as ModernBytes).toBase64!();
  let binary = "";
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode.apply(null, bytes.subarray(i, i + CHUNK) as unknown as number[]);
  }
  return btoa(binary);
}

function base64Decode(text: string): Uint8Array {
  if (hasFromBase64) return (Uint8Array as ModernBytesStatic).fromBase64!(text);
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

const HEX = Array.from({ length: 256 }, (_, byte) => byte.toString(16).padStart(2, "0"));

function hexEncode(bytes: Uint8Array): string {
  if (hasToHex) return (bytes as ModernBytes).toHex!();
  const parts: string[] = [];
  for (let i = 0; i < bytes.length; i += CHUNK) {
    let part = "";
    const end = Math.min(i + CHUNK, bytes.length);
    for (let j = i; j < end; j++) part += HEX[bytes[j]];
    parts.push(part);
  }
  return parts.join("");
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  return hexEncode(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)));
}

const decode = (bytes: Uint8Array) => new TextDecoder("utf-8", { fatal: true }).decode(bytes);
const encode = (text: string) => new TextEncoder().encode(text);

/** Caesar on bytes: the input is ASCII, so this is the fastest honest JS version. */
function caesar(bytes: Uint8Array, shift: number): Uint8Array {
  const out = new Uint8Array(bytes.length);
  for (let i = 0; i < bytes.length; i++) {
    const c = bytes[i];
    if (c >= 65 && c <= 90) out[i] = ((c - 65 + shift) % 26) + 65;
    else if (c >= 97 && c <= 122) out[i] = ((c - 97 + shift) % 26) + 97;
    else out[i] = c;
  }
  return out;
}

export const cases: BenchCase[] = [
  {
    name: "SHA-256",
    source: "raw",
    steps: [{ op: "sha256" }],
    nativeLabel: "crypto.subtle.digest",
    native: sha256Hex,
  },
  {
    name: "Base64 encode",
    source: "raw",
    steps: [{ op: "base64-standard-encode" }],
    nativeLabel: hasToBase64 ? "Uint8Array.toBase64" : "btoa",
    native: base64Encode,
  },
  {
    name: "Base64 decode",
    source: "base64",
    steps: [{ op: "utf8-decode" }, { op: "base64-standard-decode" }],
    nativeLabel: hasFromBase64 ? "TextDecoder + Uint8Array.fromBase64" : "TextDecoder + atob",
    native: (input) => base64Decode(decode(input)),
  },
  {
    name: "Hex encode",
    source: "raw",
    steps: [{ op: "base16-encode" }],
    nativeLabel: hasToHex ? "Uint8Array.toHex" : "lookup-table loop",
    native: hexEncode,
  },
  {
    name: "Caesar shift 3",
    source: "raw",
    steps: [{ op: "utf8-decode" }, { op: "caesar-cipher", config: { shift: 3 } }],
    nativeLabel: "byte loop",
    native: (input) => caesar(input, 3),
  },
  {
    name: "Uppercase, then reverse",
    source: "raw",
    steps: [{ op: "utf8-decode" }, { op: "to-uppercase" }, { op: "reverse" }],
    nativeLabel: "TextDecoder + toUpperCase + spread/reverse/join",
    native: (input) => [...decode(input).toUpperCase()].reverse().join(""),
  },
  {
    name: "Base64 encode → decode → hex → SHA-256",
    source: "raw",
    steps: [
      { op: "base64-standard-encode" },
      { op: "base64-standard-decode" },
      { op: "base16-encode" },
      { op: "sha256" },
    ],
    nativeLabel: "the native calls above, chained",
    native: (input) => sha256Hex(encode(hexEncode(base64Decode(base64Encode(input))))),
  },
];

const LINE = "The quick brown fox jumps over the lazy dog; 0123456789 +/= nohut\n";

/** Deterministic ASCII text, so every pipeline (including the text ones) accepts it. */
export function makeInput(megabytes: number): Uint8Array {
  const line = encode(LINE);
  const bytes = new Uint8Array(Math.round(megabytes * 1024 * 1024));
  for (let offset = 0; offset < bytes.length; offset += line.length) {
    bytes.set(line.subarray(0, Math.min(line.length, bytes.length - offset)), offset);
  }
  return bytes;
}

export function toBase64Bytes(bytes: Uint8Array): Uint8Array {
  return encode(base64Encode(bytes));
}

export function asBytes(result: Uint8Array | string): Uint8Array {
  return typeof result === "string" ? encode(result) : result;
}
