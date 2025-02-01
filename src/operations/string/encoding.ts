import { Operation, OperationTags, IOTypes } from "../types";
import {
  encode_base16,
  decode_base16,
  encode_base32,
  decode_base32,
  encode_base85,
  decode_base85,
  encode_base64_standard,
  decode_base64_standard,
  encode_base64_url,
  decode_base64_url,
} from "wasm";
import { validateOutputTypeStringToString } from "./utils";

export const Base16Encode: Operation = {
  name: "Hexadecimal Encode (Base16)",
  id: "base16-encode",
  description: "Encodes the input string to Base16 (hexadecimal)",
  link: "https://en.wikipedia.org/wiki/Hexadecimal",
  value: "",
  func: validateOutputTypeStringToString(
    encode_base16,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.NumberBase, OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base16Decode: Operation = {
  name: "Hexadecimal Decode (Base16)",
  id: "base16-decode",
  description: "Decodes a Base16 (hexadecimal) encoded string",
  link: "https://en.wikipedia.org/wiki/Hexadecimal",
  value: "",
  func: validateOutputTypeStringToString(
    decode_base16,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.NumberBase, OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base32Encode: Operation = {
  name: "Base32 Encode",
  id: "base32-encode",
  description: "Encodes the input string to Base32",
  link: "https://en.wikipedia.org/wiki/Base32",
  value: "",
  func: validateOutputTypeStringToString(
    encode_base32,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base32Decode: Operation = {
  name: "Base32 Decode",
  id: "base32-decode",
  description: "Decodes a Base32 encoded string",
  link: "https://en.wikipedia.org/wiki/Base32",
  value: "",
  func: validateOutputTypeStringToString(
    decode_base32,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base85Encode: Operation = {
  name: "Base85 Encode",
  id: "base85-encode",
  description: "Encodes the input string to Base85",
  link: "https://en.wikipedia.org/wiki/Ascii85",
  value: "",
  func: validateOutputTypeStringToString(
    encode_base85,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base85Decode: Operation = {
  name: "Base85 Decode",
  id: "base85-decode",
  description: "Decodes a Base85 encoded string",
  link: "https://en.wikipedia.org/wiki/Ascii85",
  value: "",
  func: validateOutputTypeStringToString(
    decode_base85,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64StandardEncode: Operation = {
  name: "Base64 Standard Encode",
  id: "base64-standard-encode",
  description: "Encodes the input string to Base64 (standard)",
  link: "https://en.wikipedia.org/wiki/Base64",
  value: "",
  func: validateOutputTypeStringToString(
    encode_base64_standard,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64StandardDecode: Operation = {
  name: "Base64 Standard Decode",
  id: "base64-standard-decode",
  description: "Decodes a Base64 encoded string (standard)",
  link: "https://en.wikipedia.org/wiki/Base64",
  value: "",
  func: validateOutputTypeStringToString(
    decode_base64_standard,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64UrlEncode: Operation = {
  name: "Base64 URL-safe Encode",
  id: "base64-url-encode",
  description: "Encodes the input string to Base64 (URL-safe)",
  link: "https://en.wikipedia.org/wiki/Base64#URL_applications",
  value: "",
  func: validateOutputTypeStringToString(
    encode_base64_url,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64UrlDecode: Operation = {
  name: "Base64 URL-safe Decode",
  id: "base64-url-decode",
  description: "Decodes a Base64 encoded string (URL-safe)",
  link: "https://en.wikipedia.org/wiki/Base64#URL_applications",
  value: "",
  func: validateOutputTypeStringToString(
    decode_base64_url,
    [IOTypes.Text],
    [IOTypes.Text]
  ),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
