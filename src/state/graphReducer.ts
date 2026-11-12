import {
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type EdgeChange,
  type NodeChange,
  type XYPosition,
} from "@xyflow/react";
import { canConnect, type OpId } from "@/engine/wasm";
import {
  DEFAULT_INPUT_ID,
  createEdge,
  createFileNode,
  createInputNode,
  createOpNode,
  inputPorts,
  outputPorts,
  type ConfigRecord,
  type GraphNode,
  type NodeData,
} from "@/graph/model";
import type { GraphSnapshot } from "@/graph/persistence";

export type GraphState = GraphSnapshot;

export type GraphAction =
  | { type: "ADD_INPUT" }
  | { type: "ADD_FILE" }
  | { type: "SET_FILE"; nodeId: string; name: string; size: number }
  | { type: "ADD_OP"; opId: OpId }
  | { type: "ADD_CHAIN"; anchorId: string; steps: ChainStep[] }
  | { type: "CONNECT"; connection: Connection }
  | { type: "SET_INPUT_TEXT"; nodeId: string; text: string }
  | { type: "SET_CONFIG"; nodeId: string; key: string; value: string | number | boolean }
  | { type: "SET_PER_LINE"; nodeId: string; perLine: boolean }
  | { type: "SELECT"; nodeId: string }
  | { type: "APPLY_NODE_CHANGES"; changes: NodeChange<GraphNode>[] }
  | { type: "APPLY_EDGE_CHANGES"; changes: EdgeChange[] }
  | { type: "LOAD"; snapshot: GraphSnapshot };

/** One step of a chain proposed by auto mode. */
export interface ChainStep {
  opId: OpId;
  config: ConfigRecord;
  note: string;
}

export function createInitialState(): GraphState {
  return {
    nodes: [createInputNode({ x: 250, y: 10 }, DEFAULT_INPUT_ID)],
    edges: [],
    selectedInputId: DEFAULT_INPUT_ID,
    selectedNodeId: "",
  };
}

const NODE_GAP = 80;

function below(node: GraphNode, dx = 0): XYPosition {
  return {
    x: node.position.x + dx,
    y: node.position.y + (node.measured?.height ?? 80) + NODE_GAP,
  };
}

function updateData(
  state: GraphState,
  nodeId: string,
  update: (data: NodeData) => NodeData
): GraphState {
  return {
    ...state,
    nodes: state.nodes.map((node) =>
      node.id === nodeId ? { ...node, data: update(node.data) } : node
    ),
  };
}

/**
 * Connects two ports. Bytes cannot feed a text port directly, because decoding can
 * fail; instead of refusing, the decode step is added to the graph where the user can
 * see and configure it.
 */
function connect(state: GraphState, connection: Connection): GraphState {
  const source = state.nodes.find((node) => node.id === connection.source);
  const target = state.nodes.find((node) => node.id === connection.target);
  const from = source && outputPorts(source.data).find((p) => p.name === connection.sourceHandle);
  const to = target && inputPorts(target.data).find((p) => p.name === connection.targetHandle);
  if (!source || !target || !from || !to) return state;

  const occupied = state.edges.some(
    (edge) => edge.target === target.id && edge.targetHandle === to.name
  );
  if (occupied) return state;

  if (canConnect(from.type, to.type)) {
    return {
      ...state,
      edges: [...state.edges, createEdge(source.id, from.name, target.id, to.name)],
    };
  }

  if (from.type === "bytes" && to.type === "text") {
    const decode = createOpNode("utf8-decode", {
      x: (source.position.x + target.position.x) / 2 + 220,
      y: (source.position.y + target.position.y) / 2,
    });
    return {
      ...state,
      nodes: [...state.nodes, decode],
      edges: [
        ...state.edges,
        createEdge(source.id, from.name, decode.id, "input"),
        createEdge(decode.id, "output", target.id, to.name),
      ],
    };
  }

  return state;
}

/** Whether {@link connect} would do anything for this pair of port types. */
export function isConnectable(
  from: NodeData,
  fromPort: string,
  to: NodeData,
  toPort: string
): boolean {
  const source = outputPorts(from).find((port) => port.name === fromPort);
  const target = inputPorts(to).find((port) => port.name === toPort);
  if (!source || !target) return false;
  return (
    canConnect(source.type, target.type) ||
    (source.type === "bytes" && target.type === "text")
  );
}

/** Adds an operation and wires it to the selected node when the types allow it. */
function addOp(state: GraphState, opId: OpId): GraphState {
  const anchor =
    state.nodes.find((node) => node.id === state.selectedNodeId) ??
    state.nodes.find((node) => node.id === state.selectedInputId);
  const node = createOpNode(opId, anchor ? below(anchor) : { x: 250, y: 10 });
  const added = { ...state, nodes: [...state.nodes, node], selectedNodeId: node.id };
  if (!anchor) return added;

  const sourcePort = outputPorts(anchor.data)[0];
  const targetPort = inputPorts(node.data)[0];
  if (!sourcePort || !targetPort) return added;
  return connect(added, {
    source: anchor.id,
    sourceHandle: sourcePort.name,
    target: node.id,
    targetHandle: targetPort.name,
  });
}

