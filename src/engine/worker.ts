/// The engine worker: owns the wasm instance that evaluates graphs and holds their values.
import * as wasm from "wasm";
import type {
  EngineNotice,
  EngineReady,
  EngineRequest,
  EngineResponse,
} from "./protocol";

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
    case "bytes":
      return {
        type: "bytes",
        id: request.id,
        bytes: wasm.output_bytes(request.nodeId, request.port),
      };
  }
}

self.onmessage = (event: MessageEvent<EngineRequest | EngineNotice>) => {
  const message = event.data;
  if (message.type === "set_source") {
    wasm.set_source(message.nodeId, message.bytes);
    return;
  }
  if (message.type === "retain_sources") {
    wasm.retain_sources(message.nodeIds);
    return;
  }

  let response: EngineResponse;
  try {
    response = handle(message);
  } catch (error) {
    // A wasm trap (typically running out of memory) leaves the instance unusable; the
    // page replaces this worker when it sees a failure.
    response = {
      type: "failed",
      id: message.id,
      message: error instanceof Error ? error.message : String(error),
    };
  }
  // Hand large results over instead of copying them a second time.
  const transfer = response.type === "bytes" && response.bytes ? [response.bytes.buffer] : [];
  self.postMessage(response, { transfer });
};

self.postMessage({ type: "ready" } satisfies EngineReady);
