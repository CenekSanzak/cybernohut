import { Operation, OperationTags, IOTypes } from "../types";
import {
  reverse_string,
  uppercase_string,
  lowercase_string,
  remove_whitespace,
} from "wasm";
import { validateOutputTypeStringToString } from "./utils";

export const Reverse: Operation = {
  name: "Reverse",
  id: "reverse",
  description: "Reverses the input string",
  value: "",
  func: validateOutputTypeStringToString(
    reverse_string,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Uppercase: Operation = {
  name: "To Uppercase",
  id: "to-uppercase",
  description: "Converts the input string to uppercase",
  value: "",
  func: validateOutputTypeStringToString(
    uppercase_string,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Lowercase: Operation = {
  name: "To Lowercase",
  id: "to-lowercase",
  description: "Converts the input string to lowercase",
  value: "",
  func: validateOutputTypeStringToString(
    lowercase_string,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const RemoveSpaces: Operation = {
  name: "Remove Spaces",
  id: "remove-spaces",
  description: "Removes all whitespace from the input string",
  value: "",
  func: validateOutputTypeStringToString(
    remove_whitespace,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
