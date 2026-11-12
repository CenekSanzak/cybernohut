/// The engine worker: owns the wasm instance that evaluates graphs and holds their values.
import * as wasm from "wasm";
import type { EngineReady, EngineRequest, EngineResponse } from "./protocol";

/** How many characters of each output a node shows. */
const PREVIEW_CHARS = 60;
/** How many characters of the selected output the page receives. */
const OUTPUT_CHARS = 200_000;

function handle(request: EngineRequest): EngineResponse {
  switch (request.type) {
    case "evaluate": {
      const started = performance.now();
      const reports = wasm.evaluate(request.graph, PREVIEW_CHARS);
      return {
        type: "evaluated",
        id: request.id,
        reports,
        elapsedMs: performance.now() - started,
      };
    }
    case "output":
      return {
        type: "output",
        id: request.id,
        output: wasm.output_text(request.nodeId, request.port, OUTPUT_CHARS),
      };
  }
}

self.onmessage = (event: MessageEvent<EngineRequest>) => {
  let response: EngineResponse;
  try {
    response = handle(event.data);
  } catch (error) {
    // A wasm trap (typically running out of memory) leaves the instance unusable; the
    // page replaces this worker when it sees a failure.
    response = {
      type: "failed",
      id: event.data.id,
      message: error instanceof Error ? error.message : String(error),
    };
  }
  self.postMessage(response);
};

self.postMessage({ type: "ready" } satisfies EngineReady);
