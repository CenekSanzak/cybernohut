import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_slice_text } from "wasm";
import { validateOutputTypeStringToString } from "./utils";

export const SliceText: Operation = {
  name: "Slice Text",
  id: "slice-text",
  description: "Extract a portion of text using start and end indices",
  value: "",
  configValues: {
    start: 0,
    end: 0,
  },
  funcBuilder: (config) => {
    const builtFunction = build_slice_text(JSON.stringify(config || {}));
    return validateOutputTypeStringToString(
      (input: string) => builtFunction(input),
      [IOTypes.Text],
      [IOTypes.Text]
    );
  },
  func: validateOutputTypeStringToString(
    (input: string) => build_slice_text(JSON.stringify({}))(input),
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
