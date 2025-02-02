import { Operation, OperationTags, IOTypes } from "../types";

export const Input: Operation = {
  name: "Input",
  description: "Input node for providing data",
  id: "input",
  value: "",
  func: () => [],
  tags: [OperationTags.IO, OperationTags.All],
  inputs: {},
  outputs: { output: IOTypes.Text },
};
