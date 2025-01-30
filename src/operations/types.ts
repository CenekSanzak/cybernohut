export enum OperationTags {
  All = "All",
  Text = "Text",
  BasicEncryption = "Basic Encryption",
  Encoding = "Encoding",
  Hashing = "Hashing",
}

export enum IOTypes {
  Text = "Text",
  Number = "Number",
  Hexadecimal = "Hexadecimal",
  Binary = "Binary",
}
export type outputTypes = string | number | number[] | string[];
export interface Operation {
  name: string;
  description: string;
  value: string;
  outputValues?: { [key: string]: outputTypes };
  id: string;
  func: (...args: outputTypes[]) => outputTypes;
  tags: OperationTags[];
  inputs: { [key: string]: IOTypes };
  outputs: { [key: string]: IOTypes };
  [key: string]: unknown;
}

