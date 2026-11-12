import React, { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { engine } from "@/engine/client";
import { catalog, getMeta, isOpId, type Proposal } from "@/engine/wasm";
import { GraphDispatchContext, ReportsContext } from "@/graph/context";
import { saveBytes } from "@/graph/files";
import { nodeTitle, outputPorts, type ConfigRecord } from "@/graph/model";
import {
  clearStorage,
  loadFromStorage,
  loadFromUrl,
  saveToStorage,
  type GraphSnapshot,
} from "@/graph/persistence";
import { useEvaluation, useNodeOutput } from "@/graph/useEvaluation";
import { createInitialState, graphReducer, type ChainStep } from "@/state/graphReducer";
import { ErrorBoundary } from "./ErrorBoundary";
import { GraphCanvas } from "./GraphCanvas";
import { GraphControls } from "./GraphControls";
import InputOutput from "./InputOutput";
import Sidebar, { type SidebarItem } from "./Sidebar";
import { NotificationContainer } from "./ui/notification";

const INPUT_ITEM: SidebarItem = {
  id: "input",
  name: "Input",
  description: "A text input to start a flow from",
  tags: ["I/O"],
};

const FILE_ITEM: SidebarItem = {
  id: "file",
  name: "File",
  description: "A file whose bytes start a flow",
  tags: ["I/O"],
};

const sidebarItems: SidebarItem[] = [
  INPUT_ITEM,
  FILE_ITEM,
  ...catalog.map((meta) => ({
    id: meta.id,
    name: meta.name,
    description: meta.description,
    link: meta.link ?? undefined,
    tags: meta.tags,
  })),
];

const SAVE_DELAY_MS = 500;

/** A share link wins over the locally saved graph, which wins over an empty one. */
function loadInitialState(): GraphSnapshot {
  const loaded = loadFromUrl() ?? loadFromStorage();
  return loaded && loaded.nodes.length > 0 ? loaded : createInitialState();
}

const Workspace: React.FC = () => {
  const [state, dispatch] = useReducer(graphReducer, undefined, loadInitialState);
  const [autoCalculate, setAutoCalculate] = useState(true);
  const { evaluation, run } = useEvaluation(state.nodes, state.edges, autoCalculate);
  const { reports } = evaluation;

  useEffect(() => {
    const timer = setTimeout(() => saveToStorage(state), SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [state]);

  // The engine keeps a file's bytes until its node is gone.
  useEffect(() => {
    engine.retainSources(
      state.nodes.filter((node) => node.data.kind === "file").map((node) => node.id)
    );
  }, [state.nodes]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") dispatch({ type: "SELECT", nodeId: "" });
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleOperationSelect = useCallback((item: SidebarItem) => {
    if (isOpId(item.id)) dispatch({ type: "ADD_OP", opId: item.id });
    else dispatch({ type: item.id === FILE_ITEM.id ? "ADD_FILE" : "ADD_INPUT" });
  }, []);

  const handleClear = useCallback(() => {
    clearStorage();
    dispatch({ type: "LOAD", snapshot: createInitialState() });
  }, []);

  const handleImport = useCallback(
    (snapshot: GraphSnapshot) => dispatch({ type: "LOAD", snapshot }),
    []
  );

  const inputs = useMemo(() => {
    const nodes = state.nodes.filter((node) => node.data.kind === "input");
    return nodes.map((node, index) => ({
      id: node.id,
      label: nodes.length > 1 ? `Input ${index + 1}` : "Input",
    }));
  }, [state.nodes]);

  const selectedInput = state.nodes.find((node) => node.id === state.selectedInputId);
  const inputText = selectedInput?.data.kind === "input" ? selectedInput.data.text : "";
  const selectedNode = state.nodes.find((node) => node.id === state.selectedNodeId);

  const selectedOutput = useNodeOutput(selectedNode, evaluation);

  const output = useMemo(() => {
    if (evaluation.error) {
      return `The engine stopped: ${evaluation.error}\nThis usually means the data got too large to hold in memory.`;
    }
    if (!selectedNode) return "";
    const report = reports.get(selectedNode.id);
    switch (report?.status) {
      case "ok": {
        if (!selectedOutput) return "";
        const { text, truncated, total_chars } = selectedOutput;
        return truncated
          ? `${text}\n\n[Showing the first ${text.length.toLocaleString()} of ${total_chars.toLocaleString()} characters]`
          : text;
      }
      case "error":
        return `Error: ${report.error.message}`;
      case "waiting":
        return "This step has an input that is not connected yet.";
      case "blocked":
        return "An earlier step has to succeed before this one can run.";
      default:
        return "";
    }
  }, [selectedNode, reports, selectedOutput, evaluation.error]);

  const selectedReport = selectedNode && reports.get(selectedNode.id);
  const downloadable = selectedReport?.status === "ok" ? selectedReport.outputs[0] : undefined;
  const handleDownload = useCallback(async () => {
    if (!selectedNode || !downloadable) return;
    const bytes = await engine.outputBytes(selectedNode.id, outputPorts(selectedNode.data)[0].name);
    if (bytes) {
      saveBytes(bytes, `${selectedNode.id}.${downloadable.type === "bytes" ? "bin" : "txt"}`);
    }
  }, [selectedNode, downloadable]);

  // Auto mode works on the selected step, or on the input when no step is selected.
  const autoTarget = selectedNode ?? selectedInput;
  const autoReady = !!autoTarget && reports.get(autoTarget.id)?.status === "ok";
  const [proposals, setProposals] = useState<Proposal[] | null>(null);
  const [suggesting, setSuggesting] = useState(false);

  // Proposals describe one value; forget them when it may have changed.
  const autoTargetId = autoTarget?.id;
  useEffect(() => setProposals(null), [autoTargetId, evaluation.version]);

  const handleSuggest = useCallback(async () => {
    if (!autoTarget) return;
    setSuggesting(true);
    try {
      setProposals(await engine.suggest(autoTarget.id, outputPorts(autoTarget.data)[0].name));
    } catch (error) {
      console.error("Auto mode failed:", error);
      setProposals([]);
    } finally {
      setSuggesting(false);
    }
  }, [autoTarget]);

  const handleApply = useCallback(
    (proposal: Proposal) => {
      if (!autoTargetId) return;
      const steps: ChainStep[] = proposal.steps.flatMap((step) =>
        isOpId(step.op_id)
          ? [{ opId: step.op_id, config: step.config as ConfigRecord, note: step.reason }]
          : []
      );
      dispatch({ type: "ADD_CHAIN", anchorId: autoTargetId, steps });
    },
    [autoTargetId]
  );

  const auto = autoTarget && {
    target: nodeTitle(autoTarget.data),
    ready: autoReady,
    busy: suggesting,
    proposals: proposals?.map((proposal) => ({
      proposal,
      steps: proposal.steps.map((step) => (isOpId(step.op_id) ? getMeta(step.op_id).name : step.op_id)),
    })),
    onSuggest: handleSuggest,
    onApply: handleApply,
  };

  const status = evaluation.running
    ? "Calculating…"
    : evaluation.elapsedMs !== null
      ? `Ran in ${evaluation.elapsedMs < 10 ? evaluation.elapsedMs.toFixed(1) : Math.round(evaluation.elapsedMs)} ms`
      : "";

  const { selectedInputId } = state;
  const handleInputChange = useCallback(
    (text: string) => dispatch({ type: "SET_INPUT_TEXT", nodeId: selectedInputId, text }),
    [selectedInputId]
  );
  const handleInputTabSelect = useCallback(
    (nodeId: string) => dispatch({ type: "SELECT", nodeId }),
    []
  );

  return (
    <GraphDispatchContext.Provider value={dispatch}>
      <ReportsContext.Provider value={reports}>
        <div className="flex min-h-screen">
          <ErrorBoundary>
            <Sidebar operations={sidebarItems} onOperationSelect={handleOperationSelect} />
          </ErrorBoundary>
          <div className="flex-1 flex flex-col">
            <GraphControls state={state} onImport={handleImport} onClear={handleClear} />
            <div className="flex-1 flex">
              <div className="w-2/3 p-4 border-r">
                <ErrorBoundary onError={(error) => console.error("Graph error:", error)}>
                  <GraphCanvas nodes={state.nodes} edges={state.edges} />
                </ErrorBoundary>
              </div>
              <div className="w-1/3 flex flex-col">
                <ErrorBoundary>
                  <InputOutput
                    input={inputText}
                    output={output}
                    status={status}
                    onDownload={downloadable && handleDownload}
                    auto={auto}
                    onInputChange={handleInputChange}
                    calculate={run}
                    autoCalculate={autoCalculate}
                    onAutoCalculateChange={setAutoCalculate}
                    selectedNodeId={selectedNode?.id}
                    selectedNodeTitle={selectedNode && nodeTitle(selectedNode.data)}
                    inputs={inputs}
                    selectedInputId={state.selectedInputId}
                    onInputTabSelect={handleInputTabSelect}
                  />
                </ErrorBoundary>
              </div>
            </div>
          </div>
          <NotificationContainer />
        </div>
      </ReportsContext.Provider>
    </GraphDispatchContext.Provider>
  );
};

export default React.memo(Workspace);
