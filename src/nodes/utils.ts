import { Node, Edge } from "@xyflow/react";

export const checkInputNodeExists = (nodes: Node[]) => {
  return nodes.some((node) => node.data?.id === "input");
};

export function getDownstreamNodes(
  edges: Edge[],
  startNodeId: string
): string[] {
  const downstream = new Set<string>();

  function traverse(nodeId: string) {
    const outgoingEdges = edges.filter((edge) => edge.source === nodeId);
    for (const edge of outgoingEdges) {
      if (!downstream.has(edge.target)) {
        downstream.add(edge.target);
        traverse(edge.target);
      }
    }
  }

  traverse(startNodeId);
  return Array.from(downstream);
}

export function getAffectedNodesFromEdgeChanges(
  oldEdges: Edge[],
  newEdges: Edge[]
): string[] {
  const affected = new Set<string>();

  const edgeDiff = new Set([
    ...oldEdges.filter((e) => !newEdges.find((ne) => ne.id === e.id)),
    ...newEdges.filter((e) => !oldEdges.find((oe) => oe.id === e.id)),
  ]);

  for (const edge of edgeDiff) {
    affected.add(edge.target);
    const downstream = getDownstreamNodes(newEdges, edge.target);
    downstream.forEach((id) => affected.add(id));
  }

  return Array.from(affected);
}
