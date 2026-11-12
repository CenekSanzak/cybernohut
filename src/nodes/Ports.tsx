import { Handle, Position } from "@xyflow/react";
import type { Port } from "@/engine/wasm";
import { typeColor, typeLabel } from "./valueTypes";

/**
 * A row of connection points. Colour carries the type; the name is written out only
 * when a node has several ports on one side and they need telling apart.
 */
export function Ports({ ports, side }: { ports: readonly Port[]; side: "input" | "output" }) {
  return ports.map((port, index) => {
    const left = `${(100 / ports.length) * (index + 0.5)}%`;
    const isInput = side === "input";
    const description = `${isInput ? "Input" : "Output"} ${port.name}: ${typeLabel[port.type]}`;
    return (
      <Handle
        key={port.name}
        id={port.name}
        type={isInput ? "target" : "source"}
        position={isInput ? Position.Top : Position.Bottom}
        style={{ background: typeColor[port.type], left, width: 10, height: 10 }}
        title={description}
        aria-label={description}
      >
        {ports.length > 1 && (
          <span
            className="pointer-events-none absolute whitespace-nowrap text-[10px]"
            style={{
              color: typeColor[port.type],
              transform: isInput ? "translate(-50%, -120%)" : "translate(-50%, 60%)",
            }}
          >
            {port.name}
          </span>
        )}
      </Handle>
    );
  });
}
