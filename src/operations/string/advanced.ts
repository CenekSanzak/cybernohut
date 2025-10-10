import {
  Operation,
  OperationTags,
  IOTypes,
  DataValue,
} from "@/operations/types";
import { tsOperation, tsMultiInputOperation } from "@/operations/wasm-adapter";

/**
 * Count words in text - demonstrates text → number operation
 */
export const CountWords: Operation = {
  name: "Count Words",
  id: "count-words",
  description: "Count the number of words in the input text",
  value: "",
  func: tsOperation((input: DataValue): DataValue[] => {
    if (input.type !== "text") {
      throw new Error("Expected text input");
    }

    const wordCount = input.value
      .split(/\s+/)
      .filter((word) => word.length > 0).length;

    return [{ type: "number", value: wordCount }];
  }),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "number", value: 0 }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Number },
};

/**
 * Split text by delimiter - demonstrates text → text_array operation
 */
export const SplitText: Operation = {
  name: "Split Text",
  id: "split-text",
  description: "Split text by a delimiter into an array of strings",
  value: "",
  configValues: {
    delimiter: ",",
  },
  funcBuilder: (config) => {
    const delimiter = String(config?.delimiter || ",");
    return tsOperation((input: DataValue): DataValue[] => {
      if (input.type !== "text") {
        throw new Error("Expected text input");
      }

      const parts = input.value.split(delimiter);
      return [{ type: "text_array", value: parts }];
    });
  },
  func: tsOperation((input: DataValue): DataValue[] => {
    if (input.type !== "text") {
      throw new Error("Expected text input");
    }

    const parts = input.value.split(",");
    return [{ type: "text_array", value: parts }];
  }),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [{ type: "text_array", value: [] }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

/**
 * Join text array - demonstrates text_array → text operation
 */
export const JoinText: Operation = {
  name: "Join Text",
  id: "join-text",
  description: "Join an array of strings with a delimiter",
  value: "",
  configValues: {
    delimiter: ",",
  },
  funcBuilder: (config) => {
    const delimiter = String(config?.delimiter || ",");
    return tsOperation((input: DataValue): DataValue[] => {
      if (input.type !== "text_array") {
        throw new Error("Expected text_array input");
      }

      const joined = input.value.join(delimiter);
      return [{ type: "text", value: joined }];
    });
  },
  func: tsOperation((input: DataValue): DataValue[] => {
    if (input.type !== "text_array") {
      throw new Error("Expected text_array input");
    }

    const joined = input.value.join(",");
    return [{ type: "text", value: joined }];
  }),
  inputTypes: [{ type: "text_array", value: [] }],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

/**
 * Concatenate multiple text inputs - demonstrates multi-input operation
 */
export const ConcatenateText: Operation = {
  name: "Concatenate Text",
  id: "concatenate-text",
  description: "Concatenate multiple text inputs with a separator",
  value: "",
  configValues: {
    separator: " ",
  },
  funcBuilder: (config) => {
    const separator = String(config?.separator || " ");
    return tsMultiInputOperation((inputs: DataValue[]): DataValue[] => {
      const textParts = inputs.map((input) => {
        if (input.type !== "text") {
          throw new Error("All inputs must be text");
        }
        return input.value;
      });

      const concatenated = textParts.join(separator);
      return [{ type: "text", value: concatenated }];
    });
  },
  func: (() => {
    const separator = " ";
    return tsMultiInputOperation((inputs: DataValue[]): DataValue[] => {
      const textParts = inputs.map((input) => {
        if (input.type !== "text") {
          throw new Error("All inputs must be text");
        }
        return input.value;
      });

      const concatenated = textParts.join(separator);
      return [{ type: "text", value: concatenated }];
    });
  })(),
  inputTypes: [
    { type: "text", value: "" },
    { type: "text", value: "" },
  ],
  outputTypes: [{ type: "text", value: "" }],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input1: IOTypes.Text, input2: IOTypes.Text },
  outputs: { output: IOTypes.Text },
};

/**
 * Calculate text statistics - demonstrates text → multiple outputs
 */
export const TextStats: Operation = {
  name: "Text Statistics",
  id: "text-stats",
  description: "Calculate various statistics about the input text",
  value: "",
  func: tsOperation((input: DataValue): DataValue[] => {
    if (input.type !== "text") {
      throw new Error("Expected text input");
    }

    const text = input.value;
    const charCount = text.length;
    const wordCount = text
      .split(/\s+/)
      .filter((word) => word.length > 0).length;
    const lineCount = text.split("\n").length;
    const nonWhitespaceCount = text.replace(/\s/g, "").length;

    return [
      { type: "number", value: charCount },
      { type: "number", value: wordCount },
      { type: "number", value: lineCount },
      { type: "number", value: nonWhitespaceCount },
    ];
  }),
  inputTypes: [{ type: "text", value: "" }],
  outputTypes: [
    { type: "number", value: 0 },
    { type: "number", value: 0 },
    { type: "number", value: 0 },
    { type: "number", value: 0 },
  ],
  tags: [OperationTags.Text, OperationTags.All],
  inputs: { input: IOTypes.Text },
  outputs: {
    charCount: IOTypes.Number,
    wordCount: IOTypes.Number,
    lineCount: IOTypes.Number,
    nonWhitespaceCount: IOTypes.Number,
  },
};
