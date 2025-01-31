import { Operation, OperationTags, IOTypes, outputTypes } from "./types";
import {
  reverse_string,
  uppercase_string,
  lowercase_string,
  remove_whitespace,
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
  calculate_md5,
  calculate_sha1,
  calculate_sha256,
} from "wasm";

const validateOutputTypeStringToString =
  (func: (s: string) => string) =>
  (...args: outputTypes[]) => {
    try {
      if (args.length !== 1) {
        throw new Error("Incorrect number of arguments");
      }
      return func(args[0] as string);
    } catch (e) {
      console.error(e);
      return "";
    }
  };

export const Reverse: Operation = {
  name: "Reverse",
  id: "reverse",
  description: "Reverses the input string",
  value: "",
  func: validateOutputTypeStringToString(reverse_string),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Uppercase: Operation = {
  name: "To Uppercase",
  id: "to-uppercase",
  description: "Converts the input string to uppercase",
  value: "",
  func: validateOutputTypeStringToString(uppercase_string),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Lowercase: Operation = {
  name: "To Lowercase",
  id: "to-lowercase",
  description: "Converts the input string to lowercase",
  value: "",
  func: validateOutputTypeStringToString(lowercase_string),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const RemoveSpaces: Operation = {
  name: "Remove Spaces",
  id: "remove-spaces",
  description: "Removes all whitespace from the input string",
  value: "",
  func: validateOutputTypeStringToString(remove_whitespace),
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base16Encode: Operation = {
  name: "Base16 (Hex) Encode",
  id: "base16-encode",
  description: "Encodes the input string to Base16 (hexadecimal)",
  value: "",
  func: validateOutputTypeStringToString(encode_base16),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base16Decode: Operation = {
  name: "Base16 (Hex) Decode",
  id: "base16-decode",
  description: "Decodes a Base16 (hexadecimal) encoded string",
  value: "",
  func: validateOutputTypeStringToString(decode_base16),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base32Encode: Operation = {
  name: "Base32 Encode",
  id: "base32-encode",
  description: "Encodes the input string to Base32",
  value: "",
  func: validateOutputTypeStringToString(encode_base32),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base32Decode: Operation = {
  name: "Base32 Decode",
  id: "base32-decode",
  description: "Decodes a Base32 encoded string",
  value: "",
  func: validateOutputTypeStringToString(decode_base32),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base85Encode: Operation = {
  name: "Base85 Encode",
  id: "base85-encode",
  description: "Encodes the input string to Base85",
  value: "",
  func: validateOutputTypeStringToString(encode_base85),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base85Decode: Operation = {
  name: "Base85 Decode",
  id: "base85-decode",
  description: "Decodes a Base85 encoded string",
  value: "",
  func: validateOutputTypeStringToString(decode_base85),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64StandardEncode: Operation = {
  name: "Base64 Standard Encode",
  id: "base64-standard-encode",
  description: "Encodes the input string to Base64 (standard)",
  value: "",
  func: validateOutputTypeStringToString(encode_base64_standard),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64StandardDecode: Operation = {
  name: "Base64 Standard Decode",
  id: "base64-standard-decode",
  description: "Decodes a Base64 encoded string (standard)",
  value: "",
  func: validateOutputTypeStringToString(decode_base64_standard),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64UrlEncode: Operation = {
  name: "Base64 URL-safe Encode",
  id: "base64-url-encode",
  description: "Encodes the input string to Base64 (URL-safe)",
  value: "",
  func: validateOutputTypeStringToString(encode_base64_url),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Base64UrlDecode: Operation = {
  name: "Base64 URL-safe Decode",
  id: "base64-url-decode",
  description: "Decodes a Base64 encoded string (URL-safe)",
  value: "",
  func: validateOutputTypeStringToString(decode_base64_url),
  tags: [OperationTags.Encoding, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Md5: Operation = {
  name: "MD5",
  id: "md5",
  description: "Hashes the input string using MD5",
  value: "",
  func: validateOutputTypeStringToString(calculate_md5),
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Sha1: Operation = {
  name: "SHA1",
  id: "sha1",
  description: "Hashes the input string using SHA1",
  value: "",
  func: validateOutputTypeStringToString(calculate_sha1),
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

export const Sha256: Operation = {
  name: "SHA256",
  id: "sha256",
  description: "Hashes the input string using SHA256",
  value: "",
  func: validateOutputTypeStringToString(calculate_sha256),
  tags: [OperationTags.Hashing, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};
