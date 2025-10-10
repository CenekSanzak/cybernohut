import { DataValue } from "./types";

/**
 * Type guard to check if a DataValue is of a specific type
 */
export function isDataValueType(
  dataValue: DataValue,
  expectedType: DataValue["type"]
): dataValue is DataValue {
  return dataValue.type === expectedType;
}

/**
 * Type guard to check if a value is a DataValue
 */
export function isDataValue(value: any): value is DataValue {
  return (
    value &&
    typeof value === "object" &&
    value !== null &&
    "type" in value &&
    "value" in value &&
    typeof value.type === "string"
  );
}

/**
 * Validates that a DataValue matches the expected type
 */
export function validateDataValueType(
  dataValue: DataValue,
  expectedType: DataValue["type"]
): void {
  if (!isDataValueType(dataValue, expectedType)) {
    throw new Error(
      `Expected ${expectedType} type, got ${(dataValue as any).type}`
    );
  }
}

/**
 * Validates input count matches expected count
 */
export function validateInputCount(
  inputs: DataValue[],
  expectedCount: number
): void {
  if (inputs.length !== expectedCount) {
    throw new Error(`Expected ${expectedCount} inputs, got ${inputs.length}`);
  }
}

/**
 * Validates that all inputs match their expected types
 */
export function validateInputTypes(
  inputs: DataValue[],
  expectedTypes: DataValue["type"][]
): void {
  validateInputCount(inputs, expectedTypes.length);

  inputs.forEach((input, index) => {
    const expectedType = expectedTypes[index];
    if (input.type !== expectedType) {
      throw new Error(
        `Input ${index}: expected ${expectedType}, got ${input.type}`
      );
    }
  });
}

/**
 * Validates that outputs match their expected types
 */
export function validateOutputTypes(
  outputs: DataValue[],
  expectedTypes: DataValue["type"][]
): void {
  if (outputs.length !== expectedTypes.length) {
    throw new Error(
      `Expected ${expectedTypes.length} outputs, got ${outputs.length}`
    );
  }

  outputs.forEach((output, index) => {
    const expectedType = expectedTypes[index];
    if (output.type !== expectedType) {
      throw new Error(
        `Output ${index}: expected ${expectedType}, got ${output.type}`
      );
    }
  });
}

/**
 * Creates a type-safe wrapper for single-input operations
 */
export function createSingleInputValidator(
  expectedInputType: DataValue["type"],
  expectedOutputTypes: DataValue["type"][]
) {
  return (func: (input: DataValue) => DataValue[]) => {
    return (input: DataValue): DataValue[] => {
      validateDataValueType(input, expectedInputType);
      const result = func(input);
      validateOutputTypes(result, expectedOutputTypes);
      return result;
    };
  };
}

/**
 * Creates a type-safe wrapper for multi-input operations
 */
export function createMultiInputValidator(
  expectedInputTypes: DataValue["type"][],
  expectedOutputTypes: DataValue["type"][]
) {
  return (func: (inputs: DataValue[]) => DataValue[]) => {
    return (inputs: DataValue[]): DataValue[] => {
      validateInputTypes(inputs, expectedInputTypes);
      const result = func(inputs);
      validateOutputTypes(result, expectedOutputTypes);
      return result;
    };
  };
}

/**
 * Helper to convert legacy outputTypes to DataValue
 */
export function legacyToDataValue(
  value: string | number | number[] | string[]
): DataValue {
  if (typeof value === "string") {
    return { type: "text", value };
  } else if (typeof value === "number") {
    return { type: "number", value };
  } else if (Array.isArray(value)) {
    if (value.length > 0 && typeof value[0] === "string") {
      return { type: "text_array", value: value as string[] };
    } else if (value.length > 0 && typeof value[0] === "number") {
      return { type: "number_array", value: value as number[] };
    }
  }
  throw new Error(`Cannot convert legacy value to DataValue: ${typeof value}`);
}

/**
 * Helper to convert DataValue to legacy format for backward compatibility
 */
export function dataValueToLegacy(
  dataValue: DataValue
): string | number | number[] | string[] {
  switch (dataValue.type) {
    case "text":
      return dataValue.value;
    case "number":
      return dataValue.value;
    case "text_array":
      return dataValue.value;
    case "number_array":
      return dataValue.value;
    case "binary":
      return btoa(String.fromCharCode(...dataValue.value));
    default:
      throw new Error(
        `Cannot convert DataValue to legacy format: ${(dataValue as any).type}`
      );
  }
}

/**
 * Creates a validation error with context
 */
export function createValidationError(
  operation: string,
  message: string
): Error {
  return new Error(`Validation error in ${operation}: ${message}`);
}
