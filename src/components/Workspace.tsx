"use client";

import React, {
  useState,
  useCallback,
  useMemo,
  useReducer,
  useEffect,
} from "react";
import {
  Edge,
  Connection,
  addEdge,
  Node,
  NodeChange,
  EdgeChange,
} from "@xyflow/react";
import {
  Operation,
  OperationTags,
  outputTypes,
  ConfigValues,
  DataValue,
} from "@/operations/types";
import {
  executeOperation,
  validateOperationInputs,
  createDataValueOutputValues,
  getOperationDisplayValue,
  getOperationFullOutput,
} from "@/operations/execution";
import { operations } from "@/operations/operations";
import {
  topologicalSort,
  getConnectedNodesFromInput,
} from "@/nodes/topological-sort";
import { checkInputNodeExists } from "@/nodes/utils";
import { Input } from "@/operations/io";
import { debounce, isEqual } from "lodash";
import { graphReducer, initialGraphState } from "@/state/graphReducer";
import { ErrorBoundary } from "./ErrorBoundary";
import { GraphControls } from "./GraphControls";
import { NotificationContainer } from "./ui/notification";
import { showNotification } from "@/utils/notifications";
import {
  saveGraphToLocalStorage,
  loadGraphFromLocalStorage,
  clearStoredGraphState,
} from "@/state/storage";
import { decodeGraphFromURL } from "@/utils/urlSharing";

import Sidebar from "@/components/Sidebar";
import InputOutput from "@/components/InputOutput";
import OperationGraph from "@/components/operationGraph";

const initialNodes: Node[] = [
  {
    id: "input-node",
    type: "custom",
    data: {
      ...Input,
      value: "",
      outputValues: { output: "" },
    },
    position: { x: 250, y: 10 },
  },
];

const generateShortId = (prefix: string) =>
  `${prefix}-${Math.random().toString(36).slice(2, 7)}`;

