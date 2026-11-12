/**
 * What a graph is, as data. A node is an operation id plus its config and nothing
 * else: no functions and no results, so a graph can be saved, shared and handed to
 * the engine as it is.
 */
import type { Edge, Node, XYPosition } from "@xyflow/react";
import {
  getMeta,
  type Graph,
  type OpConfigMap,
  type OpId,
  type Port,
} from "@/engine/wasm";

export type InputNodeData = { kind: "input"; text: string };

export type OpNodeData = {
  [K in OpId]: {
    kind: "op";
    opId: K;
    config: Required<OpConfigMap[K]>;
    perLine: boolean;
  };
}[OpId];

export type NodeData = InputNodeData | OpNodeData;
export type GraphNode = Node<NodeData, "custom">;

/** A config as the generic, catalog-driven form sees it. */
export type ConfigRecord = Record<string, string | number | boolean>;

export const DEFAULT_INPUT_ID = "input-node";

const INPUT_NODE_OUTPUTS: readonly Port[] = [{ name: "output", type: "text" }];

export function inputPorts(data: NodeData): readonly Port[] {
  return data.kind === "input" ? [] : getMeta(data.opId).inputs;
}

export function outputPorts(data: NodeData): readonly Port[] {
  return data.kind === "input" ? INPUT_NODE_OUTPUTS : getMeta(data.opId).outputs;
}

export function nodeTitle(data: NodeData): string {
  return data.kind === "input" ? "Input" : getMeta(data.opId).name;
}

export function configOf(data: OpNodeData): ConfigRecord {
  return data.config as ConfigRecord;
}

const shortId = (prefix: string) =>
  `${prefix}-${Math.random().toString(36).slice(2, 7)}`;

export function createInputNode(
  position: XYPosition,
  id = shortId("input")
): GraphNode {
  return { id, type: "custom", position, data: { kind: "input", text: "" } };
}

export function createOpNode<K extends OpId>(
  opId: K,
  position: XYPosition
): GraphNode {
  const data = {
    kind: "op",
    opId,
    config: { ...getMeta(opId).default_config },
    perLine: false,
  } as OpNodeData;
  return { id: shortId(opId), type: "custom", position, data };
}

export function createEdge(
  source: string,
  sourcePort: string,
  target: string,
  targetPort: string
): Edge {
  return {
    id: `${source}:${sourcePort}->${target}:${targetPort}`,
    source,
    sourceHandle: sourcePort,
    target,
    targetHandle: targetPort,
  };
}

/** The graph as the engine wants it: positions and selection dropped. */
export function toEngineGraph(nodes: GraphNode[], edges: Edge[]): Graph {
  return {
    nodes: nodes.map(({ id, data }) =>
      data.kind === "input"
        ? { kind: "input", id, text: data.text }
        : {
            kind: "op",
            id,
            op_id: data.opId,
            config: configOf(data),
            per_line: data.perLine,
          }
    ),
    edges: edges.flatMap((edge) =>
      edge.sourceHandle && edge.targetHandle
        ? [
            {
              source: edge.source,
              source_port: edge.sourceHandle,
              target: edge.target,
              target_port: edge.targetHandle,
            },
          ]
        : []
    ),
  };
}
