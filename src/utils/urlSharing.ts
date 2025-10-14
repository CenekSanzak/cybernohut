import LZString from "lz-string";
import { GraphState } from "@/state/graphReducer";

/**
 * Encode graph state to URL parameter
 * Compresses the data to handle large graphs
 */
export function encodeGraphToURL(state: GraphState): string {
  try {
    // Create a clean state object for sharing (exclude transient state)
    const shareableState = {
      version: "1.0",
      timestamp: new Date().toISOString(),
      nodes: state.nodes,
      edges: state.edges,
      selectedInputId: state.selectedInputId,
      input: state.input,
      selectedNodeId: state.selectedNodeId,
      selectedNodeTitle: state.selectedNodeTitle,
    };

    const jsonString = JSON.stringify(shareableState);
    const compressed = LZString.compressToEncodedURIComponent(jsonString);

    return compressed;
  } catch (error) {
    console.error("Failed to encode graph to URL:", error);
    throw new Error("Failed to create shareable URL");
  }
}

/**
 * Decode graph state from URL parameter
 * Returns null if no valid data found
 */
export function decodeGraphFromURL(): Partial<GraphState> | null {
  try {
    const urlParams = new URLSearchParams(window.location.search);
    const graphParam = urlParams.get("graph");

    if (!graphParam) return null;

    const decompressed = LZString.decompressFromEncodedURIComponent(graphParam);
    if (!decompressed) {
      console.warn("Failed to decompress URL parameter");
      return null;
    }

    const parsed = JSON.parse(decompressed);

    // Validate structure
    if (!parsed.version || !parsed.nodes || !parsed.edges) {
      console.warn("Invalid graph data in URL");
      return null;
    }

    return {
      nodes: parsed.nodes,
      edges: parsed.edges,
      selectedInputId: parsed.selectedInputId || "input-node",
      input: parsed.input || "",
      selectedNodeId: parsed.selectedNodeId || "",
      selectedNodeTitle: parsed.selectedNodeTitle || "",
    };
  } catch (error) {
    console.warn("Failed to decode graph from URL:", error);
    return null;
  }
}

/**
 * Generate a shareable URL with the current graph state
 */
export function generateShareableURL(state: GraphState): string {
  const encoded = encodeGraphToURL(state);
  const baseUrl = window.location.origin + window.location.pathname;
  return `${baseUrl}?graph=${encoded}`;
}

/**
 * Copy text to clipboard with fallback for older browsers
 */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    } else {
      // Fallback for older browsers
      const textArea = document.createElement("textarea");
      textArea.value = text;
      textArea.style.position = "fixed";
      textArea.style.left = "-999999px";
      textArea.style.top = "-999999px";
      document.body.appendChild(textArea);
      textArea.focus();
      textArea.select();

      const success = document.execCommand("copy");
      document.body.removeChild(textArea);
      return success;
    }
  } catch (error) {
    console.error("Failed to copy to clipboard:", error);
    return false;
  }
}