const Workspace: React.FC = () => {
  const [state, dispatch] = useReducer(graphReducer, {
    ...initialGraphState,
    nodes: initialNodes,
  });

  const [autoCalculate, setAutoCalculate] = useState(true);

  const onConnect = useCallback(
    (params: Connection) => {
      dispatch({
        type: "SET_EDGES",
        edges: addEdge(params, state.edges),
      });
    },
    [state.edges]
  );

  const handleInputChange = useCallback((value: string, nodeId: string) => {
    dispatch({ type: "SET_INPUT", value });
    dispatch({ type: "UPDATE_NODE_VALUE", nodeId, value });
  }, []);

  const handleInputTabSelect = useCallback((nodeId: string) => {
    dispatch({ type: "SELECT_INPUT", nodeId });
  }, []);

  const handleOperationSelect = useCallback(
    (operation: Operation) => {
      const connectedNodes = getConnectedNodesFromInput(
        state.selectedInputId,
        state.nodes,
        state.edges
      );
      const sortedConnectedNodes = topologicalSort(connectedNodes, state.edges);
      const lastConnectedNode =
        sortedConnectedNodes[sortedConnectedNodes.length - 1];

      const newNode: Node = {
        id: generateShortId(operation.id),
        type: "custom",
        data: {
          ...operation,
          onConfigChange: (newConfig: ConfigValues) => {
            if (operation.funcBuilder) {
              dispatch({
                type: "UPDATE_NODE_CONFIG",
                nodeId: newNode.id,
                config: newConfig,
              });
            }
          },
        },
        position: {
          x: lastConnectedNode ? lastConnectedNode.position.x : 250,
          y: lastConnectedNode
            ? lastConnectedNode.position.y +
              (lastConnectedNode.measured?.height || 0) +
              80
            : 10,
        },
      };

      if (operation.tags.includes(OperationTags.IO)) {
        dispatch({ type: "ADD_NODE", node: newNode });
        dispatch({ type: "SELECT_INPUT", nodeId: newNode.id });
      } else {
        const newEdge =
          lastConnectedNode &&
          operation.inputs &&
          Object.keys(operation.inputs).length > 0
            ? {
                id: `${lastConnectedNode.id}-${newNode.id}`,
                source: lastConnectedNode.id,
                sourceHandle: "output",
                target: newNode.id,
                targetHandle: Object.keys(operation.inputs)[0],
              }
            : undefined;

        dispatch({ type: "ADD_NODE", node: newNode, edge: newEdge });
        dispatch({
          type: "SELECT_NODE",
          nodeId: newNode.id,
          nodeTitle: operation.name,
        });
      }
    },
    [state.selectedInputId, state.nodes, state.edges]
  );

  const calculate = useCallback(async () => {
    try {
      dispatch({ type: "SET_CALCULATING", isCalculating: true });

      if (!checkInputNodeExists(state.nodes)) {
        dispatch({ type: "SET_OUTPUT", value: "Error: Input node not found" });
        return;
      }

      const newNodes = [...state.nodes];
      const calculationOrder = topologicalSort(state.nodes, state.edges).filter(
        (node) => state.dirtyNodes.has(node.id)
      );
      if (calculationOrder.length === 0) return;
      let hasChanges = false;

      for (const node of calculationOrder) {
        dispatch({ type: "CLEAR_ERROR", nodeId: node.id });
      }

      for (const node of calculationOrder) {
        if (!node.data) continue;
        const operation = node.data as Operation;

        if (operation.id === "input") {
          const inputValue = operation.value || "";
          const calculated = executeOperation(operation, [
            { type: "text", value: inputValue },
          ]);
          const newOutputValues = createDataValueOutputValues(
            operation,
            calculated
          );

          const nodeIndex = newNodes.findIndex((n) => n.id === node.id);
          if (nodeIndex !== -1) {
            const newNode = {
              ...node,
              data: {
                ...node.data,
                value: getOperationDisplayValue(operation),
                outputValues: newOutputValues,
                error: undefined,
              },
            };

            if (!isEqual(operation.outputValues, newOutputValues)) {
              newNodes[nodeIndex] = newNode;
              hasChanges = true;
            }
          }
          continue;
        }

        if (!operation.inputs || Object.keys(operation.inputs).length === 0)
          continue;

        try {
          const inputValues: (outputTypes | DataValue)[] = state.edges
            .filter((edge) => edge.target === node.id)
            .map((edge) => {
              const sourceNode = newNodes.find((n) => n.id === edge.source);
              if (!sourceNode) return undefined;
              const sourceData = sourceNode?.data as Operation;
              const outputValues = sourceData.outputValues;
              if (!outputValues) return undefined;
              return outputValues[edge.sourceHandle || "output"];
            })
            .filter((value) => value !== undefined) as (
            | outputTypes
            | DataValue
          )[];

          if (inputValues.length === 0) continue;

          validateOperationInputs(
            operation,
            inputValues as (outputTypes | DataValue)[]
          );

          const calculated = executeOperation(
            operation,
            inputValues as (outputTypes | DataValue)[]
          );

          const newOutputValues = createDataValueOutputValues(
            operation,
            calculated
          );

          const nodeIndex = newNodes.findIndex((n) => n.id === node.id);
          if (nodeIndex !== -1) {
            const newNode = {
              ...node,
              data: {
                ...node.data,
                value: getOperationDisplayValue(operation),
                outputValues: newOutputValues,
                error: undefined,
              },
            };

            if (!isEqual(operation.outputValues, newOutputValues)) {
              newNodes[nodeIndex] = newNode;
              hasChanges = true;
            }
          }
        } catch (nodeError) {
          const errorMessage =
            nodeError instanceof Error ? nodeError.message : "Unknown error";
          dispatch({ type: "SET_ERROR", nodeId: node.id, error: errorMessage });

          const nodeIndex = newNodes.findIndex((n) => n.id === node.id);
          if (nodeIndex !== -1) {
            newNodes[nodeIndex] = {
              ...node,
              data: {
                ...node.data,
                error: errorMessage,
              },
            };
            hasChanges = true;
          }
        }
      }

      const selectedNode = newNodes.find((n) => n.id === state.selectedNodeId);
      if (selectedNode) {
        const selectedData = selectedNode.data as Operation;
        dispatch({
          type: "SET_OUTPUT",
          value: getOperationFullOutput(selectedData),
        });
      }

      if (hasChanges) {
        dispatch({
          type: "SET_NODES_AND_CLEAR_DIRTY",
          nodes: newNodes,
        });
      }
    } catch (error) {
      console.error("Error during calculation:", error);
      dispatch({
        type: "SET_OUTPUT",
        value:
          "Error: " +
          (error instanceof Error ? error.message : "Unknown error"),
      });
    } finally {
      dispatch({ type: "SET_CALCULATING", isCalculating: false });
    }
  }, [state.nodes, state.edges, state.selectedNodeId, state.dirtyNodes]);

  const debouncedCalculate = useMemo(
    () => debounce(calculate, 100),
    [calculate]
  );

  useEffect(() => {
    if (!autoCalculate) return;
    debouncedCalculate();
    return () => {
      debouncedCalculate.cancel();
    };
  }, [autoCalculate, debouncedCalculate]);

  // Auto-save to localStorage (debounced)
  const debouncedSave = useMemo(
    () => debounce((state) => saveGraphToLocalStorage(state), 500),
    []
  );

  useEffect(() => {
    debouncedSave(state);
    return () => {
      debouncedSave.cancel();
    };
  }, [state, debouncedSave]);

  // Load from URL or localStorage on mount
  useEffect(() => {
    const loadInitialState = () => {
      // Priority: URL > LocalStorage > default
      const urlState = decodeGraphFromURL();
      if (urlState) {
        dispatch({ type: "LOAD_GRAPH", state: urlState });
        showNotification("Graph loaded from URL", "info");
        return;
      }

      const storedState = loadGraphFromLocalStorage();
      if (storedState) {
        dispatch({ type: "LOAD_GRAPH", state: storedState });
        showNotification("Graph loaded from storage", "info");
      }
    };

    loadInitialState();
  }, []);

  useEffect(() => {
    return () => {
      state.nodes.forEach((node) => {
        const operation = node.data as Operation;
        if (operation?.cleanup) {
          operation.cleanup();
        }
      });
    };
  }, [state.nodes]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "s") {
        event.preventDefault();
        // Trigger export
        const graphData = {
          version: "1.0",
          timestamp: new Date().toISOString(),
          nodes: state.nodes,
          edges: state.edges,
          selectedInputId: state.selectedInputId,
          input: state.input,
          selectedNodeId: state.selectedNodeId,
          selectedNodeTitle: state.selectedNodeTitle,
        };

        const jsonString = JSON.stringify(graphData, null, 2);
        const blob = new Blob([jsonString], { type: "application/json" });
        const url = URL.createObjectURL(blob);

        const timestamp = new Date()
          .toISOString()
          .replace(/[:.]/g, "-")
          .slice(0, 19);
        const filename = `cybernohut-graph-${timestamp}.json`;

        const link = document.createElement("a");
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);

        showNotification("Graph exported!", "success");
      }

      if ((event.ctrlKey || event.metaKey) && event.key === "z") {
        event.preventDefault();
        console.log("Undo functionality not yet implemented");
      }

      if (event.key === "Escape") {
        dispatch({ type: "SELECT_NODE", nodeId: "", nodeTitle: "" });
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [state]);

  const handleNodeClick = useCallback(
    (value: string, nodeId: string) => {
      const node = state.nodes.find((n) => n.id === nodeId);
      if (!node) return;

      const nodeData = node.data as Operation;
      if (nodeData.tags.includes(OperationTags.IO)) {
        dispatch({ type: "SELECT_INPUT", nodeId });
      } else {
        dispatch({ type: "SET_OUTPUT", value });
        dispatch({ type: "SELECT_NODE", nodeId, nodeTitle: nodeData.name });
      }
    },
    [state.nodes]
  );

  const handleNodesDelete = useCallback(
    (deleted: Node[], remainingNodes: Node[], newEdges: Edge[]) => {
      deleted.forEach((node) => {
        const operation = node.data as Operation;
        if (operation?.cleanup) {
          operation.cleanup();
        }
        dispatch({ type: "CLEANUP_NODE", nodeId: node.id });
      });

      if (deleted.some((node) => node.id === state.selectedInputId)) {
        const remainingInputNodes = remainingNodes.filter((node) =>
          (node.data as Operation).tags.includes(OperationTags.IO)
        );
        if (remainingInputNodes.length > 0) {
          dispatch({ type: "SELECT_INPUT", nodeId: remainingInputNodes[0].id });
        } else {
          dispatch({ type: "SELECT_INPUT", nodeId: "input-node" });
        }
      }

      if (deleted.some((node) => node.id === state.selectedNodeId)) {
        if (remainingNodes.length > 0) {
          try {
            const sortedNodes = topologicalSort(remainingNodes, newEdges);
            const lastNode = sortedNodes[sortedNodes.length - 1];

            if (lastNode) {
              const nodeData = lastNode.data as Operation;
              dispatch({
                type: "SELECT_NODE",
                nodeId: lastNode.id,
                nodeTitle: nodeData.name,
              });
            } else {
              dispatch({ type: "SELECT_NODE", nodeId: "", nodeTitle: "" });
            }
          } catch (error) {
            console.error("Error in topological sort:", error);
            const lastNode = remainingNodes[remainingNodes.length - 1];
            const nodeData = lastNode.data as Operation;
            dispatch({
              type: "SELECT_NODE",
              nodeId: lastNode.id,
              nodeTitle: nodeData.name,
            });
          }
        } else {
          dispatch({ type: "SELECT_NODE", nodeId: "", nodeTitle: "" });
        }
      }
    },
    [state.selectedNodeId, state.selectedInputId]
  );

  const handleNodesChange = useCallback((changes: NodeChange[]) => {
    dispatch({ type: "APPLY_NODE_CHANGES", changes });
  }, []);

  const handleEdgesChange = useCallback((changes: EdgeChange[]) => {
    dispatch({ type: "APPLY_EDGE_CHANGES", changes });
  }, []);

  // GraphControls handlers
  const handleImport = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      try {
        const content = e.target?.result as string;
        const parsed = JSON.parse(content);

        // Validate structure
        if (!parsed.nodes || !parsed.edges) {
          throw new Error("Invalid graph file format");
        }

        dispatch({ type: "LOAD_GRAPH", state: parsed });
        showNotification("Graph imported successfully!", "success");
      } catch (error) {
        console.error("Import failed:", error);
        showNotification("Invalid graph file", "error");
      }
    };

    reader.readAsText(file);
  }, []);

  const handleClear = useCallback(() => {
    dispatch({
      type: "LOAD_GRAPH",
      state: { ...initialGraphState, nodes: initialNodes },
    });
    clearStoredGraphState();
  }, []);

  const graphProps = useMemo(
    () => ({
      setEdges: (edges: Edge[]) => dispatch({ type: "SET_EDGES", edges }),
      nodes: state.nodes,
      edges: state.edges,
      onNodesChange: handleNodesChange,
      onEdgesChange: handleEdgesChange,
      onConnect,
      onNodeClick: handleNodeClick,
      onNodesDelete: handleNodesDelete,
    }),
    [
      state.nodes,
      state.edges,
      handleNodesChange,
      handleEdgesChange,
      onConnect,
      handleNodeClick,
      handleNodesDelete,
    ]
  );

  const inputOutputProps = useMemo(
    () => ({
      autoCalculate,
      onAutoCalculateChange: setAutoCalculate,
      calculate,
      input: state.input,
      output: state.output,
      onInputChange: handleInputChange,
      selectedNodeId: state.selectedNodeId,
      selectedNodeTitle: state.selectedNodeTitle,
      nodes: state.nodes,
      selectedInputId: state.selectedInputId,
      onInputTabSelect: handleInputTabSelect,
      isCalculating: state.isCalculating,
    }),
    [
      autoCalculate,
      calculate,
      state.input,
      state.output,
      state.selectedNodeId,
      state.selectedNodeTitle,
      state.nodes,
      state.selectedInputId,
      state.isCalculating,
      handleInputChange,
      handleInputTabSelect,
    ]
  );

  const sidebarProps = useMemo(
    () => ({
      operations,
      onOperationSelect: handleOperationSelect,
    }),
    [handleOperationSelect]
  );

  return (
    <div className="flex min-h-screen">
      <ErrorBoundary>
        <Sidebar {...sidebarProps} />
      </ErrorBoundary>
      <div className="flex-1 flex flex-col">
        <GraphControls
          state={state}
          onImport={handleImport}
          onClear={handleClear}
        />
        <div className="flex-1 flex">
          <div className="w-2/3 p-4 border-r">
            <ErrorBoundary
              onError={(error) => {
                console.error("Graph error:", error);
              }}
            >
              <OperationGraph {...graphProps} />
            </ErrorBoundary>
          </div>
          <div className="w-1/3 flex flex-col">
            <ErrorBoundary>
              <InputOutput {...inputOutputProps} />
            </ErrorBoundary>
          </div>
        </div>
      </div>
      <NotificationContainer />
    </div>
  );
};

export default React.memo(Workspace);
