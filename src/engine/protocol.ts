import type { Graph, NodeReport, OutputText } from "wasm";

/** Messages from the page to the engine worker. */
export type EngineRequest =
  | { type: "evaluate"; id: number; graph: Graph }
  | { type: "output"; id: number; nodeId: string; port: string }
  | { type: "bytes"; id: number; nodeId: string; port: string };

/** One-way messages that manage the content of file nodes. They get no response. */
export type EngineNotice =
  | { type: "set_source"; nodeId: string; bytes: Uint8Array }
  | { type: "retain_sources"; nodeIds: string[] };

/**
 * Sent once by the worker when it can take requests. Its wasm module loads
 * asynchronously, and a message that arrives before the handler is installed is lost.
 */
export type EngineReady = { type: "ready" };

/** Messages from the engine worker back to the page. `id` echoes the request. */
export type EngineResponse =
  | { type: "evaluated"; id: number; reports: NodeReport[]; elapsedMs: number }
  | { type: "output"; id: number; output: OutputText | undefined }
  | { type: "bytes"; id: number; bytes: Uint8Array | undefined }
  | { type: "failed"; id: number; message: string };
