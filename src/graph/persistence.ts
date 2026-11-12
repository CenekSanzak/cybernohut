/**
 * Saving and loading graphs: localStorage, exported files and share links all use the
 * same format. Anything loaded is untrusted, so it is rebuilt field by field against
 * the catalog rather than cast.
 */
import type { Edge } from "@xyflow/react";
import LZString from "lz-string";
import { getMeta, isOpId } from "@/engine/wasm";
import {
  createEdge,
  inputPorts,
  outputPorts,
  type ConfigRecord,
  type GraphNode,
  type NodeData,
} from "./model";

const VERSION = 2;
const STORAGE_KEY = "cybernohut-graph-state";

export interface GraphSnapshot {
  nodes: GraphNode[];
  edges: Edge[];
  selectedInputId: string;
  selectedNodeId: string;
}

type Json = Record<string, unknown>;

const isObject = (value: unknown): value is Json =>
  typeof value === "object" && value !== null;

const asString = (value: unknown, fallback = ""): string =>
  typeof value === "string" ? value : fallback;

export function serialize(snapshot: GraphSnapshot): string {
  return JSON.stringify({
    version: VERSION,
    nodes: snapshot.nodes.map(({ id, position, data }) => ({ id, position, data })),
    edges: snapshot.edges.map(({ source, sourceHandle, target, targetHandle }) => ({
      source,
      sourceHandle,
      target,
      targetHandle,
    })),
    selectedInputId: snapshot.selectedInputId,
    selectedNodeId: snapshot.selectedNodeId,
  });
}

/** Keeps the saved values whose key and type match the operation's defaults. */
function readConfig(defaults: ConfigRecord, saved: unknown): ConfigRecord {
  const config = { ...defaults };
  if (isObject(saved)) {
    for (const key of Object.keys(defaults)) {
      if (typeof saved[key] === typeof defaults[key]) {
        config[key] = saved[key] as ConfigRecord[string];
      }
    }
  }
  return config;
}

function readOpData(opId: unknown, config: unknown, perLine: unknown): NodeData | null {
  if (!isOpId(opId)) return null;
  const meta = getMeta(opId);
  return {
    kind: "op",
    opId,
    config: readConfig(meta.default_config as ConfigRecord, config),
    perLine: perLine === true && meta.supports_per_line,
  } as NodeData;
}

function readData(data: unknown): NodeData | null {
  if (!isObject(data)) return null;
  if (data.kind === "input") return { kind: "input", text: asString(data.text) };
  // File content is never saved, so a loaded file node starts out empty.
  if (data.kind === "file") return { kind: "file", name: "", size: 0 };
  if (data.kind === "op") return readOpData(data.opId, data.config, data.perLine);
  // Version 1 stored the whole operation object, functions and all, on the node.
  if (data.id === "input") return { kind: "input", text: asString(data.value) };
  return readOpData(data.id, data.configValues, false);
}

function readNode(node: unknown): GraphNode | null {
  if (!isObject(node) || typeof node.id !== "string") return null;
  const data = readData(node.data);
  if (!data) return null;
  const position = isObject(node.position) ? node.position : {};
  return {
    id: node.id,
    type: "custom",
    position: {
      x: typeof position.x === "number" ? position.x : 0,
      y: typeof position.y === "number" ? position.y : 0,
    },
    data,
  };
}

function readEdge(edge: unknown, nodes: Map<string, GraphNode>): Edge | null {
  if (!isObject(edge)) return null;
  const source = nodes.get(asString(edge.source));
  const target = nodes.get(asString(edge.target));
  const sourcePort = asString(edge.sourceHandle);
  const targetPort = asString(edge.targetHandle);
  if (!source || !target) return null;
  if (!outputPorts(source.data).some((port) => port.name === sourcePort)) return null;
  if (!inputPorts(target.data).some((port) => port.name === targetPort)) return null;
  return createEdge(source.id, sourcePort, target.id, targetPort);
}

/** Returns null when the text is not a graph at all. */
export function deserialize(text: string): GraphSnapshot | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(parsed) || !Array.isArray(parsed.nodes) || !Array.isArray(parsed.edges)) {
    return null;
  }

  const nodes = parsed.nodes.flatMap((node) => readNode(node) ?? []);
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const edges = parsed.edges.flatMap((edge) => readEdge(edge, byId) ?? []);
  const firstInput = nodes.find((node) => node.data.kind === "input")?.id ?? "";
  const selectedInputId = asString(parsed.selectedInputId);
  const selectedNodeId = asString(parsed.selectedNodeId);

  return {
    nodes,
    edges,
    selectedInputId:
      byId.get(selectedInputId)?.data.kind === "input" ? selectedInputId : firstInput,
    selectedNodeId: byId.has(selectedNodeId) ? selectedNodeId : "",
  };
}

export function saveToStorage(snapshot: GraphSnapshot): void {
  try {
    localStorage.setItem(STORAGE_KEY, serialize(snapshot));
  } catch (error) {
    console.warn("Failed to save the graph:", error);
  }
}

export function loadFromStorage(): GraphSnapshot | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? deserialize(stored) : null;
  } catch (error) {
    console.warn("Failed to load the saved graph:", error);
    return null;
  }
}

export function clearStorage(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    console.warn("Failed to clear the saved graph:", error);
  }
}

export function toShareUrl(snapshot: GraphSnapshot): string {
  const encoded = LZString.compressToEncodedURIComponent(serialize(snapshot));
  return `${window.location.origin}${window.location.pathname}?graph=${encoded}`;
}

export function loadFromUrl(): GraphSnapshot | null {
  const encoded = new URLSearchParams(window.location.search).get("graph");
  if (!encoded) return null;
  const text = LZString.decompressFromEncodedURIComponent(encoded);
  return text ? deserialize(text) : null;
}
