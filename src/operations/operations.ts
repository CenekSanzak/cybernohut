import {
  Reverse,
  RemoveSpaces,
  Lowercase,
  Uppercase,
  Base16Encode,
  Base16Decode,
  Base32Encode,
  Base32Decode,
  Base85Encode,
  Base85Decode,
  Base64StandardEncode,
  Base64StandardDecode,
  Base64UrlEncode,
  Base64UrlDecode,
} from "./string-operations";
import { Operation } from "./types";

export const operations: Operation[] = [
  Reverse,
  RemoveSpaces,
  Lowercase,
  Uppercase,
  Base16Encode,
  Base16Decode,
  Base32Encode,
  Base32Decode,
  Base85Encode,
  Base85Decode,
  Base64StandardEncode,
  Base64StandardDecode,
  Base64UrlEncode,
  Base64UrlDecode,
];
