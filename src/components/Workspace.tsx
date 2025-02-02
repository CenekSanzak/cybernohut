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
  IOTypes,
} from "@/operations/types";
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

      const lastNodeY = lastConnectedNode
        ? lastConnectedNode.position.y +
          (lastConnectedNode.measured?.height || 0)
        : 0;
      const lastNodeX = lastConnectedNode ? lastConnectedNode.position.x : 250;

      const newNode: Node = {
        id: generateShortId(operation.id),
        type: "custom",
        data: operation,
        position: { x: lastNodeX, y: lastNodeY + 80 },
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
                targetHandle: "input",
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
      if (!checkInputNodeExists(state.nodes)) {
        dispatch({ type: "SET_OUTPUT", value: "Error: Input node not found" });
        return;
      }

      let lastValue: outputTypes = "";
      const newNodes = [...state.nodes];
      const calculationOrder = topologicalSort(state.nodes, state.edges);
      let hasChanges = false;
      for (const node of calculationOrder) {
        if (!node.data) continue;
        const operation = node.data as Operation;
        if (!operation.inputs || Object.keys(operation.inputs).length === 0)
          continue;
        const inputValues: outputTypes[] = state.edges
          .filter((edge) => edge.target === node.id)
          .map((edge) => {
            const sourceNode = newNodes.find((n) => n.id === edge.source);
            if (!sourceNode) return undefined;
            const sourceData = sourceNode?.data as Operation;
            const outputValues = sourceData.outputValues;
            if (!outputValues) return undefined;
            return outputValues[edge.sourceHandle || "output"];
          })
          .filter((value) => value !== undefined);
        if (inputValues.length === 0) continue;
        const calculated = operation.func.apply({}, inputValues);
        lastValue = calculated[0];

        const newOutputValues: { [key: string]: outputTypes } = {};
        const outputKeys = Object.keys(
          operation.outputs as { [key: string]: IOTypes }
        );

        for (let index = 0; index < outputKeys.length; index++) {
          const key = outputKeys[index];
          newOutputValues[key] = Array.isArray(calculated)
            ? calculated[index]
            : calculated;
        }

        const nodeIndex = newNodes.findIndex((n) => n.id === node.id);
        if (nodeIndex !== -1) {
          const newNode = {
            ...node,
            data: {
              ...node.data,
              value: calculated.toString(),
              outputValues: newOutputValues,
            },
          };

          if (!isEqual(operation.outputValues, newOutputValues)) {
            newNodes[nodeIndex] = newNode;
            hasChanges = true;
          }
        }
      }

      const selectedNode = newNodes.find((n) => n.id === state.selectedNodeId);
      if (selectedNode) {
        const selectedData = selectedNode.data as Operation;
        dispatch({
          type: "SET_OUTPUT",
          value: selectedData.value?.toString() || "",
        });
      } else if (lastValue !== undefined) {
        dispatch({ type: "SET_OUTPUT", value: lastValue.toString() });
      }

      if (hasChanges) {
        dispatch({ type: "SET_NODES", nodes: newNodes });
      }
    } catch (error) {
      console.error("Error during calculation:", error);
      dispatch({
        type: "SET_OUTPUT",
        value:
          "Error: " +
          (error instanceof Error ? error.message : "Unknown error"),
      });
    }
  }, [state.nodes, state.edges, state.selectedNodeId]);

  const debouncedCalculate = useMemo(
    () => debounce(calculate, 250),
    [calculate]
  );

  useEffect(() => {
    if (!autoCalculate) return;
    debouncedCalculate();
    return () => {
      debouncedCalculate.cancel();
    };
  }, [autoCalculate, debouncedCalculate]);

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

  // Memoize props
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
  );
};

export default React.memo(Workspace);
