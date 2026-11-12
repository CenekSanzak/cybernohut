/**
 * Undo and redo for the graph: a wrapper around the graph reducer that remembers the
 * states before each change.
 */
import { graphReducer, type GraphAction, type GraphState } from "./graphReducer";

const MAX_STEPS = 100;

export interface History {
  past: GraphState[];
  present: GraphState;
  future: GraphState[];
  /**
   * Identifies the last recorded change when it is one of a run that should undo as a
   * whole: typing into one field, or dragging one node.
   */
  run: string | null;
}

export type HistoryAction = GraphAction | { type: "UNDO" } | { type: "REDO" };

export function createHistory(present: GraphState): History {
  return { past: [], present, future: [], run: null };
}

/**
 * How an action relates to history. `skip`: not something to undo (selection, a node
 * being measured). `record`: a step of its own. A string: part of a run, recorded once
 * for the whole run.
 */
function classify(action: GraphAction): "skip" | "record" | string {
  switch (action.type) {
    case "SELECT":
    case "SET_FILE":
      return "skip";
    case "SET_INPUT_TEXT":
      return `text:${action.nodeId}`;
    case "SET_CONFIG":
      return `config:${action.nodeId}:${action.key}`;
    case "APPLY_NODE_CHANGES": {
      if (action.changes.some((change) => change.type === "remove")) return "record";
      const moved = action.changes.find((change) => change.type === "position");
      return moved?.type === "position" ? `move:${moved.id}` : "skip";
    }
    case "APPLY_EDGE_CHANGES":
      return action.changes.some((change) => change.type === "remove") ? "record" : "skip";
    default:
      return "record";
  }
}

/** A drag ends with a position change that is no longer dragging. */
function endsRun(action: GraphAction): boolean {
  return (
    action.type === "APPLY_NODE_CHANGES" &&
    action.changes.some((change) => change.type === "position" && !change.dragging)
  );
}

export function historyReducer(history: History, action: HistoryAction): History {
  if (action.type === "UNDO") {
    const previous = history.past[history.past.length - 1];
    if (!previous) return history;
    return {
      past: history.past.slice(0, -1),
      present: previous,
      future: [history.present, ...history.future],
      run: null,
    };
  }
  if (action.type === "REDO") {
    const [next, ...future] = history.future;
    if (!next) return history;
    return { past: [...history.past, history.present], present: next, future, run: null };
  }

  const present = graphReducer(history.present, action);
  if (present === history.present) return history;
  const kind = classify(action);
  if (kind === "skip") return { ...history, present };

  const run = kind === "record" || endsRun(action) ? null : kind;
  // Later changes of a run replace the present without adding a step.
  if (kind !== "record" && kind === history.run) return { ...history, present, run };
  return {
    past: [...history.past, history.present].slice(-MAX_STEPS),
    present,
    future: [],
    run,
  };
}
