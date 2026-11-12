import React, { useContext, useRef } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import {
  getMeta,
  type NodeReport,
  type ParamSpec,
  type Port,
  type ValueType,
} from "@/engine/wasm";
import { GraphDispatchContext, ReportsContext } from "@/graph/context";
import { formatBytes, loadFile } from "@/graph/files";
import {
  configOf,
  inputPorts,
  nodeTitle,
  outputPorts,
  type FileNodeData,
  type GraphNode,
  type OpNodeData,
} from "@/graph/model";

const typeColor: Record<ValueType, string> = {
  text: "#2563eb",
  bytes: "#ea580c",
  number: "#16a34a",
  text_list: "#9333ea",
  number_list: "#9333ea",
};

const typeLabel: Record<ValueType, string> = {
  text: "text",
  bytes: "bytes",
  number: "number",
  text_list: "text list",
  number_list: "number list",
};

function Ports({ ports, side }: { ports: readonly Port[]; side: "input" | "output" }) {
  return ports.map((port, index) => {
    const left = `${(100 / ports.length) * (index + 0.5)}%`;
    const isInput = side === "input";
    return (
      <Handle
        key={port.name}
        id={port.name}
        type={isInput ? "target" : "source"}
        position={isInput ? Position.Top : Position.Bottom}
        style={{ background: typeColor[port.type], left }}
        aria-label={`${isInput ? "Input" : "Output"} ${port.name} of type ${typeLabel[port.type]}`}
      >
        <span
          className="absolute whitespace-nowrap text-[10px]"
          style={{
            color: typeColor[port.type],
            transform: isInput ? "translate(-50%, -110%)" : "translate(-50%, 20%)",
          }}
        >
          {port.name} ({typeLabel[port.type]})
        </span>
      </Handle>
    );
  });
}

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
          className="nodrag w-16 px-1 border rounded"
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
          className="nodrag px-1 border rounded"
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
          className="nodrag w-24 px-1 border rounded"
          value={String(value)}
          onChange={(e) => set(e.target.value)}
          aria-label={label}
        />
      );
  }
}

function Settings({ nodeId, data }: { nodeId: string; data: OpNodeData }) {
  const dispatch = useContext(GraphDispatchContext);
  const meta = getMeta(data.opId);
  if (meta.params.length === 0 && !meta.supports_per_line) return null;
  const config = configOf(data);

  return (
    <div className="mt-2 border-t pt-2 space-y-1 text-xs">
      {meta.params.map((spec) => (
        <label key={spec.key} className="flex items-center justify-between gap-2">
          {spec.label}
          <ParamInput nodeId={nodeId} spec={spec} value={config[spec.key]} />
        </label>
      ))}
      {meta.supports_per_line && (
        <label
          className="flex items-center justify-between gap-2 text-gray-600"
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

function FilePicker({ nodeId, data }: { nodeId: string; data: FileNodeData }) {
  const dispatch = useContext(GraphDispatchContext);
  const input = useRef<HTMLInputElement>(null);

  return (
    <div className="mt-2 border-t pt-2 text-xs">
      {data.name && (
        <div className="mb-1 text-gray-600 break-all">
          {data.name} ({formatBytes(data.size)})
        </div>
      )}
      <button
        type="button"
        className="nodrag px-2 py-0.5 border rounded hover:bg-gray-100"
        onClick={() => input.current?.click()}
      >
        {data.name ? "Replace file" : "Choose file"}
      </button>
      <input
        ref={input}
        type="file"
        className="hidden"
        aria-label="File to load"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file) void loadFile(nodeId, file, dispatch);
          event.target.value = "";
        }}
      />
    </div>
  );
}

function Result({ report }: { report: NodeReport | undefined }) {
  if (!report) return null;
  switch (report.status) {
    case "ok":
      return report.outputs.map((output) => (
        <div key={output.port} className="text-gray-600 break-all">
          {report.outputs.length > 1 && <span className="font-medium">{output.port}: </span>}
          {output.preview}
          {output.truncated && "…"}
        </div>
      ));
    case "error":
      return (
        <div className="text-red-600" role="alert">
          {report.error.message}
        </div>
      );
    case "waiting":
      return <div className="text-gray-400">Not connected</div>;
    case "blocked":
      return <div className="text-gray-400">Waiting for an earlier step</div>;
  }
}

const CustomNode = ({ id, data, selected }: NodeProps<GraphNode>) => {
  const report = useContext(ReportsContext).get(id);
  const failed = report?.status === "error";

  return (
    <div
      className={`p-4 border rounded shadow-md max-w-64 cursor-pointer hover:bg-gray-50 ${
        data.kind === "op" ? "bg-white" : "bg-gray-50"
      } ${failed ? "border-red-500 border-2" : selected ? "border-blue-500 border-2" : ""}`}
    >
      <Ports ports={inputPorts(data)} side="input" />
      <strong>{nodeTitle(data)}</strong>
      <div className="text-[10px] text-gray-600 -mt-1">{id}</div>
      <div className="mt-2 text-xs">
        <Result report={report} />
      </div>
      {data.kind === "op" && data.note && (
        <div className="mt-2 text-[11px] italic text-indigo-700">Suggested: {data.note}</div>
      )}
      {data.kind === "op" && <Settings nodeId={id} data={data} />}
      {data.kind === "file" && <FilePicker nodeId={id} data={data} />}
      <Ports ports={outputPorts(data)} side="output" />
    </div>
  );
};

export const MemoizedCustomNode = React.memo(CustomNode);
