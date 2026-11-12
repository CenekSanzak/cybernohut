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
            GraphNode::Input { id, .. } | GraphNode::Op { id, .. } => id,
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

/// Name of the single output port of an input node.
pub const INPUT_NODE_PORT: &str = "output";

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NodeReport {
    Ok { id: String, outputs: Vec<OutputPreview> },
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
    /// Characters for text, bytes for bytes, items for lists, 1 for a number.
    pub size: usize,
    pub preview: String,
    pub truncated: bool,
}

pub enum Outcome {
    Ok(Vec<(String, Rc<Value>)>),
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
                        outputs: outputs
                            .iter()
                            .map(|(port, value)| preview(port, value, preview_chars))
                            .collect(),
                    },
                    Outcome::Error(error) => NodeReport::Error { id, error: error.clone() },
                    Outcome::Waiting => NodeReport::Waiting { id },
                    Outcome::Blocked => NodeReport::Blocked { id },
                }
            })
            .collect()
    }
}

pub fn evaluate(graph: &Graph) -> Evaluation {
    let index: HashMap<&str, &GraphNode> = graph.nodes.iter().map(|n| (n.id(), n)).collect();
    let mut incoming: HashMap<&str, Vec<&GraphEdge>> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = graph.nodes.iter().map(|n| (n.id(), 0)).collect();

    for edge in &graph.edges {
        // Edges that point at missing nodes are leftovers of a deletion; ignore them.
        if !index.contains_key(edge.source.as_str()) || !index.contains_key(edge.target.as_str()) {
            continue;
        }
        incoming.entry(&edge.target).or_default().push(edge);
        outgoing.entry(&edge.source).or_default().push(&edge.target);
        *in_degree.get_mut(edge.target.as_str()).expect("target was indexed") += 1;
    }

    // Kahn's algorithm, seeded in declaration order so the result is deterministic.
    let mut queue: VecDeque<&str> =
        graph.nodes.iter().map(GraphNode::id).filter(|id| in_degree[id] == 0).collect();
    let mut evaluation = Evaluation::default();

    while let Some(id) = queue.pop_front() {
        let outcome = evaluate_node(index[id], incoming.get(id), &evaluation);
        evaluation.order.push(id.to_owned());
        evaluation.outcomes.insert(id.to_owned(), outcome);
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
    done: &Evaluation,
) -> Outcome {
    let (op_id, config, per_line) = match node {
        GraphNode::Input { text, .. } => {
            return Outcome::Ok(vec![(INPUT_NODE_PORT.into(), Rc::new(Value::Text(text.clone())))]);
        }
        GraphNode::Op { op_id, config, per_line, .. } => (op_id, config, *per_line),
    };
    let Some(entry) = ops::find(op_id) else {
        return Outcome::Error(OpError::unknown_op(op_id));
    };
    let meta = (entry.meta)();

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

fn preview(port: &str, value: &Value, max_chars: usize) -> OutputPreview {
    let (size, (preview, truncated)) = match value {
        Value::Text(text) => (text.chars().count(), truncate_chars(text, max_chars)),
        Value::Bytes(bytes) => (
            bytes.len(),
            match std::str::from_utf8(bytes) {
                Ok(text) => truncate_chars(text, max_chars),
                Err(_) => {
                    let shown = bytes.len().min(max_chars / 3);
                    (hex(&bytes[..shown]), shown < bytes.len())
                }
            },
        ),
        Value::Number(number) => (1, (number.to_string(), false)),
        Value::TextList(items) => (items.len(), (format!("{} items", items.len()), false)),
        Value::NumberList(items) => (items.len(), (format!("{} numbers", items.len()), false)),
    };
    OutputPreview { port: port.into(), ty: value.value_type(), size, preview, truncated }
}

/// The full, human-readable form of a value, for the output panel.
pub fn display(value: &Value) -> String {
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
        display(evaluation.output(node, "output").expect("node has an output"))
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
        let evaluation = evaluate(&graph);
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
        assert_eq!(text_of(&evaluate(&graph), "both"), "NOHUT+tuhoN");
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
        let evaluation = evaluate(&graph);
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
        assert_eq!(text_of(&evaluate(&graph), "upper"), "HELLO");
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
        let evaluation = evaluate(&graph);
        assert!(matches!(evaluation.outcomes["decode"], Outcome::Error(_)));
        assert!(matches!(evaluation.outcomes["after"], Outcome::Blocked));
        assert_eq!(text_of(&evaluation, "other"), "!!!");
    }

    #[test]
    fn an_unconnected_input_port_waits() {
        let graph = Graph { nodes: vec![op("lonely", "reverse", json!({}))], edges: vec![] };
        assert!(matches!(evaluate(&graph).outcomes["lonely"], Outcome::Waiting));
    }

    #[test]
    fn cycles_are_reported_instead_of_hanging() {
        let graph = Graph {
            nodes: vec![op("a", "reverse", json!({})), op("b", "reverse", json!({}))],
            edges: vec![edge("a", "b"), edge("b", "a")],
        };
        let evaluation = evaluate(&graph);
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
            text_of(&evaluate(&graph), "hash"),
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
            text_of(&evaluate(&graph), "hash"),
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
        let evaluation = evaluate(&graph);
        let Outcome::Error(error) = &evaluation.outcomes["decode"] else { panic!("expected an error") };
        assert!(error.message.starts_with("Line 2:"), "{}", error.message);
    }

    #[test]
    fn multiple_outputs_are_addressed_by_port_name() {
        let graph = Graph {
            nodes: vec![input("in", "one two"), op("stats", "text-stats", json!({}))],
            edges: vec![edge("in", "stats")],
        };
        let evaluation = evaluate(&graph);
        assert_eq!(evaluation.output("stats", "wordCount"), Some(&Value::Number(2.0)));
        assert_eq!(evaluation.output("stats", "charCount"), Some(&Value::Number(7.0)));
    }

    #[test]
    fn previews_are_truncated_on_character_boundaries() {
        let graph = Graph { nodes: vec![input("in", "çççççç")], edges: vec![] };
        let report = evaluate(&graph).report(3);
        let NodeReport::Ok { outputs, .. } = &report[0] else { panic!("expected ok") };
        assert_eq!(outputs[0].preview, "ççç");
        assert!(outputs[0].truncated);
        assert_eq!(outputs[0].size, 6);
    }
}
