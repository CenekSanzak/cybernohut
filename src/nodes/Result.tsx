import type { NodeReport } from "@/engine/wasm";
import { needsSetup } from "./report";

/** What a node shows of its result: the start of the output, or why there is none. */
export function Result({ report }: { report: NodeReport | undefined }) {
  if (!report) return null;
  switch (report.status) {
    case "ok":
      return report.outputs.map((output) => (
        <div key={output.port} className="break-all text-gray-600">
          {report.outputs.length > 1 && <span className="font-medium">{output.port}: </span>}
          {output.preview}
          {output.truncated && "…"}
        </div>
      ));
    case "error":
      return needsSetup(report) ? (
        <div className="font-sans text-amber-700">{report.error.message}</div>
      ) : (
        <div className="font-sans text-destructive" role="alert">
          {report.error.message}
        </div>
      );
    case "waiting":
      return <div className="font-sans text-muted-foreground">Not connected</div>;
    case "blocked":
      return <div className="font-sans text-muted-foreground">Waiting for an earlier step</div>;
  }
}
