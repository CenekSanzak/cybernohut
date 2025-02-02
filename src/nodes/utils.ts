import { Node } from "@xyflow/react";

export const checkInputNodeExists = (nodes: Node[]) => {
  return nodes.some((node) => node.data?.id === "input");
};
