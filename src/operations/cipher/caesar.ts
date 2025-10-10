import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_caesar_cipher } from "wasm";
import { wasmTextToTextWithConfig } from "@/operations/wasm-adapter";

export const CaesarCipher: Operation = {
  name: "Caesar Cipher",
  id: "caesar-cipher",
  description: "Apply Caesar cipher encryption/decryption",
  value: "",
  configValues: {
    shift: 0,
    knownPart: "",
  },
  funcBuilder: (config) => {
    return wasmTextToTextWithConfig(
      build_caesar_cipher as (config: string) => (input: string) => string[],
      config || {},
      "text"
    );
  },
  func: wasmTextToTextWithConfig(
    build_caesar_cipher as (config: string) => (input: string) => string[],
    {},
    "text"
  ),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  cleanup: () => {},
  tags: [
    OperationTags.Text,
    OperationTags.All,
    OperationTags.SubstitutionCipher,
  ],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
