"use client";

import type React from "react";
import { useState, useEffect, useCallback } from "react";
import Sidebar from "@/components/Sidebar";
import InputOutput from "@/components/InputOutput";
import OperationGraph from "@/components/operationGraph";
import {
  IOTypes,
  Operation,
  OperationTags,
  outputTypes,
} from "@/operations/types";
import { operations } from "@/operations/operations";
import {
  Edge,
  useNodesState,
  useEdgesState,
  Connection,
  addEdge,
  Node,
} from "@xyflow/react";
import { topologicalSort } from "@/nodes/topological-sort";
import { checkInputNodeExists } from "@/nodes/utils";

const initialNodes: Node[] = [
  {
    id: "input-node",
    type: "custom",
    data: {
      description: "",
      name: "Input",
      id: "input",
      tags: [OperationTags.Text, OperationTags.All],
      outputs: { output: IOTypes.Text },
    },
    position: { x: 250, y: 10 },
  },
];

const initialEdges: Edge[] = [];

const generateShortId = (prefix: string) =>
  `${prefix}-${Math.random().toString(36).slice(2, 7)}`;

const Workspace: React.FC = () => {
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  const [selectedNodeId, setSelectedNodeId] = useState<string>("");
  const [selectedNodeTitle, setSelectedNodeTitle] = useState<string>("");

  const [autoCalculate, setAutoCalculate] = useState(true);
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const handleInputChange = (value: string) => {
    setInput(value);
    setNodes((prevNodes) => {
      prevNodes[0] = {
        ...prevNodes[0],
        data: { ...prevNodes[0].data, value, outputValues: { output: value } },
      };
      return [...prevNodes];
    });
  };

  const handleOperationSelect = (operation: Operation) => {
    const max_y = Math.max(
      ...nodes.map((node) => node.position.y + (node.measured?.height || 0))
    );
    const avg_x =
      nodes.reduce((acc, node) => acc + node.position.x, 0) / nodes.length;

    const newNode: Node = {
      id: generateShortId(operation.id),
      type: "custom",
      data: operation,
      position: { x: avg_x, y: 80 + max_y },
    };
    const newNodes = [...nodes, newNode];
    setNodes(newNodes);
    setSelectedNodeId(newNode.id);
    setSelectedNodeTitle(operation.name);
    if (nodes.length > 0) {
      const newEdge: Edge = {
        id: `${nodes.length}-${newNode.id}`,
        source: nodes[nodes.length - 1].id,
        sourceHandle: "output",
        target: newNode.id,
        targetHandle: "input",
      };
      setEdges([...edges, newEdge]);
    }
  };

  const calculate = useCallback(async () => {
    try {
      if (!checkInputNodeExists(nodes)) {
        setOutput("Error: Input node not found");
        return;
      }
      const sortedNodes = topologicalSort(nodes, edges);
      let lastValue: outputTypes = "";
      const newNodes = [...nodes];

      for (const node of sortedNodes) {
        if (!node.data) continue;
        const operation = node.data as Operation;
        if (!operation.inputs || Object.keys(operation.inputs).length === 0)
          continue;
        const inputValues: outputTypes[] = edges
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
          newNodes[nodeIndex] = {
            ...node,
            data: {
              ...node.data,
              value: calculated.toString(),
              outputValues: newOutputValues,
            },
          };
        }
      }
      const selectedNode = newNodes.find((n) => n.id === selectedNodeId);
      if (selectedNode) {
        const selectedData = selectedNode.data as Operation;
        setOutput(selectedData.value?.toString() || "");
      } else if (lastValue !== undefined) {
        setOutput("" + lastValue);
      }
      setNodes(newNodes);
    } catch (error) {
      console.error("Error during calculation:", error);
      setOutput(
        "Error: " + (error instanceof Error ? error.message : "Unknown error")
      );
    }
  }, [edges, nodes, setNodes, selectedNodeId]);

  useEffect(() => {
    if (!autoCalculate) return;
    calculate();
  }, [autoCalculate, calculate]);

  const handleNodeClick = useCallback(
    (value: string, nodeId: string) => {
      setOutput(value);
      setSelectedNodeId(nodeId);
      const node = nodes.find((n) => n.id === nodeId);
      if (node) {
        const nodeData = node.data as Operation;
        setSelectedNodeTitle(nodeData.name);
      }
    },
    [nodes]
  );

  const handleNodesDelete = useCallback(
    (deleted: Node[], remainingNodes: Node[], newEdges: Edge[]) => {
      if (deleted.some((node) => node.id === selectedNodeId)) {
        if (remainingNodes.length > 0) {
          try {
            const sortedNodes = topologicalSort(remainingNodes, newEdges);
            const lastNode = sortedNodes[sortedNodes.length - 1];

            if (lastNode) {
              const nodeData = lastNode.data as Operation;
              setSelectedNodeId(lastNode.id);
              setSelectedNodeTitle(nodeData.name);
              setOutput(nodeData.value?.toString() || "");
            } else {
              setSelectedNodeId("");
              setSelectedNodeTitle("");
              setOutput("");
            }
          } catch (error) {
            console.error("Error in topological sort:", error);
            const lastNode = remainingNodes[remainingNodes.length - 1];
            const nodeData = lastNode.data as Operation;
            setSelectedNodeId(lastNode.id);
            setSelectedNodeTitle(nodeData.name);
            setOutput(nodeData.value?.toString() || "");
          }
        } else {
          setSelectedNodeId("");
          setSelectedNodeTitle("");
          setOutput("");
        }
      }
    },
    [selectedNodeId]
  );

  return (
    <div className="flex min-h-screen">
      <Sidebar
        operations={operations}
        onOperationSelect={handleOperationSelect}
      />
      <div className="flex-1 flex">
        <div className="w-2/3 p-4 border-r">
          <OperationGraph
            setEdges={setEdges}
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={handleNodeClick}
            onNodesDelete={handleNodesDelete}
          />
        </div>
        <div className="w-1/3 flex flex-col">
          <InputOutput
            autoCalculate={autoCalculate}
            onAutoCalculateChange={setAutoCalculate}
            calculate={calculate}
            input={input}
            output={output}
            onInputChange={handleInputChange}
            selectedNodeId={selectedNodeId}
            selectedNodeTitle={selectedNodeTitle}
          />
        </div>
      </div>
    </div>
  );
};

export default Workspace;
