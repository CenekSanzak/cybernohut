import React from "react";
import type { Proposal } from "@/engine/wasm";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

/** Auto mode for the step whose output is on show. */
export interface AutoMode {
  /** Name of the step the proposals are for. */
  target: string;
  /** Whether that step has an output to analyse. */
  ready: boolean;
  busy: boolean;
  /** Absent until asked for; empty when nothing would make the output more readable. */
  proposals?: { proposal: Proposal; steps: string[] }[];
  onSuggest: () => void;
  onApply: (proposal: Proposal) => void;
}

function AutoPanel({ auto }: { auto: AutoMode }) {
  return (
    <div className="mb-2 text-sm">
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={auto.onSuggest}
          disabled={!auto.ready || auto.busy}
          className="px-2 py-0.5 text-xs border rounded bg-white hover:bg-gray-100 disabled:opacity-50"
        >
          {auto.busy ? "Looking…" : "Suggest decoding steps"}
        </button>
        <span className="text-xs text-gray-500">for {auto.target}</span>
      </div>
      {auto.proposals?.length === 0 && (
        <p className="mt-2 text-xs text-gray-600">
          Nothing found that makes this more readable than it already is.
        </p>
      )}
      {auto.proposals && auto.proposals.length > 0 && (
        <ul className="mt-2 space-y-2" aria-label="Suggested decoding steps">
          {auto.proposals.map(({ proposal, steps }) => (
            <li key={steps.join(">")} className="p-2 bg-white border rounded">
              <div className="flex items-start justify-between gap-2">
                <div className="text-xs font-medium">{steps.join(" → ")}</div>
                <button
                  type="button"
                  onClick={() => auto.onApply(proposal)}
                  className="shrink-0 px-2 py-0.5 text-xs text-white bg-blue-500 rounded hover:bg-blue-600"
                >
                  Add to graph
                </button>
              </div>
              <div className="mt-1 text-xs text-gray-600 break-all line-clamp-2">
                {proposal.preview}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

interface InputOutputProps {
  input: string;
  output: string;
  status: string;
  /** Saves the selected output; absent when there is nothing to save. */
  onDownload?: () => void;
  auto?: AutoMode;
  onInputChange: (value: string) => void;
  calculate: () => void;
  autoCalculate: boolean;
  onAutoCalculateChange: (checked: boolean) => void;
  selectedNodeId?: string;
  selectedNodeTitle?: string;
  inputs: { id: string; label: string }[];
  selectedInputId?: string;
  onInputTabSelect: (nodeId: string) => void;
}

const InputOutput = React.memo<InputOutputProps>(
  ({
    input,
    output,
    status,
    onDownload,
    auto,
    onInputChange,
    calculate,
    autoCalculate,
    onAutoCalculateChange,
    selectedNodeId,
    selectedNodeTitle,
    inputs,
    selectedInputId,
    onInputTabSelect,
  }) => {
    return (
      <div className="flex flex-col h-full">
        <div className="h-1/2 p-4">
          <h3 className="text-lg font-semibold mb-2">Input</h3>
          {inputs.length > 1 && (
            <div className="flex gap-2 mb-2">
              {inputs.map((node) => (
                <button
                  key={node.id}
                  onClick={() => onInputTabSelect(node.id)}
                  className={`px-3 py-1 rounded ${
                    selectedInputId === node.id
                      ? "bg-blue-500 text-white"
                      : "bg-gray-100 hover:bg-gray-200"
                  }`}
                >
                  {node.label}
                </button>
              ))}
            </div>
          )}
          <Textarea
            value={input}
            onChange={(e) => onInputChange(e.target.value)}
            disabled={!selectedInputId}
            className="w-full h-[calc(100%-4rem)]"
            placeholder="Enter your input here..."
            aria-label="Input text"
          />
        </div>
        <div className="h-1/2 p-4 bg-gray-50">
          <h3 className="text-lg font-semibold mb-2 flex items-center gap-4">
            Output{" "}
            <span className="text-xs font-normal text-gray-500" aria-live="polite">
              {status}
            </span>
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <TooltipProvider>
                  <Tooltip delayDuration={100}>
                    <TooltipTrigger asChild>
                      <div className="flex items-center gap-2">
                        <Checkbox
                          id="auto-calculate"
                          checked={autoCalculate}
                          onCheckedChange={onAutoCalculateChange}
                        />
                        <Label
                          htmlFor="auto-calculate"
                          className="text-sm font-normal"
                        >
                          Auto-calculate
                        </Label>
                      </div>
                    </TooltipTrigger>
                    <TooltipContent className="bg-zinc-900 text-zinc-50 px-3 py-2 rounded-md shadow-lg border border-zinc-800 max-w-sm">
                      Automatically calculates the output whenever the input
                      changes. It should be disabled if the calculation is too
                      slow or heavy for the system.
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              </div>
              {!autoCalculate && (
                <TooltipProvider>
                  <Tooltip delayDuration={100}>
                    <TooltipTrigger asChild>
                      <button
                        onClick={calculate}
                        className="px-4 py-1 bg-blue-500 text-white rounded-full hover:bg-blue-600 transition-colors"
                      >
                        Calculate
                      </button>
                    </TooltipTrigger>
                    <TooltipContent className="bg-zinc-900 text-zinc-50 px-3 py-2 rounded-md shadow-lg border border-zinc-800 max-w-sm">
                      Click to manually calculate the output based on the
                      current input. Unnecessary if auto-calculate is enabled.
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              )}
            </div>
          </h3>
          {selectedNodeId && (
            <div className="text-sm text-gray-700 mb-2 flex items-center justify-between gap-2">
              <span>
                {selectedNodeTitle} ({selectedNodeId}):
              </span>
              <button
                type="button"
                onClick={onDownload}
                disabled={!onDownload}
                className="px-2 py-0.5 text-xs border rounded bg-white hover:bg-gray-100 disabled:opacity-50"
              >
                Download
              </button>
            </div>
          )}
          {auto && <AutoPanel auto={auto} />}
          <Textarea
            value={output}
            readOnly
            className="w-full h-[calc(100%-4rem)] bg-white"
            placeholder="Output will appear here..."
            aria-label="Output text"
            aria-live="polite"
          />
        </div>
      </div>
    );
  }
);

InputOutput.displayName = "InputOutput";

export default InputOutput;
