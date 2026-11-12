import { useContext } from "react";
import { getMeta, type ParamSpec } from "@/engine/wasm";
import { GraphDispatchContext } from "@/graph/context";
import { configOf, type OpNodeData } from "@/graph/model";

function ParamInput({
  nodeId,
  spec,
  value,
}: {
  nodeId: string;
  spec: ParamSpec;
  value: string | number | boolean;
}) {
  const dispatch = useContext(GraphDispatchContext);
  const set = (next: string | number | boolean) =>
    dispatch({ type: "SET_CONFIG", nodeId, key: spec.key, value: next });
  const label = `${spec.label} configuration`;

  switch (spec.kind.kind) {
    case "bool":
      return (
        <input
          type="checkbox"
          className="nodrag"
          checked={value === true}
          onChange={(e) => set(e.target.checked)}
          aria-label={label}
        />
      );
    case "int":
      return (
        <input
          type="number"
          className="nodrag w-16 rounded border px-1"
          value={Number(value)}
          min={spec.kind.min ?? undefined}
          max={spec.kind.max ?? undefined}
          step={1}
          onChange={(e) => set(Math.trunc(Number(e.target.value)) || 0)}
          aria-label={label}
        />
      );
    case "choice":
      return (
        <select
          className="nodrag rounded border px-1"
          value={String(value)}
          onChange={(e) => set(e.target.value)}
          aria-label={label}
        >
          {spec.kind.options.map((option) => (
            <option key={option}>{option}</option>
          ))}
        </select>
      );
    case "text":
      return (
        <input
          type="text"
          className="nodrag w-28 rounded border px-1"
          value={String(value)}
          onChange={(e) => set(e.target.value)}
          aria-label={label}
        />
      );
  }
}

/** The form for an operation's config, generated from the params it declares. */
export function Settings({ nodeId, data }: { nodeId: string; data: OpNodeData }) {
  const dispatch = useContext(GraphDispatchContext);
  const meta = getMeta(data.opId);
  if (meta.params.length === 0 && !meta.supports_per_line) return null;
  const config = configOf(data);

  return (
    <div className="mt-2 space-y-1 border-t pt-2">
      {meta.params.map((spec) => (
        <label key={spec.key} className="flex items-center justify-between gap-2">
          {spec.label}
          <ParamInput nodeId={nodeId} spec={spec} value={config[spec.key]} />
        </label>
      ))}
      {meta.supports_per_line && (
        <label
          className="flex items-center justify-between gap-2 text-muted-foreground"
          title="Apply the operation to each line separately"
        >
          Line by line
          <input
            type="checkbox"
            className="nodrag"
            checked={data.perLine}
            onChange={(e) =>
              dispatch({ type: "SET_PER_LINE", nodeId, perLine: e.target.checked })
            }
            aria-label="Line by line"
          />
        </label>
      )}
    </div>
  );
}
