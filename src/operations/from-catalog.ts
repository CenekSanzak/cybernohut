/**
 * Bridges operations defined in Rust into the legacy `Operation` shape the UI still
 * consumes. Nothing about an operation is written by hand here: name, ports, tags and
 * config all come from the catalog. This file goes away once the UI reads the catalog
 * directly.
 */
import {
  catalog,
  get,
  isOpError,
  put,
  release,
  run,
  type Handle,
  type OpConfigMap,
  type OpId,
  type OpMeta,
  type Value,
  type ValueType,
} from "@/engine/wasm";
import {
  ConfigValues,
  DataValue,
  IOTypes,
  Operation,
  OperationFunction,
  OperationTags,
} from "./types";

const legacyType: Record<ValueType, DataValue["type"]> = {
  bytes: "binary",
  text: "text",
  number: "number",
  text_list: "text_array",
  number_list: "number_array",
};

const ioType: Record<ValueType, IOTypes> = {
  bytes: IOTypes.Binary,
  text: IOTypes.Text,
  number: IOTypes.Number,
  text_list: IOTypes.Text,
  number_list: IOTypes.Number,
};

const placeholder: Record<ValueType, DataValue> = {
  bytes: { type: "binary", value: new Uint8Array() },
  text: { type: "text", value: "" },
  number: { type: "number", value: 0 },
  text_list: { type: "text_array", value: [] },
  number_list: { type: "number_array", value: [] },
};

function toValue(data: DataValue): Value {
  switch (data.type) {
    case "binary":
      return { type: "bytes", value: data.value };
    case "text":
      return { type: "text", value: data.value };
    case "number":
      return { type: "number", value: data.value };
    case "text_array":
      return { type: "text_list", value: data.value };
    case "number_array":
      return { type: "number_list", value: data.value };
  }
}

function toDataValue(value: Value): DataValue {
  return { type: legacyType[value.type], value: value.value } as DataValue;
}

function makeFunc<K extends OpId>(
  id: K,
  config: OpConfigMap[K]
): OperationFunction {
  return {
    signature: "multi",
    func: (inputs) => {
      const handles: Handle[] = [];
      try {
        for (const input of inputs) handles.push(put(toValue(input)));
        const outputs = run(id, config, handles);
        handles.push(...outputs);
        return outputs.map((handle) => toDataValue(get(handle)));
      } catch (error) {
        throw isOpError(error) ? new Error(error.message) : error;
      } finally {
        handles.forEach(release);
      }
    },
  };
}

function toOperation(meta: OpMeta): Operation {
  const id = meta.id as OpId;
  const hasConfig = meta.params.length > 0;
  return {
    id,
    name: meta.name,
    description: meta.description,
    link: meta.link ?? undefined,
    value: "",
    tags: [...(meta.tags as string[] as OperationTags[]), OperationTags.All],
    inputs: Object.fromEntries(meta.inputs.map((p) => [p.name, ioType[p.type]])),
    outputs: Object.fromEntries(
      meta.outputs.map((p) => [p.name, ioType[p.type]])
    ),
    inputTypes: meta.inputs.map((p) => placeholder[p.type]),
    outputTypes: meta.outputs.map((p) => placeholder[p.type]),
    func: makeFunc(id, meta.default_config as OpConfigMap[OpId]),
    ...(hasConfig && {
      configValues: meta.default_config,
      // Rust validates the config at run time, so an untyped UI value is safe to pass.
      funcBuilder: (config?: ConfigValues) =>
        makeFunc(id, (config ?? meta.default_config) as OpConfigMap[OpId]),
    }),
  };
}

export const catalogOperations: Operation[] = catalog.map(toOperation);
