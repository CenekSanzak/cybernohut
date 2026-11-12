import type { NodeReport } from "@/engine/wasm";

/**
 * A step whose settings are incomplete, such as a cipher with no key yet. That is
 * something to fill in, not a failure, and is shown differently from one.
 */
export function needsSetup(report: NodeReport | undefined): boolean {
  return report?.status === "error" && report.error.code === "invalid_config";
}
