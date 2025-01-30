import {
  Reverse,
  RemoveSpaces,
  Lowercase,
  Uppercase,
  Base64Encode,
  Base64Decode,
  Base16Encode,
  Base16Decode,
  Base32Encode,
  Base32Decode,
  Base85Encode,
  Base85Decode,
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
  Base64Encode,
  Base64Decode,
  Base85Encode,
  Base85Decode,
];
