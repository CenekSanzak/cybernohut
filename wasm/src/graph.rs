//! Evaluates a whole graph in one call.
//!
//! The UI describes the graph as plain data (node ids, operation ids, configs, edges)
//! and gets back one report per node. Values produced along the way stay here; the UI
//! receives short previews and asks for a full output only when it shows one.

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{ErrorCode, OpEntry, OpError, OpMeta, Value, ValueType};
use crate::ops;

#[derive(Debug, Clone, Deserialize, Serialize, Tsify)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNode {
    /// A source of text typed by the user.
    Input { id: String, text: String },
    /// A source of raw bytes. The bytes are too large to travel inside the graph
    /// description, so they are handed to the engine separately, keyed by node id.
    File { id: String },
    Op {
        id: String,
        op_id: String,
        #[tsify(type = "Record<string, string | number | boolean>")]
        config: serde_json::Value,
        /// Apply the operation to each line separately and join the results.
        #[serde(default)]
        per_line: bool,
    },
}

impl GraphNode {
    pub fn id(&self) -> &str {
        match self {
            GraphNode::Input { id, .. } | GraphNode::File { id } | GraphNode::Op { id, .. } => id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Tsify)]
pub struct GraphEdge {
    pub source: String,
    pub source_port: String,
    pub target: String,
    pub target_port: String,
}

/// Name of the single output port of an input or file node.
pub const INPUT_NODE_PORT: &str = "output";

/// The bytes behind file nodes, by node id.
pub type Sources = HashMap<String, Rc<Value>>;

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NodeReport {
    /// `retained` is false when the node ran but its output was let go to save
    /// memory; only the preview is left, and the graph has to run again to show more.
    Ok { id: String, outputs: Vec<OutputPreview>, retained: bool },
    Error { id: String, error: OpError },
    /// An input port has nothing connected to it yet.
    Waiting { id: String },
    /// A node upstream failed or is itself waiting.
    Blocked { id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct OutputPreview {
    pub port: String,
    #[serde(rename = "type")]
    pub ty: ValueType,
    /// Bytes for text and bytes, items for lists, 1 for a number.
    pub size: usize,
    pub preview: String,
    pub truncated: bool,
}

pub enum Outcome {
    Ok(Vec<(String, Rc<Value>)>),
    /// Succeeded, and the values have since been released. The previews remain.
    Released(Vec<OutputPreview>),
    Error(OpError),
    Waiting,
    Blocked,
}

/// The values of one evaluation, kept so the UI can fetch full outputs afterwards.
#[derive(Default)]
pub struct Evaluation {
    pub order: Vec<String>,
    pub outcomes: HashMap<String, Outcome>,
}

impl Evaluation {
    pub fn output(&self, node_id: &str, port: &str) -> Option<&Value> {
        match self.outcomes.get(node_id)? {
            Outcome::Ok(outputs) => {
                outputs.iter().find(|(name, _)| name == port).map(|(_, value)| value.as_ref())
            }
            _ => None,
        }
    }

    pub fn report(&self, preview_chars: usize) -> Vec<NodeReport> {
        self.order
            .iter()
            .map(|id| {
                let id = id.clone();
                match &self.outcomes[&id] {
                    Outcome::Ok(outputs) => NodeReport::Ok {
                        id,
                        outputs: previews(outputs, preview_chars),
                        retained: true,
                    },
                    Outcome::Released(outputs) => {
                        NodeReport::Ok { id, outputs: outputs.clone(), retained: false }
                    }
                    Outcome::Error(error) => NodeReport::Error { id, error: error.clone() },
                    Outcome::Waiting => NodeReport::Waiting { id },
                    Outcome::Blocked => NodeReport::Blocked { id },
                }
            })
            .collect()
    }
}

fn previews(outputs: &[(String, Rc<Value>)], preview_chars: usize) -> Vec<OutputPreview> {
    outputs.iter().map(|(port, value)| preview(port, value, preview_chars)).collect()
}

/// How much an evaluation may hold on to once it is done.
///
/// Keeping every intermediate result makes any node's output available at once, but a
/// chain of six steps over a 200 MB file is then 1.2 GB. Past the budget, results that
/// no later step needs are released, and a step that can work in place is given its
/// input's memory instead of allocating its own.
pub struct Retention<'a> {
    /// Bytes of results to keep. Nothing is released below this.
    pub budget_bytes: usize,
    /// Nodes whose results are kept whatever the budget: the ones being looked at.
    pub keep: &'a [String],
    /// Released results leave a preview of this many characters behind.
    pub preview_chars: usize,
}

impl Retention<'_> {
    pub const EVERYTHING: Retention<'static> =
        Retention { budget_bytes: usize::MAX, keep: &[], preview_chars: 0 };
}

fn held_bytes(value: &Value) -> usize {
    match value {
        Value::Text(text) => text.len(),
        Value::Bytes(bytes) => bytes.len(),
        Value::Number(_) => 8,
        Value::TextList(items) => items.iter().map(String::len).sum(),
        Value::NumberList(items) => items.len() * 8,
    }
}

/// Evaluates the graph and keeps every result.
pub fn evaluate(graph: &Graph, sources: &Sources) -> Evaluation {
    evaluate_within(graph, sources, &Retention::EVERYTHING)
}

pub fn evaluate_within(graph: &Graph, sources: &Sources, retention: &Retention) -> Evaluation {
    let index: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id(), n)).collect();
    let mut incoming: HashMap<&str, Vec<&GraphEdge>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = graph.nodes.iter().map(|n| (n.id(), 0)).collect();
    // How many steps have yet to read each node's results.
    let mut readers_left: HashMap<&str, usize> = HashMap::new();

    for edge in &graph.edges {
        // Edges that point at missing nodes are leftovers of a deletion; ignore them.
        if !index.contains_key(edge.source.as_str()) || !index.contains_key(edge.target.as_str()) {
            continue;
        }
        incoming.entry(&edge.target).or_default().push(edge);
        outgoing.entry(&edge.source).or_default().push(&edge.target);
        *in_degree.get_mut(edge.target.as_str()).expect("target was indexed") += 1;
        *readers_left.entry(&edge.source).or_default() += 1;
    }

    // A file's bytes belong to the page, not the evaluation, so releasing them here
    // would free nothing.
    let releasable = |id: &str| {
        !retention.keep.iter().any(|kept| kept == id) && !matches!(index[id], GraphNode::File { .. })
    };
    let release = |evaluation: &mut Evaluation, id: &str| -> usize {
        let Some(Outcome::Ok(outputs)) = evaluation.outcomes.get(id) else { return 0 };
        let freed = outputs.iter().map(|(_, value)| held_bytes(value)).sum();
        let left_behind = Outcome::Released(previews(outputs, retention.preview_chars));
        evaluation.outcomes.insert(id.to_owned(), left_behind);
        freed
    };

    // Kahn's algorithm, seeded in declaration order so the result is deterministic.
    let mut queue: VecDeque<&str> =
        graph.nodes.iter().map(GraphNode::id).filter(|id| in_degree[id] == 0).collect();
    let mut evaluation = Evaluation::default();
    let mut held = 0usize;

    while let Some(id) = queue.pop_front() {
        let edges = incoming.get(id);

        // Short of memory, a step that is the last reader of its only input is given
        // that input to rewrite in place.
        let mut handed_over = None;
        if let (GraphNode::Op { per_line: false, .. }, Some([edge])) = (index[id], edges.map(Vec::as_slice)) {
            let source = edge.source.as_str();
            let size = match evaluation.outcomes.get(source) {
                Some(Outcome::Ok(outputs)) if outputs.len() == 1 => held_bytes(&outputs[0].1),
                _ => 0,
            };
            let last_reader = readers_left.get(source) == Some(&1);
            if size > 0 && last_reader && releasable(source) && held + size > retention.budget_bytes {
                let left_behind = match &evaluation.outcomes[source] {
                    Outcome::Ok(outputs) => Outcome::Released(previews(outputs, retention.preview_chars)),
                    _ => unreachable!("checked above"),
                };
                if let Some(Outcome::Ok(mut outputs)) = evaluation.outcomes.insert(source.to_owned(), left_behind) {
                    held -= size;
                    let (_, value) = outputs.remove(0);
                    handed_over = Some(Rc::try_unwrap(value).unwrap_or_else(|shared| (*shared).clone()));
                }
            }
        }

        let outcome = evaluate_node(index[id], edges, sources, &evaluation, handed_over);
        if let (Outcome::Ok(outputs), false) = (&outcome, matches!(index[id], GraphNode::File { .. })) {
            held += outputs.iter().map(|(_, value)| held_bytes(value)).sum::<usize>();
        }
        evaluation.order.push(id.to_owned());
        evaluation.outcomes.insert(id.to_owned(), outcome);

        for edge in edges.into_iter().flatten() {
            if let Some(left) = readers_left.get_mut(edge.source.as_str()) {
                *left -= 1;
            }
        }
        if held > retention.budget_bytes {
            // Oldest first: what was computed early is least likely to be looked at.
            let finished: Vec<String> = evaluation
                .order
                .iter()
                .filter(|done| readers_left.get(done.as_str()).is_none_or(|left| *left == 0))
                .filter(|done| releasable(done))
                .cloned()
                .collect();
            for done in finished {
                if held <= retention.budget_bytes {
                    break;
                }
                held -= release(&mut evaluation, &done);
            }
        }

        for target in outgoing.get(id).into_iter().flatten() {
            let degree = in_degree.get_mut(target).expect("target was indexed");
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(target);
            }
        }
    }

    // Whatever was never reached sits on a cycle, or downstream of one.
    for node in &graph.nodes {
        if !evaluation.outcomes.contains_key(node.id()) {
            let error = OpError::new(ErrorCode::InvalidInput, "This node is part of a cycle");
            evaluation.order.push(node.id().to_owned());
            evaluation.outcomes.insert(node.id().to_owned(), Outcome::Error(error));
        }
    }

    evaluation
}

