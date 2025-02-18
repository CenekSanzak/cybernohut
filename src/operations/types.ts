export enum OperationTags {
  All = "All",
  Text = "Text",
  BasicEncryption = "Basic Encryption",
  Encoding = "Encoding",
  Hashing = "Hashing",
  NumberBase = "Number Base",
  IO = "I/O",
}

export enum IOTypes {
  Text = "Text",
  Number = "Number",
  Hexadecimal = "Hexadecimal",
  Binary = "Binary",
}

export type ConfigValues = {
  [key: string]: string | number | boolean;
};

export type outputTypes = string | number | number[] | string[];

export type OperationFunction = (...args: outputTypes[]) => outputTypes[];

export interface Operation {
  name: string;
  description: string;
  value: string;
  outputValues?: { [key: string]: outputTypes };
  configValues?: ConfigValues;
  id: string;
  funcBuilder?: (configValues?: ConfigValues) => OperationFunction;
  func: OperationFunction;
  tags: OperationTags[];
  inputs: { [key: string]: IOTypes };
  outputs: { [key: string]: IOTypes };
  link?: string;
  onConfigChange?: (newConfig: ConfigValues) => void;
  [key: string]: unknown;
}
