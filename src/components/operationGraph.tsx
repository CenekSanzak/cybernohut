import React, { useCallback } from "react";
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
  getConnectedEdges,
  getIncomers,
  getOutgoers,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { CustomNode } from "@/nodes/customNode";
import { Operation } from "@/operations/types";

type OperationGraphProps = {
  nodes: Node[];
  edges: Edge[];
  setEdges: React.Dispatch<React.SetStateAction<Edge[]>>;
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
const nodeTypes = { custom: CustomNode };

const OperationGraph: React.FC<OperationGraphProps> = ({
  nodes,
  edges,
  setEdges,
  onNodesChange,
  onEdgesChange,
  onConnect,
  onNodeClick,
  onNodesDelete,
}) => {
  const onNodesDeleteInternal = useCallback(
    (deleted: Node[]) => {
      const remainingNodes = nodes.filter(
        (node) => !deleted.some((del) => del.id === node.id)
      );

      const newEdges = deleted.reduce((acc, node) => {
        const incomers = getIncomers(node, nodes, edges);
        const outgoers = getOutgoers(node, nodes, edges);
        const connectedEdges = getConnectedEdges([node], edges);

        const remainingEdges = acc.filter(
          (edge) => !connectedEdges.includes(edge)
        );

        const createdEdges = incomers.flatMap(({ id: source }) =>
          outgoers.map(({ id: target }) => ({
            id: `${source}->${target}`,
            source,
            target,
          }))
        );

        return [...remainingEdges, ...createdEdges];
      }, edges);

      setEdges(newEdges);
      onNodesDelete?.(deleted, remainingNodes, newEdges);
    },
    [setEdges, edges, nodes, onNodesDelete]
  );

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

  const handleNodeClick = useCallback(
    (event: React.MouseEvent, node: Node) => {
      const nodeData = node.data as Operation;
      onNodeClick?.(nodeData.value?.toString() || "", node.id);
    },
    [onNodeClick]
  );

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
          onNodesDelete={onNodesDeleteInternal}
          onNodeClick={handleNodeClick}
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
