import { DataValue, OperationFunction } from "./types";
import { validateDataValueType, createValidationError } from "./validators";

/**
 * WASM function signature for text-to-text operations
 */
type WasmTextFunction = (input: string) => string[];

/**
 * WASM closure builder signature - returns a JavaScript function
 */
type WasmClosureBuilder = (config: string) => (input: string) => string[];

/**
 * Converts DataValue to string for WASM interop
 */
function dataValueToString(dataValue: DataValue): string {
  switch (dataValue.type) {
    case "text":
      return dataValue.value;
    case "number":
      return dataValue.value.toString();
    case "binary":
      return btoa(String.fromCharCode(...dataValue.value));
    case "text_array":
      return JSON.stringify(dataValue.value);
    case "number_array":
      return JSON.stringify(dataValue.value);
    default:
      throw new Error(
        `Cannot convert ${(dataValue as DataValue).type} to string for WASM`
      );
  }
}

/**
 * Converts string result from WASM back to DataValue
 */
function stringToDataValue(
  result: string,
  expectedType: DataValue["type"]
): DataValue {
  switch (expectedType) {
    case "text":
      return { type: "text", value: result };
    case "number": {
      const num = parseFloat(result);
      if (isNaN(num)) {
        throw new Error(`Cannot convert "${result}" to number`);
      }
      return { type: "number", value: num };
    }
    case "binary":
      try {
        const binary = Uint8Array.from(atob(result), (c: string) =>
          c.charCodeAt(0)
        );
        return { type: "binary", value: binary };
      } catch {
        throw new Error(`Cannot decode base64: ${result}`);
      }
    case "text_array":
      try {
        const parsed = JSON.parse(result);
        if (
          !Array.isArray(parsed) ||
          !parsed.every((item) => typeof item === "string")
        ) {
          throw new Error("Invalid text array format");
        }
        return { type: "text_array", value: parsed };
      } catch {
        throw new Error(`Cannot parse text array: ${result}`);
      }
    case "number_array":
      try {
        const parsed = JSON.parse(result);
        if (
          !Array.isArray(parsed) ||
          !parsed.every((item) => typeof item === "number")
        ) {
          throw new Error("Invalid number array format");
        }
        return { type: "number_array", value: parsed };
      } catch {
        throw new Error(`Cannot parse number array: ${result}`);
      }
    default:
      throw new Error(`Unsupported type: ${expectedType}`);
  }
}

/**
 * Creates a WASM adapter for text-to-text operations
 */
export function wasmTextToText(
  wasmFunc: WasmTextFunction,
  expectedOutputType: DataValue["type"] = "text"
): OperationFunction {
  return {
    signature: "single",
    func: (input: DataValue): DataValue[] => {
      validateDataValueType(input, "text");

      try {
        const result = wasmFunc(String(input.value));
        return result.map((item) =>
          stringToDataValue(item, expectedOutputType)
        );
      } catch (error) {
        throw createValidationError(
          "WASM text operation",
          error instanceof Error ? error.message : "Unknown WASM error"
        );
      }
    },
  };
}

/**
 * Creates a WASM adapter for text-to-text operations with config
 */
export function wasmTextToTextWithConfig(
  wasmFuncBuilder: WasmClosureBuilder,
  config: Record<string, string | number | boolean>,
  expectedOutputType: DataValue["type"] = "text"
): OperationFunction {
  return {
    signature: "single",
    func: (input: DataValue): DataValue[] => {
      validateDataValueType(input, "text");

      try {
        const wasmFunc = wasmFuncBuilder(JSON.stringify(config)) as (
          input: string
        ) => string[];
        const result = wasmFunc(String(input.value));
        const resultArray = Array.isArray(result) ? result : [result];
        return resultArray.map((item) =>
          stringToDataValue(item, expectedOutputType)
        );
      } catch (error) {
        throw createValidationError(
          "WASM text operation with config",
          error instanceof Error ? error.message : "Unknown WASM error"
        );
      }
    },
  };
}

/**
 * Creates a WASM adapter for multi-input operations
 */
export function wasmMultiInputToText(
  wasmFunc: (inputs: string[]) => string[],
  expectedInputTypes: DataValue["type"][],
  expectedOutputType: DataValue["type"] = "text"
): OperationFunction {
  return {
    signature: "multi",
    func: (inputs: DataValue[]): DataValue[] => {
      if (inputs.length !== expectedInputTypes.length) {
        throw createValidationError(
          "WASM multi-input operation",
          `Expected ${expectedInputTypes.length} inputs, got ${inputs.length}`
        );
      }

      inputs.forEach((input, index) => {
        validateDataValueType(input, expectedInputTypes[index]);
      });

      try {
        const stringInputs = inputs.map(dataValueToString);
        const result = wasmFunc(stringInputs);
        return result.map((item) =>
          stringToDataValue(item, expectedOutputType)
        );
      } catch (error) {
        throw createValidationError(
          "WASM multi-input operation",
          error instanceof Error ? error.message : "Unknown WASM error"
        );
      }
    },
  };
}

/**
 * Creates a TypeScript-only operation adapter
 */
export function tsOperation(
  func: (input: DataValue) => DataValue[]
): OperationFunction {
  return {
    signature: "single",
    func,
  };
}

/**
 * Creates a TypeScript-only multi-input operation adapter
 */
export function tsMultiInputOperation(
  func: (inputs: DataValue[]) => DataValue[]
): OperationFunction {
  return {
    signature: "multi",
    func,
  };
}

/**
 * Helper to create a configurable WASM operation
 */
export function createConfigurableWasmOperation(
  wasmFuncBuilder: WasmClosureBuilder,
  defaultConfig: Record<string, string | number | boolean>,
  expectedOutputType: DataValue["type"] = "text"
) {
  return (
    config?: Record<string, string | number | boolean>
  ): OperationFunction => {
    const mergedConfig = { ...defaultConfig, ...config };
    return wasmTextToTextWithConfig(
      wasmFuncBuilder,
      mergedConfig,
      expectedOutputType
    );
  };
}

/**
 * Serializes DataValue for WASM JSON operations
 */
export function serializeDataValueForWasm(dataValue: DataValue): string {
  return JSON.stringify({
    type: dataValue.type,
    value:
      dataValue.type === "binary"
        ? Array.from(dataValue.value)
        : dataValue.value,
  });
}

/**
 * Deserializes DataValue from WASM JSON operations
 */
export function deserializeDataValueFromWasm(json: string): DataValue {
  try {
    const parsed = JSON.parse(json);
    if (parsed.type === "binary") {
      return { type: "binary", value: new Uint8Array(parsed.value) };
    }
    return parsed as DataValue;
  } catch (error) {
    throw new Error(`Failed to deserialize DataValue: ${error}`);
  }
}
