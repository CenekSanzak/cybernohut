import { Wand2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { AutoMode } from "@/graph/useAutoMode";

export function AutoButton({ auto }: { auto: AutoMode }) {
  return (
    <Button
      variant="outline"
      size="sm"
      onClick={auto.suggest}
      disabled={!auto.ready || auto.busy}
      title={`Look for decoding steps for the output of ${auto.target}`}
    >
      <Wand2 />
      {auto.busy ? "Looking…" : "Suggest steps"}
    </Button>
  );
}

/** The proposals auto mode came back with, or a note that there were none. */
export function AutoProposals({ auto }: { auto: AutoMode }) {
  if (!auto.proposals) return null;
  if (auto.proposals.length === 0) {
    return (
      <p className="mb-2 text-xs text-muted-foreground" role="status">
        Nothing found that makes the output of {auto.target} more readable than it already is.
      </p>
    );
  }
  return (
    <ul
      className="mb-2 max-h-44 space-y-2 overflow-y-auto"
      aria-label={`Suggested decoding steps for ${auto.target}`}
    >
      {auto.proposals.map(({ proposal, steps }) => (
        <li key={steps.join(">")} className="rounded-md border bg-white p-2">
          <div className="flex items-start justify-between gap-2">
            <div className="text-xs font-medium">{steps.join(" → ")}</div>
            <Button size="sm" className="h-6 shrink-0 px-2" onClick={() => auto.apply(proposal)}>
              Add to graph
            </Button>
          </div>
          <div className="mt-1 line-clamp-2 break-all font-mono text-xs text-muted-foreground">
            {proposal.preview}
          </div>
        </li>
      ))}
    </ul>
  );
}