/**
 * Adds a proposed chain under the anchor, each step wired to the one before. The steps
 * become ordinary nodes; the note on each records why it was proposed.
 */
function addChain(state: GraphState, anchorId: string, steps: ChainStep[]): GraphState {
  let previous = state.nodes.find((node) => node.id === anchorId);
  if (!previous) return state;
  // Beside the anchor's existing column, so the chain does not land on other nodes.
  const taken = state.edges.filter((edge) => edge.source === anchorId).length;
  let position = below(previous, taken * 280);
  let next = state;

  for (const step of steps) {
    const node = createOpNode(step.opId, position);
    const data = node.data as Extract<NodeData, { kind: "op" }>;
    node.data = { ...data, config: { ...data.config, ...step.config }, note: step.note } as NodeData;
    next = { ...next, nodes: [...next.nodes, node], selectedNodeId: node.id };

    const sourcePort = outputPorts(previous.data)[0];
    const targetPort = inputPorts(node.data)[0];
    if (sourcePort && targetPort) {
      next = connect(next, {
        source: previous.id,
        sourceHandle: sourcePort.name,
        target: node.id,
        targetHandle: targetPort.name,
      });
    }
    previous = node;
    // Room for a node with a note and a setting or two; nodes are measured only later.
    position = { x: position.x, y: position.y + 210 };
  }
  return next;
}

/** Where a new input or file node goes: to the right of the last one. */
function nextSourcePosition(state: GraphState): XYPosition {
  const sources = state.nodes.filter((node) => node.data.kind !== "op");
  const last = sources[sources.length - 1];
  return last ? { x: last.position.x + 260, y: last.position.y } : { x: 250, y: 10 };
}

/** Drops selections that point at nodes which no longer exist. */
function repairSelection(state: GraphState): GraphState {
  const ids = new Set(state.nodes.map((node) => node.id));
  const firstInput = state.nodes.find((node) => node.data.kind === "input")?.id ?? "";
  const selectedInputId = ids.has(state.selectedInputId) ? state.selectedInputId : firstInput;
  const selectedNodeId = ids.has(state.selectedNodeId) ? state.selectedNodeId : "";
  if (selectedInputId === state.selectedInputId && selectedNodeId === state.selectedNodeId) {
    return state;
  }
  return { ...state, selectedInputId, selectedNodeId };
}

export function graphReducer(state: GraphState, action: GraphAction): GraphState {
  switch (action.type) {
    case "ADD_INPUT": {
      const node = createInputNode(nextSourcePosition(state));
      return { ...state, nodes: [...state.nodes, node], selectedInputId: node.id };
    }

    case "ADD_FILE": {
      const node = createFileNode(nextSourcePosition(state));
      return { ...state, nodes: [...state.nodes, node], selectedNodeId: node.id };
    }

    case "SET_FILE":
      return updateData(state, action.nodeId, (data) =>
        data.kind === "file" ? { ...data, name: action.name, size: action.size } : data
      );

    case "ADD_OP":
      return addOp(state, action.opId);

    case "ADD_CHAIN":
      return addChain(state, action.anchorId, action.steps);

    case "CONNECT":
      return connect(state, action.connection);

    case "SET_INPUT_TEXT":
      return updateData(state, action.nodeId, (data) =>
        data.kind === "input" ? { ...data, text: action.text } : data
      );

    case "SET_CONFIG":
      return updateData(state, action.nodeId, (data) =>
        data.kind === "op"
          ? ({ ...data, config: { ...data.config, [action.key]: action.value } } as NodeData)
          : data
      );

    case "SET_PER_LINE":
      return updateData(state, action.nodeId, (data) =>
        data.kind === "op" ? { ...data, perLine: action.perLine } : data
      );

    case "SELECT": {
      const node = state.nodes.find((n) => n.id === action.nodeId);
      if (!node) return { ...state, selectedNodeId: "" };
      return node.data.kind === "input"
        ? { ...state, selectedInputId: node.id }
        : { ...state, selectedNodeId: node.id };
    }

    case "APPLY_NODE_CHANGES":
      return repairSelection({
        ...state,
        nodes: applyNodeChanges(action.changes, state.nodes),
      });

    case "APPLY_EDGE_CHANGES":
      return { ...state, edges: applyEdgeChanges(action.changes, state.edges) };

    case "LOAD":
      return action.snapshot;

    default:
      return state;
  }
}