fn evaluate_node(
    node: &GraphNode,
    incoming: Option<&Vec<&GraphEdge>>,
    sources: &Sources,
    done: &Evaluation,
    handed_over: Option<Value>,
) -> Outcome {
    let (op_id, config, per_line) = match node {
        GraphNode::Input { text, .. } => {
            return Outcome::Ok(vec![(INPUT_NODE_PORT.into(), Rc::new(Value::Text(text.clone())))]);
        }
        GraphNode::File { id } => {
            return match sources.get(id) {
                Some(bytes) => Outcome::Ok(vec![(INPUT_NODE_PORT.into(), bytes.clone())]),
                None => Outcome::Error(OpError::invalid_input("No file loaded. Choose a file.")),
            };
        }
        GraphNode::Op { op_id, config, per_line, .. } => (op_id, config, *per_line),
    };
    let Some(entry) = ops::find(op_id) else {
        return Outcome::Error(OpError::unknown_op(op_id));
    };
    let meta = (entry.meta)();

    if let Some(value) = handed_over {
        let port = &meta.inputs[0];
        if !port.ty.accepts(value.value_type()) {
            return Outcome::Error(mismatch(&port.name, port.ty, value.value_type()));
        }
        let result = (entry.take_over)(config, value)
            .unwrap_or_else(|value| (entry.run)(config, &[&value]));
        return finish(&meta, result);
    }

    let mut inputs: Vec<Rc<Value>> = Vec::with_capacity(meta.inputs.len());
    for port in &meta.inputs {
        let edge = incoming.and_then(|edges| edges.iter().find(|e| e.target_port == port.name));
        let Some(edge) = edge else { return Outcome::Waiting };
        let value = match done.outcomes.get(&edge.source) {
            Some(Outcome::Ok(outputs)) => {
                outputs.iter().find(|(name, _)| *name == edge.source_port).map(|(_, v)| v.clone())
            }
            _ => return Outcome::Blocked,
        };
        let Some(value) = value else {
            return Outcome::Error(OpError::new(
                ErrorCode::InvalidInput,
                format!("The connected node has no output named \"{}\"", edge.source_port),
            ));
        };
        if !port.ty.accepts(value.value_type()) {
            return Outcome::Error(mismatch(&port.name, port.ty, value.value_type()));
        }
        inputs.push(value);
    }

    let refs: Vec<&Value> = inputs.iter().map(Rc::as_ref).collect();
    let result = if per_line {
        run_per_line(entry, &meta, config, &refs)
    } else {
        (entry.run)(config, &refs)
    };
    finish(&meta, result)
}

