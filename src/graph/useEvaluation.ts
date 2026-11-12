import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Edge } from "@xyflow/react";
import { engine, Superseded } from "@/engine/client";
import type { NodeReport, OutputText } from "@/engine/wasm";
import { outputPorts, toEngineGraph, type GraphNode, type NodeData } from "./model";

const DEBOUNCE_MS = 100;

export interface Evaluation {
  reports: ReadonlyMap<string, NodeReport>;
  /** Bumped on every finished run; a new value means the engine holds new outputs. */
  version: number;
  running: boolean;
  elapsedMs: number | null;
  /** Set when the engine itself failed, as opposed to a node in the graph. */
  error: string | null;
}

const EMPTY: Evaluation = {
  reports: new Map(),
  version: 0,
  running: false,
  elapsedMs: null,
  error: null,
};

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
  const [evaluation, setEvaluation] = useState(EMPTY);
  const fingerprint = useFingerprint(nodes, edges);
  const latest = useRef({ nodes, edges });
  latest.current = { nodes, edges };

  const run = useCallback(() => {
    const { nodes, edges } = latest.current;
    setEvaluation((previous) => ({ ...previous, running: true }));
    engine.evaluate(toEngineGraph(nodes, edges)).then(
      ({ reports, elapsedMs }) =>
        setEvaluation((previous) => ({
          reports,
          version: previous.version + 1,
          running: false,
          elapsedMs,
          error: null,
        })),
      (error: Error) => {
        // A superseded run has a newer one in flight that will report instead.
        if (error instanceof Superseded) return;
        setEvaluation((previous) => ({
          reports: new Map(),
          version: previous.version + 1,
          running: false,
          elapsedMs: null,
          error: error.message,
        }));
      }
    );
  }, []);

  useEffect(() => {
    if (!auto) return;
    const timer = setTimeout(run, DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [fingerprint, auto, run]);

  return { evaluation, run };
}

/** The displayable output of one node, refetched whenever a run finishes. */
export function useNodeOutput(
  node: GraphNode | undefined,
  evaluation: Evaluation
): OutputText | undefined {
  const [output, setOutput] = useState<OutputText>();
  const nodeId = node?.id;
  const port = node && outputPorts(node.data)[0]?.name;
  const ready = nodeId !== undefined && evaluation.reports.get(nodeId)?.status === "ok";

  useEffect(() => {
    if (!ready || nodeId === undefined || port === undefined) {
      setOutput(undefined);
      return;
    }
    let current = true;
    engine.output(nodeId, port).then(
      (result) => current && setOutput(result),
      () => current && setOutput(undefined)
    );
    return () => {
      current = false;
    };
  }, [nodeId, port, ready, evaluation.version]);

  return output;
}
