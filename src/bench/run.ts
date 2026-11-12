import { engine } from "@/engine/client";
import { getMeta, type Graph } from "@/engine/wasm";
import { asBytes, type BenchCase } from "./cases";

export interface BenchResult {
  name: string;
  nativeLabel: string;
  /** Median milliseconds inside the engine worker. */
  engineMs: number;
  /** Median milliseconds for the browser-native equivalent. */
  nativeMs: number;
  /** Whether both sides produced identical bytes. */
  matches: boolean;
  error?: string;
}

const SOURCE_ID = "source";

/** A straight chain: the file source, then each step feeding the next. */
function toGraph(benchCase: BenchCase): Graph {
  const graph: Graph = { nodes: [{ kind: "file", id: SOURCE_ID }], edges: [] };
  let previous = { id: SOURCE_ID, port: "output" };
  benchCase.steps.forEach((step, index) => {
    const meta = getMeta(step.op);
    const id = `step-${index}`;
    graph.nodes.push({
      kind: "op",
      id,
      op_id: step.op,
      config: { ...meta.default_config, ...step.config },
      per_line: false,
    });
    graph.edges.push({
      source: previous.id,
      source_port: previous.port,
      target: id,
      target_port: meta.inputs[0].name,
    });
    previous = { id, port: meta.outputs[0].name };
  });
  return graph;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

/**
 * Times one pipeline on both sides. The input is already in memory on both: the engine
 * holds it as a file source and the native side gets the same bytes, so neither timing
 * includes loading or copying the input.
 */
export async function runCase(
  benchCase: BenchCase,
  input: Uint8Array,
  runs: number
): Promise<BenchResult> {
  const base = { name: benchCase.name, nativeLabel: benchCase.nativeLabel };
  try {
    engine.setSource(SOURCE_ID, input);
    const graph = toGraph(benchCase);
    const last = graph.nodes[graph.nodes.length - 1].id;
    const lastPort = getMeta(benchCase.steps[benchCase.steps.length - 1].op).outputs[0].name;

    const engineTimes: number[] = [];
    for (let run = 0; run < runs; run++) {
      const { reports, elapsedMs } = await engine.evaluate(graph);
      const report = reports.get(last);
      if (report?.status !== "ok") {
        const reason = report?.status === "error" ? report.error.message : report?.status;
        throw new Error(`Engine: ${reason ?? "no result"}`);
      }
      engineTimes.push(elapsedMs);
    }
    const engineOutput = await engine.outputBytes(last, lastPort);

    const nativeTimes: number[] = [];
    let nativeOutput: Uint8Array | string = "";
    for (let run = 0; run < runs; run++) {
      const started = performance.now();
      nativeOutput = await benchCase.native(input);
      nativeTimes.push(performance.now() - started);
    }

    return {
      ...base,
      engineMs: median(engineTimes),
      nativeMs: median(nativeTimes),
      matches: !!engineOutput && sameBytes(engineOutput, asBytes(nativeOutput)),
    };
  } catch (error) {
    return {
      ...base,
      engineMs: NaN,
      nativeMs: NaN,
      matches: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
