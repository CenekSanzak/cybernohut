import React, { useCallback } from "react";
import {
  Node,
  Edge,
  OnNodesChange,
  OnEdgesChange,
  ReactFlow,
  Connection,
  Background,
  Controls,
  MiniMap,
  BackgroundVariant,
  IsValidConnection,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { CustomNode } from "@/nodes/customNode";

interface FlowGraphProps {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: (params: Connection) => void;
  onNodeClick?: (nodeId: string) => void;
  isValidConnection: IsValidConnection<Edge>;
  onNodesDelete?: (
    deleted: Node[],
    remainingNodes: Node[],
    newEdges: Edge[]
  ) => void;
}

const nodeTypes = { custom: CustomNode };

export const FlowGraph = React.memo<FlowGraphProps>(
  ({
    nodes,
    edges,
    onNodesChange,
    onEdgesChange,
    onConnect,
    onNodeClick,
    isValidConnection,
    onNodesDelete,
  }) => {
    const handleNodesDelete = useCallback(
      (deleted: Node[]) => {
        const remainingNodes = nodes.filter(
          (node) => !deleted.some((del) => del.id === node.id)
        );
        const newEdges = edges.filter(
          (edge) =>
            !deleted.some(
              (node) => node.id === edge.source || node.id === edge.target
            )
        );
        onNodesDelete?.(deleted, remainingNodes, newEdges);
      },
      [nodes, edges, onNodesDelete]
    );

    return (
      <div className="h-full w-full">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          onConnect={onConnect}
          isValidConnection={isValidConnection}
          onNodesDelete={handleNodesDelete}
          onNodeClick={(_, node) => onNodeClick?.(node.id)}
          fitView
        >
          <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
          <Controls />
          <MiniMap />
        </ReactFlow>
      </div>
    );
  }
);

FlowGraph.displayName = "FlowGraph";
