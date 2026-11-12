import React, { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { catalog, isOpId, outputText } from "@/engine/wasm";
import { GraphDispatchContext, ReportsContext } from "@/graph/context";
import { nodeTitle, outputPorts } from "@/graph/model";
import {
  clearStorage,
  loadFromStorage,
  loadFromUrl,
  saveToStorage,
  type GraphSnapshot,
} from "@/graph/persistence";
import { useEvaluation } from "@/graph/useEvaluation";
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

const sidebarItems: SidebarItem[] = [
  INPUT_ITEM,
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
  const { reports, run } = useEvaluation(state.nodes, state.edges, autoCalculate);

  useEffect(() => {
    const timer = setTimeout(() => saveToStorage(state), SAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [state]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") dispatch({ type: "SELECT", nodeId: "" });
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleOperationSelect = useCallback((item: SidebarItem) => {
    if (isOpId(item.id)) dispatch({ type: "ADD_OP", opId: item.id });
    else dispatch({ type: "ADD_INPUT" });
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

  // The engine keeps the values of its last evaluation; `reports` changing is the
  // signal that there is a new full output to fetch for the selected node.
  const output = useMemo(() => {
    if (!selectedNode) return "";
    const report = reports.get(selectedNode.id);
    switch (report?.status) {
      case "ok":
        return outputText(selectedNode.id, outputPorts(selectedNode.data)[0].name) ?? "";
      case "error":
        return `Error: ${report.error.message}`;
      case "waiting":
        return "This step has an input that is not connected yet.";
      case "blocked":
        return "An earlier step has to succeed before this one can run.";
      default:
        return "";
    }
  }, [selectedNode, reports]);

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
