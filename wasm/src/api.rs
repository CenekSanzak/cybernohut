//! The wasm boundary.
//!
//! Two kinds of data cross it, by two different routes:
//!
//! - Control data (the catalog, the graph description, per-node reports, errors) is
//!   small and structured, so it goes through serde and gets generated TypeScript types.
//! - Payloads stay in wasm memory. An evaluation keeps every value it produced, and
//!   JavaScript copies one out only when it has to show it.

use std::cell::RefCell;

use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::core::{OpMeta, ValueType};
use crate::graph::{self, Evaluation, Graph, NodeReport, OutputText};
use crate::ops;

thread_local! {
    /// The most recent evaluation. Replacing it frees the previous one's values.
    static LAST: RefCell<Evaluation> = RefCell::new(Evaluation::default());
}

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
/// graph description itself is malformed.
#[wasm_bindgen]
pub fn evaluate(graph: Ts<Graph>, preview_chars: usize) -> Result<Vec<Ts<NodeReport>>, JsError> {
    let graph = graph.to_rust().map_err(to_js_error)?;
    let evaluation = graph::evaluate(&graph);
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
