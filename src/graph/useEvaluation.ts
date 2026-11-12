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

/** Whether a node ran but had its output released to save memory. */
export function wasReleased(report: NodeReport | undefined): boolean {
  return report?.status === "ok" && !report.retained;
}

/**
 * Runs the graph when it changes. `keep` lists the nodes being looked at: with large
 * data the engine releases other intermediate results, and looking at a released one
 * runs the graph again with that node kept.
 */
export function useEvaluation(
  nodes: GraphNode[],
  edges: Edge[],
  auto: boolean,
  keep: readonly string[]
) {
  const [evaluation, setEvaluation] = useState(EMPTY);
  const fingerprint = useFingerprint(nodes, edges);
  const latest = useRef({ nodes, edges, keep });
  latest.current = { nodes, edges, keep };

  const run = useCallback(() => {
    const { nodes, edges, keep } = latest.current;
    setEvaluation((previous) => ({ ...previous, running: true }));
    engine.evaluate(toEngineGraph(nodes, edges), [...keep]).then(
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

  // At most one extra run per selection and result, so a node that cannot be kept
  // does not cause a loop.
  const keepKey = keep.join(",");
  const rerunFor = useRef("");
  useEffect(() => {
    if (!auto || evaluation.running) return;
    const key = `${evaluation.version}:${keepKey}`;
    const released = latest.current.keep.some((id) => wasReleased(evaluation.reports.get(id)));
    if (released && rerunFor.current !== key) {
      rerunFor.current = key;
      run();
    }
  }, [auto, keepKey, evaluation, run]);

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
  const report = nodeId === undefined ? undefined : evaluation.reports.get(nodeId);
  const ready = report?.status === "ok" && report.retained;

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
