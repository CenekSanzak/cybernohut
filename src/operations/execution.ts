import { DataValue, Operation, outputTypes } from "./types";
import { legacyToDataValue, isDataValue } from "./validators";

export { isDataValue };

/**
 * Executes an operation with proper type handling
 */
export function executeOperation(
  operation: Operation,
  inputValues: (outputTypes | DataValue)[]
): DataValue[] {
  const dataInputs: DataValue[] = inputValues.map((value) =>
    isDataValue(value) ? value : legacyToDataValue(value)
  );

  let result: DataValue[];

  if (operation.func.signature === "single") {
    if (dataInputs.length !== 1) {
      throw new Error(
        `Single-input operation expected 1 input, got ${dataInputs.length}`
      );
    }
    result = operation.func.func(dataInputs[0]);
  } else if (operation.func.signature === "multi") {
    result = operation.func.func(dataInputs);
  } else {
    throw new Error(
      `Unknown operation signature: ${
        (operation.func as { signature?: string })?.signature || "unknown"
      }`
    );
  }

  return result;
}

/**
 * Validates that input values match the operation's expected input types
 */
export function validateOperationInputs(
  operation: Operation,
  inputValues: (outputTypes | DataValue)[]
): void {
  if (inputValues.length !== operation.inputTypes.length) {
    throw new Error(
      `Operation ${operation.name} expects ${operation.inputTypes.length} inputs, got ${inputValues.length}`
    );
  }

  inputValues.forEach((value, index) => {
    try {
      const dataValue = isDataValue(value) ? value : legacyToDataValue(value);
      const expectedType = operation.inputTypes[index].type;

      if (dataValue.type !== expectedType) {
        throw new Error(
          `Input ${index}: expected ${expectedType}, got ${dataValue.type}`
        );
      }
    } catch (error) {
      throw new Error(`Input ${index} validation failed: ${error}`);
    }
  });
}

/**
 * Creates output values object from operation results
 */
export function createOutputValues(
  operation: Operation,
  results: outputTypes[]
): { [key: string]: outputTypes } {
  const outputValues: { [key: string]: outputTypes } = {};
  const outputKeys = Object.keys(operation.outputs);

  results.forEach((result, index) => {
    const key = outputKeys[index] || `output${index}`;
    outputValues[key] = result;
  });

  return outputValues;
}

/**
 * Creates output values object from DataValue results (new type system)
 */
export function createDataValueOutputValues(
  operation: Operation,
  results: DataValue[]
): { [key: string]: DataValue } {
  const outputValues: { [key: string]: DataValue } = {};
  const outputKeys = Object.keys(operation.outputs);

  results.forEach((result, index) => {
    const key = outputKeys[index] || `output${index}`;
    outputValues[key] = result;
  });

  return outputValues;
}

/**
 * Formats DataValue for display in the UI (short version for nodes)
 */
export function formatDataValueForDisplay(dataValue: DataValue): string {
  switch (dataValue.type) {
    case "text":
      return (
        dataValue.value.slice(0, 30) +
        (dataValue.value.length > 30 ? "..." : "")
      );
    case "number":
      return dataValue.value.toString();
    case "binary":
      return `[Binary: ${dataValue.value.length} bytes]`;
    case "text_array":
      return `[${dataValue.value.length} items]`;
    case "number_array":
      return `[${dataValue.value.length} numbers]`;
    default:
      return "[Unknown type]";
  }
}

/**
 * Formats DataValue for full output display (detailed version for output text box)
 */
export function formatDataValueForOutput(dataValue: DataValue): string {
  switch (dataValue.type) {
    case "text":
      return dataValue.value;
    case "number":
      return dataValue.value.toString();
    case "binary":
      return `Binary Data (${dataValue.value.length} bytes):\n${Array.from(
        dataValue.value
      )
        .map((b) => b.toString(16).padStart(2, "0"))
        .join(" ")}`;
    case "text_array":
      return `Text Array (${dataValue.value.length} items):\n${dataValue.value
        .map((item, index) => `${index}: "${item}"`)
        .join("\n")}`;
    case "number_array":
      return `Number Array (${
        dataValue.value.length
      } numbers):\n${dataValue.value
        .map((item, index) => `${index}: ${item}`)
        .join("\n")}`;
    default:
      return "[Unknown type]";
  }
}

/**
 * Gets the display value for an operation node (short version)
 */
export function getOperationDisplayValue(operation: Operation): string {
  if (operation.outputValues) {
    const outputKeys = Object.keys(operation.outputValues);
    if (outputKeys.length > 0) {
      const firstOutput = operation.outputValues[outputKeys[0]];
      if (firstOutput) {
        return formatDataValueForDisplay(firstOutput);
      }
    }
  }
  return operation.value || "";
}

/**
 * Gets the full output value for an operation (detailed version for output text box)
 */
export function getOperationFullOutput(operation: Operation): string {
  if (operation.outputValues) {
    const outputKeys = Object.keys(operation.outputValues);
    if (outputKeys.length > 0) {
      const firstOutput = operation.outputValues[outputKeys[0]];
      if (firstOutput) {
        return formatDataValueForOutput(firstOutput);
      }
    }
  }
  return operation.value || "";
}