fn finish(meta: &OpMeta, result: Result<Vec<Value>, OpError>) -> Outcome {
    match result {
        Ok(values) => Outcome::Ok(
            meta.outputs.iter().map(|p| p.name.clone()).zip(values.into_iter().map(Rc::new)).collect(),
        ),
        Err(error) => Outcome::Error(error),
    }
}

fn mismatch(port: &str, expected: ValueType, got: ValueType) -> OpError {
    let hint = if expected == ValueType::Text && got == ValueType::Bytes {
        " Add a \"Decode Text (UTF-8)\" step in between."
    } else {
        ""
    };
    OpError::new(
        ErrorCode::TypeMismatch,
        format!("\"{port}\" expects {} but is connected to {}.{hint}", expected.label(), got.label()),
    )
}

fn run_per_line(
    entry: &OpEntry,
    meta: &OpMeta,
    config: &serde_json::Value,
    inputs: &[&Value],
) -> Result<Vec<Value>, OpError> {
    if !meta.supports_per_line {
        return Err(OpError::invalid_config(format!("{} cannot run line by line", meta.name)));
    }
    // Text is split into text lines; raw bytes are split on the newline byte.
    let lines: Vec<Value> = match inputs {
        [Value::Text(text)] => text.split('\n').map(|line| Value::Text(line.to_owned())).collect(),
        [Value::Bytes(bytes)] => {
            bytes.split(|byte| *byte == b'\n').map(|line| Value::Bytes(line.to_vec())).collect()
        }
        _ => return Err(OpError::invalid_input("Running line by line needs text or bytes")),
    };
    let mut results = Vec::with_capacity(lines.len());
    for (number, line) in lines.iter().enumerate() {
        let output = (entry.run)(config, &[line])
            .map_err(|e| OpError::new(e.code, format!("Line {}: {}", number + 1, e.message)))?;
        match output.into_iter().next() {
            Some(Value::Text(result)) => results.push(result),
            _ => return Err(OpError::invalid_input("Expected a text result")),
        }
    }
    Ok(vec![Value::Text(results.join("\n"))])
}

