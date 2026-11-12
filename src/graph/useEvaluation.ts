import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Edge } from "@xyflow/react";
import { evaluate, type NodeReport } from "@/engine/wasm";
import { toEngineGraph, type GraphNode, type NodeData } from "./model";

const DEBOUNCE_MS = 100;

/**
 * A cheap fingerprint of everything the engine cares about. Node data objects are
 * replaced only when their content changes, so their identity stands in for their
 * content; moving a node keeps the fingerprint, and the graph is not re-evaluated.
 */
function useFingerprint(nodes: GraphNode[], edges: Edge[]): string {
  const ids = useRef(new WeakMap<NodeData, number>());
  const next = useRef(0);
  return useMemo(() => {
    const dataId = (data: NodeData) => {
      let id = ids.current.get(data);
      if (id === undefined) {
        id = next.current++;
        ids.current.set(data, id);
      }
      return id;
    };
    const nodePart = nodes.map((node) => `${node.id}=${dataId(node.data)}`).join(",");
    const edgePart = edges.map((edge) => edge.id).join(",");
    return `${nodePart}|${edgePart}`;
  }, [nodes, edges]);
}

export function useEvaluation(nodes: GraphNode[], edges: Edge[], auto: boolean) {
  const [reports, setReports] = useState<ReadonlyMap<string, NodeReport>>(new Map());
  const fingerprint = useFingerprint(nodes, edges);
  const latest = useRef({ nodes, edges });
  latest.current = { nodes, edges };

  const run = useCallback(() => {
    const { nodes, edges } = latest.current;
    setReports(evaluate(toEngineGraph(nodes, edges)));
  }, []);

  useEffect(() => {
    if (!auto) return;
    const timer = setTimeout(run, DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [fingerprint, auto, run]);

  return { reports, run };
}
