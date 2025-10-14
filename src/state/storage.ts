import { GraphState } from "./graphReducer";

const STORAGE_KEY = "cybernohut-graph-state";
const VERSION = "1.0";

import { Node, Edge } from "@xyflow/react";

export interface StoredGraphState {
  version: string;
  timestamp: string;
  nodes: Node[];
  edges: Edge[];
  selectedInputId: string;
  input: string;
  selectedNodeId: string;
  selectedNodeTitle: string;
}

/**
 * Save graph state to localStorage
 * Excludes transient state like dirtyNodes, isCalculating, errors, output
 */
export function saveGraphToLocalStorage(state: GraphState): void {
  try {
    const storedState: StoredGraphState = {
      version: VERSION,
      timestamp: new Date().toISOString(),
      nodes: state.nodes,
      edges: state.edges,
      selectedInputId: state.selectedInputId,
      input: state.input,
      selectedNodeId: state.selectedNodeId,
      selectedNodeTitle: state.selectedNodeTitle,
    };

    localStorage.setItem(STORAGE_KEY, JSON.stringify(storedState));
  } catch (error) {
    console.warn("Failed to save graph state to localStorage:", error);
  }
}

/**
 * Load graph state from localStorage
 * Returns null if no valid data found
 */
export function loadGraphFromLocalStorage(): Partial<GraphState> | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return null;

    const parsed: StoredGraphState = JSON.parse(stored);

    // Validate structure
    if (!parsed.version || !parsed.nodes || !parsed.edges) {
      console.warn("Invalid stored graph state structure");
      return null;
    }

    // For now, accept any version (future: add migration logic)
    return {
      nodes: parsed.nodes,
      edges: parsed.edges,
      selectedInputId: parsed.selectedInputId || "input-node",
      input: parsed.input || "",
      selectedNodeId: parsed.selectedNodeId || "",
      selectedNodeTitle: parsed.selectedNodeTitle || "",
    };
  } catch (error) {
    console.warn("Failed to load graph state from localStorage:", error);
    return null;
  }
}

/**
 * Clear stored graph state from localStorage
 */
export function clearStoredGraphState(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    console.warn("Failed to clear stored graph state:", error);
  }
}
