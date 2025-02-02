import React, { useCallback } from "react";
import {
  Edge,
  Node,
  OnNodesChange,
  OnEdgesChange,
  Connection,
  IsValidConnection,
} from "@xyflow/react";
import { Operation } from "@/operations/types";
import { FlowGraph } from "./flow/FlowGraph";

type OperationGraphProps = {
  nodes: Node[];
  edges: Edge[];
  setEdges: (edges: Edge[]) => void;
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: (params: Connection) => void;
  onNodeClick?: (value: string, nodeId: string) => void;
  onNodesDelete?: (
    deleted: Node[],
    remainingNodes: Node[],
    newEdges: Edge[]
  ) => void;
};

const OperationGraph = React.memo<OperationGraphProps>(
  ({
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    onNodeClick,
    onNodesDelete,
  }) => {
    const isValidConnection = useCallback<IsValidConnection<Edge>>(
      (connection) => {
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
      },
      [edges, nodes]
    );

    const handleNodeClick = useCallback(
      (nodeId: string) => {
        const node = nodes.find((n) => n.id === nodeId);
        if (!node) return;
        const nodeData = node.data as Operation;
        onNodeClick?.(nodeData.value?.toString() || "", nodeId);
      },
      [nodes, onNodeClick]
    );

    return (
      <div className="h-full overflow-y-auto">
        <h2 className="text-xl font-bold mb-4">Selected Operations</h2>
        <div className="space-y-2 w-full h-full">
          <FlowGraph
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            onNodeClick={handleNodeClick}
            onNodesDelete={onNodesDelete}
          />
        </div>
      </div>
    );
  }
);

OperationGraph.displayName = "OperationGraph";

export default OperationGraph;
