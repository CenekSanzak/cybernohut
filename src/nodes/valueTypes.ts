import type { ValueType } from "@/engine/wasm";

/** The colour that stands for each value type on ports and labels. */
export const typeColor: Record<ValueType, string> = {
  text: "#2563eb",
  bytes: "#ea580c",
  number: "#16a34a",
  text_list: "#9333ea",
  number_list: "#9333ea",
};

export const typeLabel: Record<ValueType, string> = {
  text: "text",
  bytes: "bytes",
  number: "number",
  text_list: "text list",
  number_list: "number list",
};
