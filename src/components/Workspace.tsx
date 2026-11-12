import React, { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { engine } from "@/engine/client";
import { catalog, isOpId } from "@/engine/wasm";
import { GraphDispatchContext, ReportsContext } from "@/graph/context";
import { saveBytes } from "@/graph/files";
import { nodeTitle, outputPorts } from "@/graph/model";
import {
  clearStorage,
  loadFromStorage,
  loadFromUrl,
  saveToStorage,
  type GraphSnapshot,
} from "@/graph/persistence";
import { useAutoMode } from "@/graph/useAutoMode";
import { useEvaluation, useNodeOutput, wasReleased } from "@/graph/useEvaluation";
import { createInitialState, graphReducer } from "@/state/graphReducer";
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
  const [sidebarOpen, setSidebarOpen] = useState(false);
  // The nodes whose full output the panel shows or auto mode reads.
  const watched = useMemo(
    () => [state.selectedNodeId, state.selectedInputId].filter(Boolean),
    [state.selectedNodeId, state.selectedInputId]
  );
  const { evaluation, run } = useEvaluation(state.nodes, state.edges, autoCalculate, watched);
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
      if (event.key !== "Escape") return;
      setSidebarOpen(false);
      dispatch({ type: "SELECT", nodeId: "" });
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleOperationSelect = useCallback((item: SidebarItem) => {
    if (isOpId(item.id)) dispatch({ type: "ADD_OP", opId: item.id });
    else dispatch({ type: item.id === FILE_ITEM.id ? "ADD_FILE" : "ADD_INPUT" });
    setSidebarOpen(false);
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
        if (wasReleased(report)) {
          return autoCalculate
            ? "Calculating…"
            : "This step's output was released to save memory. Press Run to compute it again.";
        }
        if (!selectedOutput) return "";
        const { text, truncated, total_chars } = selectedOutput;
        return truncated
          ? `${text}\n\n[Showing the first ${text.length.toLocaleString()} of ${total_chars.toLocaleString()} characters]`
          : text;
      }
      case "error":
        return report.error.code === "invalid_config"
          ? `This step is not set up yet: ${report.error.message}`
          : `Error: ${report.error.message}`;
      case "waiting":
        return "This step has an input that is not connected yet.";
      case "blocked":
        return "An earlier step has to succeed before this one can run.";
      default:
        return "";
    }
  }, [selectedNode, reports, selectedOutput, evaluation.error, autoCalculate]);

  const selectedReport = selectedNode && reports.get(selectedNode.id);
  const downloadable =
    selectedReport?.status === "ok" && selectedReport.retained
      ? selectedReport.outputs[0]
      : undefined;
  const handleDownload = useCallback(async () => {
    if (!selectedNode || !downloadable) return;
    const bytes = await engine.outputBytes(selectedNode.id, outputPorts(selectedNode.data)[0].name);
    if (bytes) {
      saveBytes(bytes, `${selectedNode.id}.${downloadable.type === "bytes" ? "bin" : "txt"}`);
    }
  }, [selectedNode, downloadable]);

  // Auto mode works on the selected step, or on the input when no step is selected.
  const autoTarget = selectedNode ?? selectedInput;
  const autoReport = autoTarget && reports.get(autoTarget.id);
  const autoReady = autoReport?.status === "ok" && autoReport.retained;
  const auto = useAutoMode(autoTarget, autoReady, evaluation.version, dispatch);

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
        <div className="flex h-screen overflow-hidden">
          <ErrorBoundary>
            <Sidebar
              operations={sidebarItems}
              onOperationSelect={handleOperationSelect}
              open={sidebarOpen}
            />
          </ErrorBoundary>
          {sidebarOpen && (
            <div
              className="fixed inset-0 z-30 bg-black/30 md:hidden"
              onClick={() => setSidebarOpen(false)}
              aria-hidden="true"
            />
          )}
          <div className="flex min-w-0 flex-1 flex-col">
            <GraphControls
              state={state}
              onImport={handleImport}
              onClear={handleClear}
              autoCalculate={autoCalculate}
              onAutoCalculateChange={setAutoCalculate}
              onRun={run}
              onShowOperations={() => setSidebarOpen(true)}
            />
            <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
              <div className="min-h-0 min-w-0 flex-1 bg-gray-100">
                <ErrorBoundary onError={(error) => console.error("Graph error:", error)}>
                  <GraphCanvas nodes={state.nodes} edges={state.edges} />
                </ErrorBoundary>
              </div>
              <div className="h-1/2 shrink-0 border-t bg-white lg:h-auto lg:w-[26rem] lg:border-l lg:border-t-0">
                <ErrorBoundary>
                  <InputOutput
                    input={inputText}
                    onInputChange={handleInputChange}
                    inputs={inputs}
                    selectedInputId={state.selectedInputId}
                    onInputTabSelect={handleInputTabSelect}
                    output={output}
                    status={status}
                    selectedNodeTitle={selectedNode && nodeTitle(selectedNode.data)}
                    onDownload={downloadable && handleDownload}
                    auto={auto}
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
