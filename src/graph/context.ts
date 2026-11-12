import { createContext, type Dispatch } from "react";
import type { NodeReport } from "@/engine/wasm";
import type { GraphAction } from "@/state/graphReducer";

/** Results of the latest evaluation, by node id. Kept apart from the graph itself. */
export const ReportsContext = createContext<ReadonlyMap<string, NodeReport>>(new Map());

export const GraphDispatchContext = createContext<Dispatch<GraphAction>>(() => {});
