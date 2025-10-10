import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_caesar_cipher } from "wasm";
import { wasmTextToTextWithConfig } from "@/operations/wasm-adapter";

export const ROT13: Operation = {
  name: "ROT13",
  id: "rot13",
  description:
    "Apply ROT13 substitution cipher (Caesar cipher with shift of 13)",
  value: "",
  func: wasmTextToTextWithConfig(
    build_caesar_cipher as (config: string) => (input: string) => string[],
    { shift: 13 },
    "text"
  ),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [
    OperationTags.Text,
    OperationTags.All,
    OperationTags.SubstitutionCipher,
  ],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
