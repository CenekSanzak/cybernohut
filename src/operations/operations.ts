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
  SliceText,
} from "./string";
import {
  CountWords,
  SplitText,
  JoinText,
  ConcatenateText,
  TextStats,
} from "./string/advanced";
import { Input } from "./io";
import { catalogOperations } from "./from-catalog";
import { Operation } from "./types";

export const operations: Operation[] = [
  Input,
  // Defined in Rust; see wasm/src/ops.
  ...catalogOperations,
  // Still hand-written in TypeScript, waiting to be moved.
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
  SliceText,
  CountWords,
  SplitText,
  JoinText,
  ConcatenateText,
  TextStats,
];
