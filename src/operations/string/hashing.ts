import { Operation, OperationTags, IOTypes } from "@/operations/types";
import { calculate_md5, calculate_sha1, calculate_sha256 } from "wasm";
import { wasmTextToText } from "@/operations/wasm-adapter";

export const Md5: Operation = {
  name: "MD5",
  id: "md5",
  description: "Hashes the input string using MD5",
  link: "https://en.wikipedia.org/wiki/MD5",
  value: "",
  func: wasmTextToText(calculate_md5),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Sha1: Operation = {
  name: "SHA1",
  id: "sha1",
  description: "Hashes the input string using SHA1",
  link: "https://en.wikipedia.org/wiki/SHA-1",
  value: "",
  func: wasmTextToText(calculate_sha1),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Sha256: Operation = {
  name: "SHA256",
  id: "sha256",
  description: "Hashes the input string using SHA256",
  link: "https://en.wikipedia.org/wiki/SHA-2",
  value: "",
  func: wasmTextToText(calculate_sha256),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
