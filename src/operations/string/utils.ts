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
        throw new Error("Incorrect number of arguments");
      }
      const function_result = func(args[0] as string);
      if (function_result.length !== outputTypes.length) {
        throw new Error("Incorrect number of arguments");
      }
      return function_result as outputTypes[];
    } catch (e) {
      console.error(e);
      return [];
    }
  };
