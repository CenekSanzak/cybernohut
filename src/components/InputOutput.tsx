import React, { useMemo } from "react";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Node } from "@xyflow/react";
import { Operation, OperationTags } from "@/operations/types";

interface InputOutputProps {
  input: string;
  output: string;
  onInputChange: (value: string, nodeId: string) => void;
  calculate: () => void;
  autoCalculate: boolean;
  onAutoCalculateChange: (checked: boolean) => void;
  selectedNodeId?: string;
  selectedNodeTitle?: string;
  nodes: Node[];
  selectedInputId?: string;
  onInputTabSelect: (nodeId: string) => void;
}

const InputOutput = React.memo<InputOutputProps>(
  ({
    input,
    output,
    onInputChange,
    calculate,
    autoCalculate,
    onAutoCalculateChange,
    selectedNodeId,
    selectedNodeTitle,
    nodes,
    selectedInputId,
    onInputTabSelect,
  }) => {
    const inputNodes = useMemo(
      () =>
        nodes.filter((node) =>
          (node.data as Operation).tags.includes(OperationTags.IO)
        ),
      [nodes]
    );

    return (
      <div className="flex flex-col h-full">
        <div className="flex-1 p-4">
          <h3 className="text-lg font-semibold mb-2">Input</h3>
          {inputNodes.length > 0 && (
            <div className="flex gap-2 mb-2">
              {inputNodes.map((node) => (
                <button
                  key={node.id}
                  onClick={() => onInputTabSelect(node.id)}
                  className={`px-3 py-1 rounded ${
                    selectedInputId === node.id
                      ? "bg-blue-500 text-white"
                      : "bg-gray-100 hover:bg-gray-200"
                  }`}
                >
                  {(node.data as Operation).name}
                </button>
              ))}
            </div>
          )}
          <Textarea
            value={input}
            onChange={(e) =>
              onInputChange(e.target.value, selectedInputId || "input-node")
            }
            className="w-full h-full min-h-[200px]"
            placeholder="Enter your input here..."
          />
        </div>
        <div className="flex-1 p-4 bg-gray-50">
          <h3 className="text-lg font-semibold mb-2 flex items-center gap-4">
            Output{" "}
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <Checkbox
                  id="auto-calculate"
                  checked={autoCalculate}
                  onCheckedChange={onAutoCalculateChange}
                />
                <Label htmlFor="auto-calculate" className="text-sm font-normal">
                  Auto-calculate
                </Label>
              </div>
              <button
                onClick={calculate}
                className="px-4 py-1 bg-blue-500 text-white rounded-full hover:bg-blue-600 transition-colors"
              >
                Calculate
              </button>
            </div>
          </h3>
          {selectedNodeId && (
            <div className="text-sm text-gray-500 mb-2">
              {selectedNodeTitle} ({selectedNodeId}):
            </div>
          )}
          <Textarea
            value={output}
            readOnly
            className="w-full h-full min-h-[200px] bg-white"
            placeholder="Output will appear here..."
          />
        </div>
      </div>
    );
  }
);

InputOutput.displayName = "InputOutput";

export default InputOutput;
