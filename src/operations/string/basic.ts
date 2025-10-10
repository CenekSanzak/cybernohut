import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_slice_text } from "wasm";
import { wasmTextToTextWithConfig } from "@/operations/wasm-adapter";

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
    return wasmTextToTextWithConfig(
      build_slice_text as (config: string) => (input: string) => string[],
      config || {},
      "text"
    );
  },
  func: wasmTextToTextWithConfig(
    build_slice_text as (config: string) => (input: string) => string[],
    {},
    "text"
  ),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  cleanup: () => {},
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
