import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_caesar_cipher } from "wasm";
import { validateOutputTypeStringToString } from "../string/utils";

export const ROT13: Operation = {
  name: "ROT13",
  id: "rot13",
  description:
    "Apply ROT13 substitution cipher (Caesar cipher with shift of 13)",
  value: "",
  func: validateOutputTypeStringToString(
    (input: string) =>
      build_caesar_cipher(JSON.stringify({ shift: 13 }))(input),
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [
    OperationTags.Text,
    OperationTags.All,
    OperationTags.SubstitutionCipher,
  ],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
