import React, { useContext } from "react";
import type { NodeProps } from "@xyflow/react";
import { ReportsContext } from "@/graph/context";
import { inputPorts, nodeTitle, outputPorts, type GraphNode } from "@/graph/model";
import { FilePicker } from "./FilePicker";
import { Ports } from "./Ports";
import { needsSetup } from "./report";
import { Result } from "./Result";
import { Settings } from "./Settings";
import { typeColor, typeLabel } from "./valueTypes";

function frame(state: "failed" | "setup" | "selected" | "idle"): string {
  switch (state) {
    case "failed":
      return "border-destructive ring-1 ring-destructive";
    case "setup":
      return "border-amber-500 ring-1 ring-amber-500";
    case "selected":
      return "border-blue-500 ring-1 ring-blue-500";
    case "idle":
      return "border-border";
  }
}

const CustomNode = ({ id, data, selected }: NodeProps<GraphNode>) => {
  const report = useContext(ReportsContext).get(id);
  const state = needsSetup(report)
    ? "setup"
    : report?.status === "error"
      ? "failed"
      : selected
        ? "selected"
        : "idle";
  const outputs = outputPorts(data);

  return (
    <div
      className={`w-60 cursor-pointer rounded-md border bg-white p-3 text-xs shadow-sm ${frame(state)}`}
      title={id}
    >
      <Ports ports={inputPorts(data)} side="input" />
      <div className="flex items-center justify-between gap-2">
        <strong className="text-sm">{nodeTitle(data)}</strong>
        {outputs.length === 1 && (
          <span className="shrink-0 text-[10px]" style={{ color: typeColor[outputs[0].type] }}>
            {typeLabel[outputs[0].type]}
          </span>
        )}
      </div>
      <div className="mt-1 font-mono">
        <Result report={report} />
      </div>
      {data.kind === "op" && data.note && (
        <div className="mt-2 border-l-2 border-gray-300 pl-2 text-[11px] text-muted-foreground">
          <span className="font-medium">Why: </span>
          {data.note}
        </div>
      )}
      {data.kind === "op" && <Settings nodeId={id} data={data} />}
      {data.kind === "file" && <FilePicker nodeId={id} data={data} />}
      <Ports ports={outputs} side="output" />
    </div>
  );
};

export const MemoizedCustomNode = React.memo(CustomNode);
