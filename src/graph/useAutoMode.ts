import { useCallback, useEffect, useState, type Dispatch } from "react";
import { engine } from "@/engine/client";
import { getMeta, isOpId, type Proposal } from "@/engine/wasm";
import type { ChainStep, GraphAction } from "@/state/graphReducer";
import { nodeTitle, outputPorts, type ConfigRecord, type GraphNode } from "./model";

/** Auto mode for one node's output, as the output panel consumes it. */
export interface AutoMode {
  /** Name of the step the proposals are for. */
  target: string;
  /** Whether that step has an output to analyse. */
  ready: boolean;
  busy: boolean;
  /** Absent until asked for; empty when nothing would make the output more readable. */
  proposals?: { proposal: Proposal; steps: string[] }[];
  suggest: () => void;
  apply: (proposal: Proposal) => void;
}

/**
 * Asks the engine for decoding steps for `target`'s output and adds an accepted
 * proposal to the graph. `version` is the evaluation counter: proposals describe one
 * value, so they are dropped when a new run may have changed it.
 */
export function useAutoMode(
  target: GraphNode | undefined,
  ready: boolean,
  version: number,
  dispatch: Dispatch<GraphAction>
): AutoMode | undefined {
  const [proposals, setProposals] = useState<Proposal[] | null>(null);
  const [busy, setBusy] = useState(false);
  const targetId = target?.id;

  useEffect(() => setProposals(null), [targetId, version]);

  const suggest = useCallback(async () => {
    if (!target) return;
    setBusy(true);
    try {
      setProposals(await engine.suggest(target.id, outputPorts(target.data)[0].name));
    } catch (error) {
      console.error("Auto mode failed:", error);
      setProposals([]);
    } finally {
      setBusy(false);
    }
  }, [target]);

  const apply = useCallback(
    (proposal: Proposal) => {
      if (!targetId) return;
      const steps: ChainStep[] = proposal.steps.flatMap((step) =>
        isOpId(step.op_id)
          ? [{ opId: step.op_id, config: step.config as ConfigRecord, note: step.reason }]
          : []
      );
      dispatch({ type: "ADD_CHAIN", anchorId: targetId, steps });
    },
    [targetId, dispatch]
  );

  if (!target) return undefined;
  return {
    target: nodeTitle(target.data),
    ready,
    busy,
    proposals: proposals?.map((proposal) => ({
      proposal,
      steps: proposal.steps.map((step) =>
        isOpId(step.op_id) ? getMeta(step.op_id).name : step.op_id
      ),
    })),
    suggest,
    apply,
  };
}
