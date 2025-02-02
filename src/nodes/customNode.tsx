import { Handle, Position } from "@xyflow/react";
import { Operation } from "@/operations/types";

export const CustomNode = ({ data }: { data: Operation }) => {
  const input_length = data.inputs ? Object.keys(data.inputs).length : 0;
  const output_length = data.outputs ? Object.keys(data.outputs).length : 0;
  return (
    <div className="p-4 border rounded shadow-md bg-white items-center">
      <strong>{data.name}</strong>
      {!data.inputs
        ? undefined
        : Object.entries(data.inputs).map(([key, type], index) => (
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
                  left: `${+(
                    50 / input_length +
                    (100 / input_length) * index
                  )}%`,
                }}
              >
                {key} ({type})
              </span>
            </Handle>
          ))}
      <div className="mt-2">{data.value}</div>
      {!data.outputs
        ? undefined
        : Object.entries(data.outputs).map(([key, type], index) => (
            <Handle
              key={key}
              type="source"
              position={Position.Bottom}
              id={key}
              style={{
                background: "blue",
                left: `${+(
                  50 / output_length +
                  (100 / output_length) * index
                )}%`,
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
          ))}
    </div>
  );
};
