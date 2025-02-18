import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { build_caesar_cipher } from "wasm";
import { validateOutputTypeStringToString } from "../string/utils";

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
    const builtFunction = build_caesar_cipher(JSON.stringify(config || {}));
    return validateOutputTypeStringToString(
      (input: string) => builtFunction(input),
      [IOTypes.Text],
      [IOTypes.Text]
    );
  },
  func: validateOutputTypeStringToString(
    (input: string) => build_caesar_cipher(JSON.stringify({}))(input),
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
