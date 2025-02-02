import { Operation } from "@/operations/types";
import {
  Edge,
  Node,
  applyNodeChanges,
  applyEdgeChanges,
  NodeChange,
  EdgeChange,
} from "@xyflow/react";
import {
  getAffectedNodesFromEdgeChanges,
  getDownstreamNodes,
} from "@/nodes/utils";

export type GraphState = {
  nodes: Node[];
  edges: Edge[];
  selectedNodeId: string;
  selectedInputId: string;
  input: string;
  output: string;
  selectedNodeTitle: string;
  dirtyNodes: Set<string>;
};

export type GraphAction =
  | { type: "SET_NODES"; nodes: Node[] }
  | { type: "SET_EDGES"; edges: Edge[] }
  | { type: "SELECT_NODE"; nodeId: string; nodeTitle: string }
  | { type: "SELECT_INPUT"; nodeId: string }
  | { type: "SET_INPUT"; value: string }
  | { type: "SET_OUTPUT"; value: string }
  | { type: "ADD_NODE"; node: Node; edge?: Edge }
  | { type: "UPDATE_NODE_VALUE"; nodeId: string; value: string }
  | { type: "DELETE_NODES"; nodeIds: string[] }
  | { type: "APPLY_NODE_CHANGES"; changes: NodeChange[] }
  | { type: "APPLY_EDGE_CHANGES"; changes: EdgeChange[] }
  | { type: "SET_EDGES_AND_NODES"; edges: Edge[]; nodes: Node[] }
  | { type: "SET_NODES_AND_CLEAR_DIRTY"; nodes: Node[] };

export const initialGraphState: GraphState = {
  nodes: [],
  edges: [],
  selectedNodeId: "",
  selectedInputId: "input-node",
  input: "",
  output: "",
  selectedNodeTitle: "",
  dirtyNodes: new Set<string>(),
};

export const graphReducer = (
  state: GraphState,
  action: GraphAction
): GraphState => {
  switch (action.type) {
    case "SET_NODES":
      return { ...state, nodes: action.nodes };

    case "SET_EDGES": {
      const changedNodes = getAffectedNodesFromEdgeChanges(
        state.edges,
        action.edges
      );
      return {
        ...state,
        edges: action.edges,
        dirtyNodes: new Set([...state.dirtyNodes, ...changedNodes]),
      };
    }

    case "SELECT_NODE":
      return {
        ...state,
        selectedNodeId: action.nodeId,
        selectedNodeTitle: action.nodeTitle || "",
      };

    case "SELECT_INPUT":
      return {
        ...state,
        selectedInputId: action.nodeId,
        input:
          (state.nodes.find((n) => n.id === action.nodeId)?.data as Operation)
            ?.value || "",
      };

    case "SET_INPUT":
      return { ...state, input: action.value };

    case "SET_OUTPUT":
      return { ...state, output: action.value };

    case "ADD_NODE": {
      const newNodeId = action.node.id;
      return {
        ...state,
        nodes: [...state.nodes, action.node],
        edges: action.edge ? [...state.edges, action.edge] : state.edges,
        dirtyNodes: new Set([
          ...state.dirtyNodes,
          newNodeId,
          ...(action.edge ? [action.edge.target] : []),
        ]),
      };
    }

    case "UPDATE_NODE_VALUE": {
      const affectedNodes = getDownstreamNodes(state.edges, action.nodeId);
      return {
        ...state,
        nodes: state.nodes.map((node) =>
          node.id === action.nodeId
            ? {
                ...node,
                data: {
                  ...node.data,
                  value: action.value,
                  outputValues: { output: action.value },
                },
              }
            : node
        ),
        dirtyNodes: new Set([...state.dirtyNodes, ...affectedNodes]),
      };
    }

    case "DELETE_NODES":
      return {
        ...state,
        nodes: state.nodes.filter((node) => !action.nodeIds.includes(node.id)),
      };

    case "APPLY_NODE_CHANGES":
      return {
        ...state,
        nodes: applyNodeChanges(action.changes, state.nodes),
      };

    case "APPLY_EDGE_CHANGES":
      return {
        ...state,
        edges: applyEdgeChanges(action.changes, state.edges),
      };

    case "SET_EDGES_AND_NODES":
      return {
        ...state,
        edges: action.edges,
        nodes: action.nodes,
      };

    case "SET_NODES_AND_CLEAR_DIRTY":
      return {
        ...state,
        nodes: action.nodes,
        dirtyNodes: new Set(),
      };

    default:
      return state;
  }
};
