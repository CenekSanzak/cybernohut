import { Node, Edge } from "@xyflow/react";

export const topologicalSort = (nodes: Node[], edges: Edge[]): Node[] => {
  const inDegree = new Map<string, number>();
  const adjacencyList = new Map<string, string[]>();

  nodes.forEach((node) => {
    inDegree.set(node.id, 0);
    adjacencyList.set(node.id, []);
  });

  edges.forEach((edge) => {
    const target = edge.target;
    inDegree.set(target, (inDegree.get(target) || 0) + 1);
    adjacencyList.get(edge.source)?.push(target);
  });

  const queue: string[] = [];
  inDegree.forEach((degree, nodeId) => {
    if (degree === 0) queue.push(nodeId);
  });

  const sortedNodes: Node[] = [];

  while (queue.length > 0) {
    const nodeId = queue.shift()!;
    const node = nodes.find((n) => n.id === nodeId);
    if (node) {
      sortedNodes.push(node);
    }
    adjacencyList.get(nodeId)?.forEach((adjacentId) => {
      inDegree.set(adjacentId, (inDegree.get(adjacentId) || 0) - 1);
      if (inDegree.get(adjacentId) === 0) {
        queue.push(adjacentId);
      }
    });
  }

  if (sortedNodes.length !== nodes.length) {
    throw new Error("Graph contains a cycle");
  }

  return sortedNodes;
};

export const getConnectedNodesFromInput = (
  startNodeId: string,
  nodes: Node[],
  edges: Edge[]
): Node[] => {
  const visited = new Set<string>();
  const connectedNodes: Node[] = [];

  const dfs = (nodeId: string) => {
    if (visited.has(nodeId)) return;
    visited.add(nodeId);

    const node = nodes.find((n) => n.id === nodeId);
    if (node) {
      connectedNodes.push(node);
      const outgoingEdges = edges.filter((edge) => edge.source === nodeId);
      outgoingEdges.forEach((edge) => dfs(edge.target));
    }
  };

  dfs(startNodeId);
  return connectedNodes;
};
