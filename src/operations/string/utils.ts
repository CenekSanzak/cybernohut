import { IOTypes, outputTypes } from "../types";

export const validateOutputTypeStringToString =
  (
    func: (...s: string[]) => string[],
    inputTypes: IOTypes[],
    outputTypes: IOTypes[]
  ) =>
  (...args: outputTypes[]) => {
    try {
      if (args.length !== inputTypes.length) {
        throw new Error(
          `Expected ${inputTypes.length} arguments, got ${args.length}`
        );
      }

      if (typeof args[0] !== "string") {
        throw new Error(`Expected string input, got ${typeof args[0]}`);
      }

      const function_result = func(args[0] as string);

      if (!Array.isArray(function_result)) {
        throw new Error("Operation returned non-array result");
      }

      if (function_result.length !== outputTypes.length) {
        throw new Error(
          `Expected ${outputTypes.length} outputs, got ${function_result.length}`
        );
      }

      return function_result as outputTypes[];
    } catch (e) {
      const errorMessage =
        e instanceof Error ? e.message : "Unknown error occurred";
      console.error("Operation validation error:", errorMessage);
      throw new Error(`Operation failed: ${errorMessage}`);
    }
  };
