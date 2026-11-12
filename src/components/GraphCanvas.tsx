import React, { useCallback, useContext, useEffect, useRef, useState } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useNodesInitialized,
  useReactFlow,
  type Connection,
  type Edge,
  type EdgeChange,
  type IsValidConnection,
  type NodeChange,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { GraphDispatchContext } from "@/graph/context";
import { loadFile } from "@/graph/files";
import { newFileNodeId, type GraphNode } from "@/graph/model";
import { MemoizedCustomNode } from "@/nodes/customNode";
import { isConnectable } from "@/state/graphReducer";

const nodeTypes = { custom: MemoizedCustomNode };

interface GraphCanvasProps {
  nodes: GraphNode[];
  edges: Edge[];
}

const FIT = { maxZoom: 1, padding: 0.2 };

function carriesFiles(event: React.DragEvent): boolean {
  return event.dataTransfer.types.includes("Files");
}

const Canvas = ({ nodes, edges }: GraphCanvasProps) => {
  const dispatch = useContext(GraphDispatchContext);
  const { fitView, screenToFlowPosition } = useReactFlow();
  const [dropping, setDropping] = useState(false);

  // Several nodes arriving at once (a proposed chain, an imported graph) usually land
  // partly outside the view, so the view follows them. A single new node is left
  // alone: moving the view on every click would be disorienting.
  const initialized = useNodesInitialized();
  const previousCount = useRef(nodes.length);
  const fitPending = useRef(false);
  useEffect(() => {
    if (nodes.length - previousCount.current >= 2) fitPending.current = true;
    previousCount.current = nodes.length;
    // New nodes can only be fitted once they have been measured.
    if (fitPending.current && initialized) {
      fitPending.current = false;
      void fitView(FIT);
    }
  }, [nodes.length, initialized, fitView]);

  const handleDrop = (event: React.DragEvent) => {
    if (!carriesFiles(event)) return;
    event.preventDefault();
    setDropping(false);
    const origin = screenToFlowPosition({ x: event.clientX, y: event.clientY });
    Array.from(event.dataTransfer.files).forEach((file, index) => {
      const nodeId = newFileNodeId();
      dispatch({
        type: "ADD_FILE",
        nodeId,
        position: { x: origin.x + index * 260, y: origin.y },
      });
      void loadFile(nodeId, file, dispatch);
    });
  };

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
    <div
      className={`h-full ${dropping ? "outline outline-2 -outline-offset-2 outline-blue-500" : ""}`}
      aria-label="Operation graph"
      onDragOver={(event) => {
        if (!carriesFiles(event)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
        setDropping(true);
      }}
      onDragLeave={() => setDropping(false)}
      onDrop={handleDrop}
    >
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
          fitViewOptions={FIT}
          defaultEdgeOptions={{ style: { strokeWidth: 1.5 } }}
          proOptions={{ hideAttribution: true }}
        >
          <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
          <Controls />
          <MiniMap className="max-lg:hidden" pannable zoomable nodeColor="#94a3b8" nodeStrokeWidth={0} />
        </ReactFlow>
    </div>
  );
};

export const GraphCanvas = React.memo<GraphCanvasProps>((props) => (
  <ReactFlowProvider>
    <Canvas {...props} />
  </ReactFlowProvider>
));

GraphCanvas.displayName = "GraphCanvas";
