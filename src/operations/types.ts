export enum OperationTags {
  All = "All",
  Text = "Text",
  BasicEncryption = "Basic Encryption",
  Encoding = "Encoding",
  Hashing = "Hashing",
  NumberBase = "Number Base",
}

export enum IOTypes {
  Text = "Text",
  Number = "Number",
  Hexadecimal = "Hexadecimal",
  Binary = "Binary",
}

export type outputTypes = string | number | number[] | string[];

export type OperationFunction = (...args: outputTypes[]) => outputTypes[];

export interface Operation {
  name: string;
  description: string;
  value: string;
  outputValues?: { [key: string]: outputTypes };
  id: string;
  func: OperationFunction;
  tags: OperationTags[];
  inputs: { [key: string]: IOTypes };
  outputs: { [key: string]: IOTypes };
  [key: string]: unknown;
}
