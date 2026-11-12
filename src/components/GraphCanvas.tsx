import React, { useCallback, useContext } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  type Connection,
  type Edge,
  type EdgeChange,
  type IsValidConnection,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { GraphDispatchContext } from "@/graph/context";
import type { GraphNode } from "@/graph/model";
import { MemoizedCustomNode } from "@/nodes/customNode";
import { isConnectable } from "@/state/graphReducer";

const nodeTypes = { custom: MemoizedCustomNode };

interface GraphCanvasProps {
  nodes: GraphNode[];
  edges: Edge[];
}

export const GraphCanvas = React.memo<GraphCanvasProps>(({ nodes, edges }) => {
  const dispatch = useContext(GraphDispatchContext);

  const isValidConnection = useCallback<IsValidConnection<Edge>>(
    ({ source, target, sourceHandle, targetHandle }) => {
      if (!sourceHandle || !targetHandle || source === target) return false;
      const occupied = edges.some(
        (edge) => edge.target === target && edge.targetHandle === targetHandle
      );
      if (occupied) return false;
      const from = nodes.find((node) => node.id === source);
      const to = nodes.find((node) => node.id === target);
      return !!from && !!to && isConnectable(from.data, sourceHandle, to.data, targetHandle);
    },
    [nodes, edges]
  );

  return (
    <div className="h-full" aria-label="Operation graph">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={(changes: NodeChange<GraphNode>[]) =>
            dispatch({ type: "APPLY_NODE_CHANGES", changes })
          }
          onEdgesChange={(changes: EdgeChange[]) =>
            dispatch({ type: "APPLY_EDGE_CHANGES", changes })
          }
          onConnect={(connection: Connection) => dispatch({ type: "CONNECT", connection })}
          onNodeClick={(_, node) => dispatch({ type: "SELECT", nodeId: node.id })}
          isValidConnection={isValidConnection}
          fitView
          fitViewOptions={{ maxZoom: 1, padding: 0.2 }}
          defaultEdgeOptions={{ style: { strokeWidth: 1.5 } }}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
          <Controls />
          <MiniMap className="max-lg:hidden" pannable zoomable nodeColor="#94a3b8" nodeStrokeWidth={0} />
        </ReactFlow>
    </div>
  );
});

GraphCanvas.displayName = "GraphCanvas";
