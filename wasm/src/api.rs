//! The wasm boundary.
//!
//! Two kinds of data cross it, by two different routes:
//!
//! - Control data (the catalog, the graph description, per-node reports, errors) is
//!   small and structured, so it goes through serde and gets generated TypeScript types.
//! - Payloads stay in wasm memory. An evaluation keeps the values it produced, up to a
//!   memory budget, and JavaScript copies one out only when it has to show it.

use std::cell::RefCell;
use std::rc::Rc;

use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::auto::{self, Proposal};
use crate::core::{OpMeta, Value, ValueType};
use crate::graph::{self, Evaluation, Graph, NodeReport, OutputText, Retention, Sources};
use crate::ops;

thread_local! {
    /// The most recent evaluation. Replacing it frees the previous one's values.
    static LAST: RefCell<Evaluation> = RefCell::new(Evaluation::default());
    /// The bytes behind file nodes. They outlive evaluations, so a file is copied into
    /// wasm memory once, not on every run.
    static SOURCES: RefCell<Sources> = RefCell::new(Sources::new());
}

/// Results kept after a run so any node's output can be shown at once. Beyond this,
/// results no later step needs are released; see [`Retention`]. The address space is
/// 4 GB and a run also needs room for the step in flight, so this stays well below.
const RETAINED_BYTES: usize = 512 * 1024 * 1024;

fn to_js_error(error: impl std::fmt::Display) -> JsError {
    JsError::new(&error.to_string())
}

/// Every available operation, described for the UI.
#[wasm_bindgen]
pub fn catalog() -> Result<Vec<Ts<OpMeta>>, JsError> {
    ops::registry()
        .iter()
        .map(|entry| Ts::from_rust(&(entry.meta)()).map_err(to_js_error))
        .collect()
}

/// Whether an output of type `from` may be connected to an input of type `to`.
#[wasm_bindgen]
pub fn can_connect(from: Ts<ValueType>, to: Ts<ValueType>) -> Result<bool, JsError> {
    let from = from.to_rust().map_err(to_js_error)?;
    let to = to.to_rust().map_err(to_js_error)?;
    Ok(to.accepts(from))
}

/// Evaluates the graph and returns one report per node, in execution order.
///
/// Failures of individual nodes are part of the report. This only throws when the
/// graph description itself is malformed. `keep` names the nodes whose full output
/// will be asked for, so they are not released when memory runs short.
#[wasm_bindgen]
pub fn evaluate(
    graph: Ts<Graph>,
    preview_chars: usize,
    keep: Vec<String>,
) -> Result<Vec<Ts<NodeReport>>, JsError> {
    let graph = graph.to_rust().map_err(to_js_error)?;
    let retention = Retention { budget_bytes: RETAINED_BYTES, keep: &keep, preview_chars };
    // The previous run's values go first: both sets at once would not fit.
    LAST.with(|last| *last.borrow_mut() = Evaluation::default());
    let evaluation =
        SOURCES.with(|sources| graph::evaluate_within(&graph, &sources.borrow(), &retention));
    let reports = evaluation
        .report(preview_chars)
        .iter()
        .map(|report| Ts::from_rust(report).map_err(to_js_error))
        .collect();
    LAST.with(|last| *last.borrow_mut() = evaluation);
    reports
}

/// The displayable output of a node from the last evaluation, cut to `max_chars` so a
/// huge result cannot freeze the page that shows it.
#[wasm_bindgen]
pub fn output_text(node_id: &str, port: &str, max_chars: usize) -> Result<Option<Ts<OutputText>>, JsError> {
    LAST.with(|last| {
        last.borrow()
            .output(node_id, port)
            .map(|value| Ts::from_rust(&graph::display(value, max_chars)).map_err(to_js_error))
            .transpose()
    })
}

/// Auto mode: chains of decoding steps that make a node's output more readable, best
/// first. Empty when the output is already as readable as anything reachable from it.
#[wasm_bindgen]
pub fn suggest(node_id: &str, port: &str) -> Result<Vec<Ts<Proposal>>, JsError> {
    LAST.with(|last| {
        last.borrow()
            .output(node_id, port)
            .map(auto::suggest)
            .unwrap_or_default()
            .iter()
            .map(|proposal| Ts::from_rust(proposal).map_err(to_js_error))
            .collect()
    })
}

/// Stores the content of a file node.
#[wasm_bindgen]
pub fn set_source(node_id: String, bytes: Vec<u8>) {
    SOURCES.with(|sources| sources.borrow_mut().insert(node_id, Rc::new(Value::Bytes(bytes))));
}

/// Drops the content of every file node that is not in `node_ids`.
#[wasm_bindgen]
pub fn retain_sources(node_ids: Vec<String>) {
    SOURCES.with(|sources| sources.borrow_mut().retain(|id, _| node_ids.contains(id)));
}

/// The complete output of a node from the last evaluation, as bytes to save.
#[wasm_bindgen]
pub fn output_bytes(node_id: &str, port: &str) -> Option<Vec<u8>> {
    LAST.with(|last| last.borrow().output(node_id, port).map(graph::to_bytes))
}
