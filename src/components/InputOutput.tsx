import React from "react";
import { Download } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import type { AutoMode } from "@/graph/useAutoMode";
import { AutoButton, AutoProposals } from "./AutoPanel";

interface InputOutputProps {
  input: string;
  onInputChange: (value: string) => void;
  inputs: { id: string; label: string }[];
  selectedInputId?: string;
  onInputTabSelect: (nodeId: string) => void;
  output: string;
  /** How the last run went, e.g. its duration. */
  status: string;
  /** Name of the step whose output is shown, if one is selected. */
  selectedNodeTitle?: string;
  /** Saves the selected output; absent when there is nothing to save. */
  onDownload?: () => void;
  auto?: AutoMode;
}

const InputOutput = React.memo<InputOutputProps>(
  ({
    input,
    onInputChange,
    inputs,
    selectedInputId,
    onInputTabSelect,
    output,
    status,
    selectedNodeTitle,
    onDownload,
    auto,
  }) => (
    <div className="flex h-full min-h-0 flex-col">
      <section className="flex min-h-0 flex-1 flex-col p-4" aria-labelledby="input-heading">
        <div className="mb-2 flex items-center gap-3">
          <h2 id="input-heading" className="text-sm font-semibold">
            Input
          </h2>
          {inputs.length > 1 && (
            <div className="flex gap-1" role="tablist" aria-label="Inputs">
              {inputs.map((node) => (
                <button
                  key={node.id}
                  role="tab"
                  aria-selected={selectedInputId === node.id}
                  onClick={() => onInputTabSelect(node.id)}
                  className={`rounded px-2 py-0.5 text-xs ${
                    selectedInputId === node.id
                      ? "bg-primary text-primary-foreground"
                      : "bg-secondary hover:bg-accent"
                  }`}
                >
                  {node.label}
                </button>
              ))}
            </div>
          )}
        </div>
        <Textarea
          value={input}
          onChange={(e) => onInputChange(e.target.value)}
          disabled={!selectedInputId}
          className="min-h-0 flex-1 resize-none font-mono text-sm"
          placeholder={
            selectedInputId ? "Type or paste the text to transform" : "Add an Input from the I/O group"
          }
          aria-label="Input text"
          spellCheck={false}
        />
      </section>

      <section
        className="flex min-h-0 flex-1 flex-col border-t bg-gray-50 p-4"
        aria-labelledby="output-heading"
      >
        <div className="mb-2 flex items-baseline gap-2">
          <h2 id="output-heading" className="text-sm font-semibold">
            Output
          </h2>
          {selectedNodeTitle && (
            <span className="truncate text-sm text-muted-foreground">of {selectedNodeTitle}</span>
          )}
          <span className="ml-auto shrink-0 text-xs text-muted-foreground" aria-live="polite">
            {status}
          </span>
        </div>
        <div className="mb-2 flex flex-wrap gap-2">
          {auto && <AutoButton auto={auto} />}
          <Button variant="outline" size="sm" onClick={onDownload} disabled={!onDownload}>
            <Download />
            Download
          </Button>
        </div>
        {auto && <AutoProposals auto={auto} />}
        <Textarea
          value={output}
          readOnly
          className="min-h-0 flex-1 resize-none bg-white font-mono text-sm"
          placeholder={
            selectedNodeTitle
              ? "This step has no output yet"
              : "Pick an operation on the left; the output of the selected step appears here"
          }
          aria-label="Output text"
        />
      </section>
    </div>
  )
);

InputOutput.displayName = "InputOutput";

export default InputOutput;
