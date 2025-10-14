import { Operation, OperationTags, IOTypes } from "../types";
import {
  reverse_string,
  uppercase_string,
  lowercase_string,
  remove_whitespace,
} from "wasm";
import { wasmTextToText } from "../wasm-adapter";

export const Reverse: Operation = {
  name: "Reverse",
  id: "reverse",
  description: "Reverses the input string",
  value: "",
  func: wasmTextToText(reverse_string),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Uppercase: Operation = {
  name: "To Uppercase",
  id: "to-uppercase",
  description: "Converts the input string to uppercase",
  value: "",
  func: wasmTextToText(uppercase_string),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Lowercase: Operation = {
  name: "To Lowercase",
  id: "to-lowercase",
  description: "Converts the input string to lowercase",
  value: "",
  func: wasmTextToText(lowercase_string),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const RemoveSpaces: Operation = {
  name: "Remove Spaces",
  id: "remove-spaces",
  description: "Removes all whitespace from the input string",
  value: "",
  func: wasmTextToText(remove_whitespace),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
