import React from "react";
import {
  ReactFlow,
  Edge,
  Node,
  Background,
  Controls,
  MiniMap,
  OnNodesChange,
  BackgroundVariant,
  Connection,
  OnEdgesChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { CustomNode } from "../nodes/customNode";
import { Operation } from "../operations/types";

type OperationGraphProps = {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: (params: Connection) => void;
};
const nodeTypes = { custom: CustomNode };

const OperationGraph: React.FC<OperationGraphProps> = ({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onConnect,
}) => {
  const isValidConnection = (connection: Edge | Connection) => {
    if (
      !connection.source ||
      !connection.target ||
      !connection.sourceHandle ||
      !connection.targetHandle ||
      (connection.source === connection.target &&
        connection.sourceHandle === connection.targetHandle)
    )
      return false;

    const targetHandleHasConnection = edges.some(
      (edge) =>
        edge.target === connection.target &&
        edge.targetHandle === connection.targetHandle
    );

    if (targetHandleHasConnection) return false;

    const source = nodes.find((node) => node.id === connection.source);
    const target = nodes.find((node) => node.id === connection.target);
    const sourceData = source?.data as Operation;
    const targetData = target?.data as Operation;
    const sourceHandle = sourceData.outputs?.[connection.sourceHandle];
    const targetHandle = targetData.inputs?.[connection.targetHandle];
    return sourceHandle === targetHandle;
  };
  return (
    <div className="h-full overflow-y-auto">
      <h2 className="text-xl font-bold mb-4">Selected Operations</h2>
      <div className="space-y-2 w-full h-full">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          onConnect={onConnect}
          isValidConnection={isValidConnection}
        >
          <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>
    </div>
  );
};

export default OperationGraph;
