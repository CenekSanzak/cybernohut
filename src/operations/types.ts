export enum OperationTags {
  All = "All",
  Text = "Text",
  BasicEncryption = "Basic Encryption",
  Encoding = "Encoding",
  Hashing = "Hashing",
  NumberBase = "Number Base",
  IO = "I/O",
  SubstitutionCipher = "Substitution Cipher",
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

export type DataValue =
  | { type: "text"; value: string }
  | { type: "number"; value: number }
  | { type: "binary"; value: Uint8Array }
  | { type: "text_array"; value: string[] }
  | { type: "number_array"; value: number[] };

export type OperationFunction =
  | { signature: "single"; func: (input: DataValue) => DataValue[] }
  | { signature: "multi"; func: (inputs: DataValue[]) => DataValue[] };

export type outputTypes = string | number | number[] | string[];

export interface Operation {
  name: string;
  description: string;
  value: string;
  outputValues?: { [key: string]: DataValue };
  configValues?: ConfigValues;
  id: string;
  funcBuilder?: (configValues?: ConfigValues) => OperationFunction;
  func: OperationFunction;
  tags: OperationTags[];
  inputs: { [key: string]: IOTypes };
  outputs: { [key: string]: IOTypes };
  inputTypes: DataValue[];
  outputTypes: DataValue[];
  link?: string;
  onConfigChange?: (newConfig: ConfigValues) => void;
  cleanup?: () => void;
  error?: string;
  [key: string]: unknown;
}