fn truncate_chars(text: &str, max_chars: usize) -> (String, bool) {
    match text.char_indices().nth(max_chars) {
        Some((end, _)) => (text[..end].to_owned(), true),
        None => (text.to_owned(), false),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

/// The longest prefix of `bytes` that is enough to preview `max_chars` characters, as
/// text if it is valid UTF-8. Only the prefix is examined: a preview is made for every
/// output of every run, so it must not scan values that can be hundreds of megabytes.
fn text_prefix(bytes: &[u8], max_chars: usize) -> Option<&str> {
    // A character is at most four bytes; one more is enough to tell if there is more.
    let window = &bytes[..bytes.len().min((max_chars + 1) * 4)];
    match std::str::from_utf8(window) {
        Ok(text) => Some(text),
        // The window ended in the middle of a character: everything before it is fine.
        Err(error) if error.error_len().is_none() && window.len() < bytes.len() => {
            std::str::from_utf8(&window[..error.valid_up_to()]).ok()
        }
        Err(_) => None,
    }
}

fn preview(port: &str, value: &Value, max_chars: usize) -> OutputPreview {
    let text_preview = |bytes: &[u8]| {
        text_prefix(bytes, max_chars).map(|prefix| {
            let (shown, cut) = truncate_chars(prefix, max_chars);
            (shown, cut || prefix.len() < bytes.len())
        })
    };
    let (size, (preview, truncated)) = match value {
        Value::Text(text) => (
            text.len(),
            text_preview(text.as_bytes()).expect("a prefix of text cut at a boundary is text"),
        ),
        Value::Bytes(bytes) => (
            bytes.len(),
            text_preview(bytes).unwrap_or_else(|| {
                let shown = bytes.len().min(max_chars / 3);
                (hex(&bytes[..shown]), shown < bytes.len())
            }),
        ),
        Value::Number(number) => (1, (number.to_string(), false)),
        Value::TextList(items) => (items.len(), (format!("{} items", items.len()), false)),
        Value::NumberList(items) => (items.len(), (format!("{} numbers", items.len()), false)),
    };
    OutputPreview { port: port.into(), ty: value.value_type(), size, preview, truncated }
}

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct OutputText {
    pub text: String,
    /// Characters in the complete output, of which `text` may be only the beginning.
    pub total_chars: usize,
    pub truncated: bool,
}

fn render(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        // Bytes that happen to be valid UTF-8 are shown as text. This is display only:
        // the value keeps its type and still needs an explicit decode to feed a text port.
        Value::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => text.to_owned(),
            Err(_) => format!("Binary data ({} bytes):\n{}", bytes.len(), hex(bytes)),
        },
        Value::Number(number) => number.to_string(),
        Value::TextList(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{i}: {item:?}"))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::NumberList(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{i}: {item}"))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// The value as bytes, for saving to a file: text as UTF-8, bytes as they are, and
/// anything else in its displayed form.
pub fn to_bytes(value: &Value) -> Vec<u8> {
    match value {
        Value::Bytes(bytes) => bytes.clone(),
        other => render(other).into_bytes(),
    }
}

fn cut(text: &str, max_chars: usize) -> OutputText {
    let (shown, truncated) = truncate_chars(text, max_chars);
    OutputText { text: shown, total_chars: text.chars().count(), truncated }
}

/// The human-readable form of a value for the output panel, cut to `max_chars`.
///
/// Large values are cut before they are formatted: a hex dump is three characters per
/// byte, so dumping a whole file only to throw most of it away could exhaust memory.
pub fn display(value: &Value, max_chars: usize) -> OutputText {
    match value {
        Value::Text(text) => cut(text, max_chars),
        Value::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => cut(text, max_chars),
            Err(_) => {
                let header = format!("Binary data ({} bytes):\n", bytes.len());
                let shown = bytes.len().min(max_chars / 3);
                OutputText {
                    total_chars: header.len() + (bytes.len() * 3).saturating_sub(1),
                    truncated: shown < bytes.len(),
                    text: header + &hex(&bytes[..shown]),
                }
            }
        },
        other => cut(&render(other), max_chars),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(id: &str, text: &str) -> GraphNode {
        GraphNode::Input { id: id.into(), text: text.into() }
    }

    fn op(id: &str, op_id: &str, config: serde_json::Value) -> GraphNode {
        GraphNode::Op { id: id.into(), op_id: op_id.into(), config, per_line: false }
    }

    fn edge(source: &str, target: &str) -> GraphEdge {
        edge_ports(source, "output", target, "input")
    }

    fn edge_ports(source: &str, source_port: &str, target: &str, target_port: &str) -> GraphEdge {
        GraphEdge {
            source: source.into(),
            source_port: source_port.into(),
            target: target.into(),
            target_port: target_port.into(),
        }
    }

    fn text_of(evaluation: &Evaluation, node: &str) -> String {
        display(evaluation.output(node, "output").expect("node has an output"), usize::MAX).text
    }

    /// in -> rot13 -> xor -> utf8 -> upper, plus a second reader of `rot13`.
    fn long_chain() -> Graph {
        Graph {
            nodes: vec![
                input("in", "a text that is long enough to count"),
                op("rot13", "rot13", json!({})),
                op("xor", "xor", json!({ "key": "20", "keyFormat": "hex" })),
                op("utf8", "utf8-decode", json!({})),
                op("upper", "to-uppercase", json!({})),
                op("side", "reverse", json!({})),
            ],
            edges: vec![
                edge("in", "rot13"),
                edge("rot13", "xor"),
                edge("xor", "utf8"),
                edge("utf8", "upper"),
                edge("rot13", "side"),
            ],
        }
    }

    fn retained(evaluation: &Evaluation) -> Vec<&str> {
        let mut ids: Vec<&str> = evaluation
            .order
            .iter()
            .filter(|id| matches!(evaluation.outcomes[*id], Outcome::Ok(_)))
            .map(String::as_str)
            .collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn without_a_budget_every_result_is_kept() {
        let evaluation = evaluate(&long_chain(), &Sources::new());
        assert_eq!(retained(&evaluation), ["in", "rot13", "side", "upper", "utf8", "xor"]);
    }

    #[test]
    fn past_the_budget_only_what_is_looked_at_is_kept_and_results_do_not_change() {
        let graph = long_chain();
        let everything = evaluate(&graph, &Sources::new());
        let keep = ["upper".to_owned(), "side".to_owned()];
        let tight = Retention { budget_bytes: 0, keep: &keep, preview_chars: 10 };
        let evaluation = evaluate_within(&graph, &Sources::new(), &tight);

        assert_eq!(retained(&evaluation), ["side", "upper"]);
        for node in ["upper", "side"] {
            assert_eq!(text_of(&evaluation, node), text_of(&everything, node));
        }
        // A released step still reports that it succeeded, with a preview.
        let reports = evaluation.report(10);
        let NodeReport::Ok { outputs, retained, .. } = &reports[1] else { panic!("rot13 ran") };
        assert!(!retained);
        assert_eq!(outputs[0].preview, "n grkg gun");
        assert!(evaluation.output("rot13", "output").is_none());
    }

    #[test]
    fn a_result_with_a_reader_still_to_come_is_not_given_away() {
        // `rot13` feeds both `xor` and `side`. Whichever runs first must leave it intact.
        let graph = long_chain();
        let keep = ["side".to_owned()];
        let tight = Retention { budget_bytes: 0, keep: &keep, preview_chars: 10 };
        let evaluation = evaluate_within(&graph, &Sources::new(), &tight);
        assert_eq!(
            text_of(&evaluation, "side"),
            text_of(&evaluate(&graph, &Sources::new()), "side")
        );
    }

    #[test]
    fn a_failing_step_after_a_hand_over_reports_its_own_error() {
        let graph = Graph {
            nodes: vec![
                input("in", "plain text, more than a few bytes of it"),
                op("xor", "xor", json!({ "key": "ff", "keyFormat": "hex" })),
                op("utf8", "utf8-decode", json!({})),
            ],
            edges: vec![edge("in", "xor"), edge("xor", "utf8")],
        };
        let tight = Retention { budget_bytes: 0, keep: &[], preview_chars: 10 };
        let evaluation = evaluate_within(&graph, &Sources::new(), &tight);
        assert!(matches!(evaluation.outcomes["xor"], Outcome::Released(_)));
        let Outcome::Error(error) = &evaluation.outcomes["utf8"] else { panic!("not valid text") };
        assert!(error.message.contains("Not valid UTF-8"), "{}", error.message);
    }

    #[test]
    fn a_file_is_never_released() {
        let mut sources = Sources::new();
        sources.insert("file".into(), Rc::new(Value::Bytes(vec![1, 2, 3, 4])));
        let graph = Graph {
            nodes: vec![GraphNode::File { id: "file".into() }, op("xor", "xor", json!({ "key": "k" }))],
            edges: vec![edge("file", "xor")],
        };
        let tight = Retention { budget_bytes: 0, keep: &[], preview_chars: 10 };
        let evaluation = evaluate_within(&graph, &sources, &tight);
        assert!(matches!(evaluation.outcomes["file"], Outcome::Ok(_)));
        // The file's bytes were read, not rewritten.
        assert_eq!(*sources["file"], Value::Bytes(vec![1, 2, 3, 4]));
    }

    #[test]
    fn a_chain_runs_in_dependency_order_regardless_of_declaration_order() {
        let graph = Graph {
            nodes: vec![
                op("hash", "sha256", json!({})),
                op("caesar", "caesar-cipher", json!({ "shift": 3 })),
                input("in", "abc"),
            ],
            edges: vec![edge("caesar", "hash"), edge("in", "caesar")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        assert_eq!(evaluation.order, ["in", "caesar", "hash"]);
        assert_eq!(text_of(&evaluation, "caesar"), "def");
        // `printf def | sha256sum`
        assert_eq!(
            text_of(&evaluation, "hash"),
            "cb8379ac2098aa165029e3938a51da0bcecfc008fd6795f401178647f96c5b34"
        );
    }

    #[test]
    fn one_output_can_feed_several_branches() {
        let graph = Graph {
            nodes: vec![
                input("in", "Nohut"),
                op("upper", "to-uppercase", json!({})),
                op("reverse", "reverse", json!({})),
                op("both", "concatenate-text", json!({ "separator": "+" })),
            ],
            edges: vec![
                edge("in", "upper"),
                edge("in", "reverse"),
                edge_ports("upper", "output", "both", "input1"),
                edge_ports("reverse", "output", "both", "input2"),
            ],
        };
        assert_eq!(text_of(&evaluate(&graph, &Sources::new()), "both"), "NOHUT+tuhoN");
    }

    #[test]
    fn bytes_into_a_text_port_is_a_type_error_with_a_hint() {
        let graph = Graph {
            nodes: vec![
                input("in", "aGVsbG8="),
                op("decode", "base64-standard-decode", json!({})),
                op("upper", "to-uppercase", json!({})),
            ],
            edges: vec![edge("in", "decode"), edge("decode", "upper")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        let Outcome::Error(error) = &evaluation.outcomes["upper"] else { panic!("expected an error") };
        assert_eq!(error.code, ErrorCode::TypeMismatch);
        assert!(error.message.contains("Decode Text (UTF-8)"));
    }

    #[test]
    fn an_explicit_decode_makes_the_chain_valid() {
        let graph = Graph {
            nodes: vec![
                input("in", "aGVsbG8="),
                op("decode", "base64-standard-decode", json!({})),
                op("text", "utf8-decode", json!({})),
                op("upper", "to-uppercase", json!({})),
            ],
            edges: vec![edge("in", "decode"), edge("decode", "text"), edge("text", "upper")],
        };
        assert_eq!(text_of(&evaluate(&graph, &Sources::new()), "upper"), "HELLO");
    }

    #[test]
    fn a_failure_blocks_downstream_but_not_other_branches() {
        let graph = Graph {
            nodes: vec![
                input("in", "!!!"),
                op("decode", "base64-standard-decode", json!({})),
                op("after", "utf8-decode", json!({})),
                op("other", "reverse", json!({})),
            ],
            edges: vec![edge("in", "decode"), edge("decode", "after"), edge("in", "other")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        assert!(matches!(evaluation.outcomes["decode"], Outcome::Error(_)));
        assert!(matches!(evaluation.outcomes["after"], Outcome::Blocked));
        assert_eq!(text_of(&evaluation, "other"), "!!!");
    }

    fn file_graph() -> Graph {
        Graph {
            nodes: vec![GraphNode::File { id: "file".into() }, op("hash", "sha256", json!({}))],
            edges: vec![edge("file", "hash")],
        }
    }

    #[test]
    fn a_file_node_feeds_its_bytes_into_the_graph() {
        let mut sources = Sources::new();
        sources.insert("file".into(), Rc::new(Value::Bytes(b"abc".to_vec())));
        let evaluation = evaluate(&file_graph(), &sources);
        assert_eq!(
            text_of(&evaluation, "hash"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(to_bytes(evaluation.output("file", "output").unwrap()), b"abc");
    }

    #[test]
    fn a_file_node_without_bytes_is_an_error_and_blocks_downstream() {
        let evaluation = evaluate(&file_graph(), &Sources::new());
        assert!(matches!(evaluation.outcomes["file"], Outcome::Error(_)));
        assert!(matches!(evaluation.outcomes["hash"], Outcome::Blocked));
    }

    #[test]
    fn an_unconnected_input_port_waits() {
        let graph = Graph { nodes: vec![op("lonely", "reverse", json!({}))], edges: vec![] };
        assert!(matches!(evaluate(&graph, &Sources::new()).outcomes["lonely"], Outcome::Waiting));
    }

    #[test]
    fn cycles_are_reported_instead_of_hanging() {
        let graph = Graph {
            nodes: vec![op("a", "reverse", json!({})), op("b", "reverse", json!({}))],
            edges: vec![edge("a", "b"), edge("b", "a")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        assert!(matches!(evaluation.outcomes["a"], Outcome::Error(_)));
        assert!(matches!(evaluation.outcomes["b"], Outcome::Error(_)));
    }

    #[test]
    fn per_line_applies_the_operation_to_each_line() {
        let mut node = op("hash", "md5", json!({}));
        if let GraphNode::Op { per_line, .. } = &mut node {
            *per_line = true;
        }
        let graph = Graph { nodes: vec![input("in", "a\nb"), node], edges: vec![edge("in", "hash")] };
        assert_eq!(
            text_of(&evaluate(&graph, &Sources::new()), "hash"),
            "0cc175b9c0f1b6a831c399e269772661\n92eb5ffee6ae2fec3ad71c777531578f"
        );
    }

    #[test]
    fn per_line_splits_raw_bytes_on_the_newline_byte() {
        let mut hash = op("hash", "md5", json!({}));
        if let GraphNode::Op { per_line, .. } = &mut hash {
            *per_line = true;
        }
        let graph = Graph {
            // "a\nb" in Base64
            nodes: vec![input("in", "YQpi"), op("decode", "base64-standard-decode", json!({})), hash],
            edges: vec![edge("in", "decode"), edge("decode", "hash")],
        };
        assert_eq!(
            text_of(&evaluate(&graph, &Sources::new()), "hash"),
            "0cc175b9c0f1b6a831c399e269772661\n92eb5ffee6ae2fec3ad71c777531578f"
        );
    }

    #[test]
    fn a_failing_line_is_named_in_the_error() {
        let mut decode = op("decode", "utf8-decode", json!({}));
        if let GraphNode::Op { per_line, .. } = &mut decode {
            *per_line = true;
        }
        let graph = Graph {
            // "ok\n\xff" in Base64
            nodes: vec![input("in", "b2sK/w=="), op("raw", "base64-standard-decode", json!({})), decode],
            edges: vec![edge("in", "raw"), edge("raw", "decode")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        let Outcome::Error(error) = &evaluation.outcomes["decode"] else { panic!("expected an error") };
        assert!(error.message.starts_with("Line 2:"), "{}", error.message);
    }

    #[test]
    fn multiple_outputs_are_addressed_by_port_name() {
        let graph = Graph {
            nodes: vec![input("in", "one two"), op("stats", "text-stats", json!({}))],
            edges: vec![edge("in", "stats")],
        };
        let evaluation = evaluate(&graph, &Sources::new());
        assert_eq!(evaluation.output("stats", "wordCount"), Some(&Value::Number(2.0)));
        assert_eq!(evaluation.output("stats", "charCount"), Some(&Value::Number(7.0)));
    }

    #[test]
    fn long_outputs_are_cut_for_display_and_say_so() {
        let shown = display(&Value::Text("nohut".repeat(10)), 12);
        assert_eq!(shown.text, "nohutnohutno");
        assert_eq!((shown.total_chars, shown.truncated), (50, true));
    }

    #[test]
    fn binary_output_is_dumped_only_as_far_as_it_is_shown() {
        let shown = display(&Value::Bytes(vec![0xff; 1000]), 30);
        assert_eq!(shown.text, format!("Binary data (1000 bytes):\n{}", ["ff"; 10].join(" ")));
        assert!(shown.truncated);
        let whole = display(&Value::Bytes(vec![0xff; 4]), 30);
        assert_eq!(whole.text, "Binary data (4 bytes):\nff ff ff ff");
        assert_eq!((whole.total_chars, whole.truncated), (whole.text.len(), false));
    }

    #[test]
    fn previews_are_truncated_on_character_boundaries() {
        let graph = Graph { nodes: vec![input("in", "çççççç")], edges: vec![] };
        let report = evaluate(&graph, &Sources::new()).report(3);
        let NodeReport::Ok { outputs, .. } = &report[0] else { panic!("expected ok") };
        assert_eq!(outputs[0].preview, "ççç");
        assert!(outputs[0].truncated);
        assert_eq!(outputs[0].size, 12);
    }

    #[test]
    fn previews_examine_only_the_start_of_a_value() {
        // Valid text followed, far past the preview, by bytes that are not UTF-8.
        let mut bytes = "nohut ".repeat(100).into_bytes();
        bytes.extend([0xff, 0xfe]);
        let shown = preview("output", &Value::Bytes(bytes), 8);
        assert_eq!((shown.preview.as_str(), shown.truncated), ("nohut no", true));

        let exact = preview("output", &Value::Text("çay".into()), 3);
        assert_eq!((exact.preview.as_str(), exact.truncated), ("çay", false));

        let binary = preview("output", &Value::Bytes(vec![0xff, 0x00, 0x10]), 60);
        assert_eq!((binary.preview.as_str(), binary.truncated), ("ff 00 10", false));
    }
}
