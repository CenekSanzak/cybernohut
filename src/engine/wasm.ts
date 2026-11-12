/**
 * Typed access to the Rust engine.
 *
 * Every type here is generated from the Rust source by wasm-bindgen and tsify; nothing
 * about an operation is declared in TypeScript.
 */
import * as wasm from "wasm";
import type {
  Graph,
  NodeReport,
  OpConfigMap,
  OpId,
  OpMeta,
  ValueType,
} from "wasm";

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

/** How many characters of each output the engine returns as a preview. */
const PREVIEW_CHARS = 60;

/**
 * Evaluates the whole graph. A node that fails is reported, not thrown; this throws
 * only if the graph description itself is malformed.
 */
export function evaluate(graph: Graph): Map<string, NodeReport> {
  const reports = wasm.evaluate(graph, PREVIEW_CHARS);
  return new Map(reports.map((report) => [report.id, report]));
}

/** The full output of a node, from the most recent {@link evaluate} call. */
export function outputText(nodeId: string, port: string): string | undefined {
  return wasm.output_text(nodeId, port);
}
