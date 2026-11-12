import { useState } from "react";
import { Button } from "@/components/ui/button";
import { cases, makeInput, toBase64Bytes } from "./cases";
import { runCase, type BenchResult } from "./run";

const SIZES_MB = [1, 10, 50];

function formatMs(ms: number): string {
  return ms < 10 ? ms.toFixed(2) : ms < 100 ? ms.toFixed(1) : Math.round(ms).toString();
}

/** How the engine compares, phrased from the engine's point of view. */
function verdict(result: BenchResult): { text: string; className: string } {
  if (result.error) return { text: "failed", className: "text-red-700" };
  const ratio = result.nativeMs / result.engineMs;
  if (ratio >= 1.1) return { text: `${ratio.toFixed(1)}× faster`, className: "text-green-700" };
  if (ratio <= 0.9) return { text: `${(1 / ratio).toFixed(1)}× slower`, className: "text-red-700" };
  return { text: "about the same", className: "text-gray-600" };
}

function toMarkdown(results: BenchResult[], sizeMb: number, runs: number): string {
  const rows = results.map((result) =>
    result.error
      ? `| ${result.name} | failed | | | ${result.error} |`
      : `| ${result.name} | ${formatMs(result.engineMs)} | ${formatMs(result.nativeMs)} | ${
          verdict(result).text
        } | ${result.nativeLabel} |`
  );
  return [
    `${sizeMb} MB input, median of ${runs} runs. ${navigator.userAgent}`,
    "",
    "| Pipeline | Engine (ms) | Native (ms) | Engine is | Native side |",
    "| --- | ---: | ---: | --- | --- |",
    ...rows,
  ].join("\n");
}

export function Bench() {
  const [sizeMb, setSizeMb] = useState(10);
  const [results, setResults] = useState<BenchResult[]>([]);
  const [measured, setMeasured] = useState<{ sizeMb: number; runs: number } | null>(null);
  const [progress, setProgress] = useState("");
  const running = progress !== "";

  const run = async () => {
    const runs = sizeMb >= 50 ? 3 : 5;
    setResults([]);
    setMeasured({ sizeMb, runs });
    setProgress("Preparing input…");
    // Let the page paint before each long step.
    const paint = () => new Promise((resolve) => setTimeout(resolve, 30));
    await paint();

    const raw = makeInput(sizeMb);
    const inputs = { raw, base64: toBase64Bytes(raw) };
    const collected: BenchResult[] = [];
    for (const benchCase of cases) {
      setProgress(`Running: ${benchCase.name}`);
      await paint();
      collected.push(await runCase(benchCase, inputs[benchCase.source], runs));
      setResults([...collected]);
    }
    setProgress("");
  };

  return (
    <main className="min-h-screen bg-gray-100 p-8">
      <div className="max-w-4xl mx-auto space-y-6">
        <header>
          <a href="./" className="text-sm text-gray-600 underline">
            ← Back to the graph
          </a>
          <h1 className="text-2xl font-bold mt-2">Benchmark</h1>
          <p className="text-gray-700 mt-2">
            Each pipeline runs twice on the same input: once in the Rust engine, and once
            with the fastest equivalent this browser offers without it. Both sides start
            with the input already in memory, and their outputs are compared byte for
            byte. Numbers are the median of several runs on this machine.
          </p>
        </header>

        <div className="flex items-center gap-3">
          <label className="text-sm" htmlFor="size">
            Input size
          </label>
          <select
            id="size"
            className="border rounded px-2 py-1 bg-white"
            value={sizeMb}
            disabled={running}
            onChange={(event) => setSizeMb(Number(event.target.value))}
          >
            {SIZES_MB.map((size) => (
              <option key={size} value={size}>
                {size} MB
              </option>
            ))}
          </select>
          <Button onClick={run} disabled={running}>
            {running ? "Running…" : "Run"}
          </Button>
          <span className="text-sm text-gray-600" aria-live="polite">
            {progress}
          </span>
        </div>

        {results.length > 0 && measured && (
          <>
            <table className="w-full bg-white shadow-sm rounded text-sm">
              <thead>
                <tr className="text-left border-b">
                  <th className="p-3">Pipeline</th>
                  <th className="p-3 text-right">Engine (ms)</th>
                  <th className="p-3 text-right">Native (ms)</th>
                  <th className="p-3">Engine is</th>
                  <th className="p-3">Same output</th>
                </tr>
              </thead>
              <tbody>
                {results.map((result) => {
                  const { text, className } = verdict(result);
                  return (
                    <tr key={result.name} className="border-b last:border-0 align-top">
                      <td className="p-3">
                        <div className="font-medium">{result.name}</div>
                        <div className="text-xs text-gray-500">
                          {result.error ?? `vs ${result.nativeLabel}`}
                        </div>
                      </td>
                      <td className="p-3 text-right tabular-nums">
                        {result.error ? "–" : formatMs(result.engineMs)}
                      </td>
                      <td className="p-3 text-right tabular-nums">
                        {result.error ? "–" : formatMs(result.nativeMs)}
                      </td>
                      <td className={`p-3 font-medium ${className}`}>{text}</td>
                      <td className="p-3">
                        {result.error ? "–" : result.matches ? "Yes" : "No, outputs differ"}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>

            {!running && (
              <div className="flex items-center gap-3">
                <Button
                  variant="outline"
                  onClick={() =>
                    navigator.clipboard.writeText(
                      toMarkdown(results, measured.sizeMb, measured.runs)
                    )
                  }
                >
                  Copy as Markdown
                </Button>
                <span className="text-sm text-gray-600">
                  {measured.sizeMb} MB input, median of {measured.runs} runs
                </span>
              </div>
            )}
          </>
        )}

        <section className="text-sm text-gray-700 space-y-2">
          <h2 className="font-semibold">Reading the numbers</h2>
          <p>
            Engine time is measured inside the worker, around the whole graph evaluation.
            Native time is measured on this page. Neither includes getting the input into
            memory.
          </p>
          <p>
            Expect the engine to lose wherever the browser has a built-in written in
            optimized native code: SHA-256 in Web Crypto, and Base64 and hex on
            Uint8Array in recent browsers. It tends to win where the JavaScript
            equivalent has to build strings piece by piece, such as reversing text.
          </p>
        </section>
      </div>
    </main>
  );
}
