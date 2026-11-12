/**
 * Typed access to the Rust engine.
 *
 * Every type here is generated from the Rust source by wasm-bindgen and tsify. This
 * file only narrows the two places where the generated signature is looser than the
 * contract: handles are branded, and `run` ties the config type to the operation id.
 */
import * as wasm from "wasm";
import type {
  ErrorCode,
  OpConfigMap,
  OpError,
  OpId,
  OpMeta,
  ParamKind,
  ParamSpec,
  Port,
  Tag,
  ValueType,
} from "wasm";

export type {
  ErrorCode,
  OpConfigMap,
  OpError,
  OpId,
  OpMeta,
  ParamKind,
  ParamSpec,
  Port,
  Tag,
  ValueType,
};

/** A value that lives in wasm memory. Release it when it is no longer needed. */
export type Handle = number & { readonly __brand: "Handle" };

/** A value copied out of wasm memory, tagged with its type. */
export type Value =
  | { type: "bytes"; value: Uint8Array }
  | { type: "text"; value: string }
  | { type: "number"; value: number }
  | { type: "text_list"; value: string[] }
  | { type: "number_list"; value: number[] };

type OpMetaById = { [K in OpId]: OpMeta & { id: K } };

export const catalog: readonly OpMeta[] = wasm.catalog();

const metaById = Object.fromEntries(
  catalog.map((meta) => [meta.id, meta])
) as OpMetaById;

export function isOpId(id: string): id is OpId {
  return id in metaById;
}

export function getMeta<K extends OpId>(id: K): OpMetaById[K] {
  return metaById[id];
}

export function isOpError(error: unknown): error is OpError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    "message" in error
  );
}

/** Runs an operation on stored values. Throws an {@link OpError}. */
export function run<K extends OpId>(
  id: K,
  config: OpConfigMap[K],
  inputs: readonly Handle[]
): Handle[] {
  return Array.from(
    wasm.run(id, config, Uint32Array.from(inputs))
  ) as Handle[];
}

export function put(value: Value): Handle {
  switch (value.type) {
    case "bytes":
      return wasm.put_bytes(value.value) as Handle;
    case "text":
      return wasm.put_text(value.value) as Handle;
    case "number":
      return wasm.put_number(value.value) as Handle;
    case "text_list":
      return wasm.put_text_list(value.value) as Handle;
    case "number_list":
      return wasm.put_number_list(Float64Array.from(value.value)) as Handle;
  }
}

export function get(handle: Handle): Value {
  const type = wasm.value_type(handle);
  switch (type) {
    case "bytes":
      return { type, value: wasm.get_bytes(handle) };
    case "text":
      return { type, value: wasm.get_text(handle) };
    case "number":
      return { type, value: wasm.get_number(handle) };
    case "text_list":
      return { type, value: wasm.get_text_list(handle) };
    case "number_list":
      return { type, value: Array.from(wasm.get_number_list(handle)) };
  }
}

export function release(handle: Handle): void {
  wasm.release(handle);
}
