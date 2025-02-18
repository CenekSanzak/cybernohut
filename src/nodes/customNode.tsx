import React, { useMemo } from "react";
import { Handle, Position } from "@xyflow/react";
import { Operation, OperationTags } from "@/operations/types";

interface CustomNodeProps {
  id: string;
  data: Operation;
  selected?: boolean;
}

const propsAreEqual = (
  prevProps: CustomNodeProps,
  nextProps: CustomNodeProps
) => {
  return (
    prevProps.id === nextProps.id &&
    prevProps.selected === nextProps.selected &&
    prevProps.data.value === nextProps.data.value &&
    prevProps.data.outputValues === nextProps.data.outputValues &&
    prevProps.data.configValues === nextProps.data.configValues
  );
};

const CustomNode = ({ id, data, selected }: CustomNodeProps) => {
  const input_length = data.inputs ? Object.keys(data.inputs).length : 0;
  const output_length = data.outputs ? Object.keys(data.outputs).length : 0;
  const inputHandles = useMemo(() => {
    if (!data.inputs) return null;
    return Object.entries(data.inputs).map(([key, type], index) => (
      <Handle
        key={key}
        type="target"
        position={Position.Top}
        id={key}
        style={{ background: "red" }}
      >
        <span
          style={{
            position: "absolute",
            transform: "translate(-50%, -110%)",
            fontSize: "10px",
            left: `${+(50 / input_length + (100 / input_length) * index)}%`,
          }}
        >
          {key} ({type})
        </span>
      </Handle>
    ));
  }, [data.inputs, input_length]);

  const outputHandles = useMemo(() => {
    if (!data.outputs) return null;
    return Object.entries(data.outputs).map(([key, type], index) => (
      <Handle
        key={key}
        type="source"
        position={Position.Bottom}
        id={key}
        style={{
          background: "blue",
          left: `${+(50 / output_length + (100 / output_length) * index)}%`,
          textAlign: "center",
        }}
      >
        <span
          style={{
            position: "absolute",
            transform: "translate(-50%, 20%)",
            fontSize: "10px",
          }}
        >
          {key} ({type})
        </span>
      </Handle>
    ));
  }, [data.outputs, output_length]);

  const configInputs = useMemo(() => {
    if (!data.configValues) return null;
    return (
      <div className="mt-2 border-t pt-2">
        {Object.entries(data.configValues).map(([key, value]) => (
          <div key={key} className="flex items-center text-xs">
            <label className="mr-2">{key}:</label>
            {typeof value === "boolean" ? (
              <input
                type="checkbox"
                checked={value}
                onChange={(e) => {
                  if (data.onConfigChange) {
                    data.onConfigChange({
                      ...data.configValues,
                      [key]: e.target.checked,
                    });
                  }
                }}
              />
            ) : typeof value === "number" ? (
              <input
                type="number"
                value={value}
                onChange={(e) => {
                  if (data.onConfigChange) {
                    data.onConfigChange({
                      ...data.configValues,
                      [key]: Number(e.target.value),
                    });
                  }
                }}
                className="w-16 px-1"
              />
            ) : (
              <input
                type="text"
                value={value}
                onChange={(e) => {
                  if (data.onConfigChange) {
                    data.onConfigChange({
                      ...data.configValues,
                      [key]: e.target.value,
                    });
                  }
                }}
                className="w-24 px-1"
              />
            )}
          </div>
        ))}
      </div>
    );
  }, [data]);

  const nodeClassName = useMemo(
    () =>
      `p-4 border rounded shadow-md bg-white items-center cursor-pointer hover:bg-gray-50 ${
        selected ? "border-blue-500 border-2" : ""
      } ${data.tags.includes(OperationTags.IO) ? "bg-gray-50" : ""}`,
    [selected, data.tags]
  );

  return (
    <div className={nodeClassName} data-value={data.value}>
      <div>
        <strong>{data.name}</strong>
        <div className="text-[10px] text-gray-400 -mt-1">{id}</div>
      </div>
      {inputHandles}
      <div className="mt-2">{data.value?.slice(0, 30) || ""}</div>
      {configInputs}
      {outputHandles}
    </div>
  );
};

CustomNode.displayName = "CustomNode";

export const MemoizedCustomNode = React.memo(CustomNode, propsAreEqual);
