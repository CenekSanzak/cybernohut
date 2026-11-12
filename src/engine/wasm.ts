/**
 * The engine's metadata, on the main thread.
 *
 * Every type here is generated from the Rust source by wasm-bindgen and tsify; nothing
 * about an operation is declared in TypeScript. This instance only answers questions
 * about operations and types. Graphs are evaluated by a second instance in a worker
 * (see ./client.ts), so heavy work never blocks the page.
 */
import * as wasm from "wasm";
import type { OpConfigMap, OpId, OpMeta, ValueType } from "wasm";

export type {
  ErrorCode,
  Graph,
  GraphEdge,
  GraphNode as EngineNode,
  NodeReport,
  OpConfigMap,
  OpError,
  OpId,
  OpMeta,
  OutputPreview,
  OutputText,
  ParamKind,
  ParamSpec,
  Port,
  Tag,
  ValueType,
} from "wasm";

/** The catalog entry of one operation, with its config default typed by its id. */
export type OpMetaOf<K extends OpId> = OpMeta & {
  id: K;
  default_config: Required<OpConfigMap[K]>;
};

export const catalog = wasm.catalog() as readonly OpMetaOf<OpId>[];

const metaById = new Map<string, OpMeta>(catalog.map((meta) => [meta.id, meta]));

export function isOpId(id: unknown): id is OpId {
  return typeof id === "string" && metaById.has(id);
}

export function getMeta<K extends OpId>(id: K): OpMetaOf<K> {
  return metaById.get(id) as OpMetaOf<K>;
}

/** Whether an output of type `from` may feed an input of type `to`. */
export function canConnect(from: ValueType, to: ValueType): boolean {
  return wasm.can_connect(from, to);
}
