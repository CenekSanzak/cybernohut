/**
 * The page's handle on the engine worker.
 *
 * Only the latest evaluation matters. If a new one is requested while another is still
 * running, the worker is replaced: that cancels the running work immediately instead
 * of queueing behind it. The same replacement recovers from a crashed engine.
 */
import type { Graph, NodeReport, OutputText } from "wasm";
import type { EngineReady, EngineRequest, EngineResponse } from "./protocol";

/** Rejection for a request that was abandoned in favour of a newer one. */
export class Superseded extends Error {
  constructor() {
    super("Superseded by a newer request");
  }
}

export interface Evaluated {
  reports: ReadonlyMap<string, NodeReport>;
  elapsedMs: number;
}

interface Pending {
  resolve: (response: EngineResponse) => void;
  reject: (error: Error) => void;
}

type WithoutId<T> = T extends unknown ? Omit<T, "id"> : never;

class Engine {
  private pending = new Map<number, Pending>();
  private nextId = 1;
  private evaluating = false;
  private markReady = () => {};
  private ready = Promise.resolve();
  private worker = this.spawn();

  private spawn(): Worker {
    const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    this.ready = new Promise((resolve) => (this.markReady = resolve));
    worker.onmessage = (event: MessageEvent<EngineResponse | EngineReady>) => {
      const response = event.data;
      if (response.type === "ready") {
        this.markReady();
        return;
      }
      const pending = this.pending.get(response.id);
      this.pending.delete(response.id);
      if (response.type === "failed") {
        this.replace(new Error(response.message));
        pending?.reject(new Error(response.message));
      } else {
        pending?.resolve(response);
      }
    };
    worker.onerror = (event) => this.replace(new Error(event.message || "The engine crashed"));
    return worker;
  }

  /** Stops the current worker, failing whatever it was doing, and starts a fresh one. */
  private replace(reason: Error): void {
    this.worker.terminate();
    const abandoned = [...this.pending.values()];
    this.pending.clear();
    this.evaluating = false;
    this.worker = this.spawn();
    abandoned.forEach((pending) => pending.reject(reason));
  }

  private request(request: WithoutId<EngineRequest>): Promise<EngineResponse> {
    const id = this.nextId++;
    const worker = this.worker;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      // If the worker was replaced while it was starting, this request was already
      // rejected and must not reach the new one.
      void this.ready.then(() => {
        if (worker === this.worker && this.pending.has(id)) {
          worker.postMessage({ ...request, id });
        }
      });
    });
  }

  /** Evaluates the graph. A node that fails is part of the result, not a rejection. */
  async evaluate(graph: Graph): Promise<Evaluated> {
    if (this.evaluating) this.replace(new Superseded());
    this.evaluating = true;
    const response = await this.request({ type: "evaluate", graph });
    this.evaluating = false;
    if (response.type !== "evaluated") throw new Error("Unexpected engine response");
    return {
      reports: new Map(response.reports.map((report) => [report.id, report])),
      elapsedMs: response.elapsedMs,
    };
  }

  /** The output of a node from the most recent evaluation, cut to a displayable size. */
  async output(nodeId: string, port: string): Promise<OutputText | undefined> {
    const response = await this.request({ type: "output", nodeId, port });
    if (response.type !== "output") throw new Error("Unexpected engine response");
    return response.output;
  }
}

export const engine = new Engine();
