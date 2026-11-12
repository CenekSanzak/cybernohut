import { useContext, useRef } from "react";
import { GraphDispatchContext } from "@/graph/context";
import { formatBytes, loadFile } from "@/graph/files";
import type { FileNodeData } from "@/graph/model";

export function FilePicker({ nodeId, data }: { nodeId: string; data: FileNodeData }) {
  const dispatch = useContext(GraphDispatchContext);
  const input = useRef<HTMLInputElement>(null);

  return (
    <div className="mt-2 border-t pt-2">
      {data.name && (
        <div className="mb-1 break-all text-gray-600">
          {data.name} ({formatBytes(data.size)})
        </div>
      )}
      <button
        type="button"
        className="nodrag rounded border px-2 py-0.5 hover:bg-gray-100"
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
