import type { Dispatch } from "react";
import { engine } from "@/engine/client";
import type { GraphAction } from "@/state/graphReducer";
import { showNotification } from "@/utils/notifications";

/**
 * The engine is 32-bit wasm, so everything it holds must fit in 4 GB: the file, a copy
 * of it on the page, and every intermediate result, some of which (hex, Base64) are
 * larger than their input. 256 MB leaves room for a few such steps.
 */
const MAX_FILE_BYTES = 256 * 1024 ** 2;

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${unit === 0 ? value : value.toFixed(1)} ${units[unit]}`;
}

/** Reads a file into the engine and records its name and size on the node. */
export async function loadFile(
  nodeId: string,
  file: File,
  dispatch: Dispatch<GraphAction>
): Promise<void> {
  if (file.size > MAX_FILE_BYTES) {
    showNotification(
      `${file.name} is ${formatBytes(file.size)}; the limit is ${formatBytes(MAX_FILE_BYTES)}`,
      "error"
    );
    return;
  }
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    // The engine must have the bytes before the node change triggers a new run.
    engine.setSource(nodeId, bytes);
    dispatch({ type: "SET_FILE", nodeId, name: file.name, size: file.size });
  } catch (error) {
    console.error("Failed to read file:", error);
    showNotification(`Could not read ${file.name}`, "error");
  }
}

/** Saves bytes as a file through the browser's download mechanism. */
export function saveBytes(bytes: Uint8Array, filename: string): void {
  const url = URL.createObjectURL(new Blob([bytes]));
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
