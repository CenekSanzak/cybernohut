//! cybernohut on the command line: one operation as a filter, a graph exported from
//! the web app, or auto mode. Everything runs the same Rust code the web app runs as
//! WebAssembly.

use std::collections::{HashMap, HashSet};
use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;
use std::rc::Rc;

use serde_json::{json, Value as Json};
use wasm::core::{OpMeta, ParamKind, Value, ValueType};
use wasm::graph::{self, Graph, GraphEdge, GraphNode, Outcome, Sources, INPUT_NODE_PORT};
use wasm::{auto, ops};

const USAGE: &str = "\
cybernohut: encode, decode, hash and analyse data

Usage:
  cybernohut <operation> [--setting value]... [-i FILE]
      Run one operation on FILE or standard input.
  cybernohut run GRAPH.json [-i FILE] [--node ID]
      Run a graph exported from the web app. With -i (use - for standard
      input) the data replaces what the graph's input node holds.
      Prints the last step, or the step named by --node.
  cybernohut auto [-i FILE] [--apply | --graph]
      Suggest decoding steps. --apply prints the result of the best
      suggestion; --graph prints it as a graph the web app can import.
  cybernohut list [WORD]
      List operations, optionally only those matching WORD.
  cybernohut help <operation>
      Show an operation's settings.

Text input loses one trailing newline, so `echo` works as expected.";

type Failure = String;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cybernohut: {message}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(args: &[String]) -> Result<(), Failure> {
    match args.split_first() {
        None => Err(format!("no command given\n\n{USAGE}")),
        Some((command, rest)) => match command.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                Ok(())
            }
            "help" => match rest.first() {
                Some(id) => describe(id),
                None => {
                    println!("{USAGE}");
                    Ok(())
                }
            },
            "list" => list(rest.first().map(String::as_str)),
            "run" => run_graph(rest),
            "auto" => run_auto(rest),
            id => run_operation(id, rest),
        },
    }
}

/// Options shared by every command, split from the `--setting value` pairs.
#[derive(Default)]
struct Options {
    input: Option<String>,
    flags: HashSet<String>,
    settings: Vec<(String, Option<String>)>,
    positional: Vec<String>,
}

fn parse_options(args: &[String], flags: &[&str], valued: &[&str]) -> Result<Options, Failure> {
    let mut options = Options::default();
    let mut rest = args.iter().peekable();
    while let Some(arg) = rest.next() {
        if arg == "-i" || arg == "--input" {
            options.input = Some(rest.next().ok_or("-i needs a file name")?.clone());
        } else if let Some(name) = arg.strip_prefix("--") {
            let (name, inline) = match name.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (name, None),
            };
            if flags.contains(&name) {
                options.flags.insert(name.to_owned());
                continue;
            }
            // A setting takes the next argument as its value, unless that is another
            // option; a bare `--decrypt` then means true.
            let value = inline.or_else(|| {
                let takes_next = valued.contains(&name)
                    || rest.peek().is_some_and(|next| !next.starts_with("--") && *next != "-i");
                takes_next.then(|| rest.next().cloned()).flatten()
            });
            options.settings.push((name.to_owned(), value));
        } else {
            options.positional.push(arg.clone());
        }
    }
    Ok(options)
}

fn read_input(path: Option<&str>) -> Result<Option<Vec<u8>>, Failure> {
    match path {
        Some(path) if path != "-" => {
            std::fs::read(path).map(Some).map_err(|error| format!("cannot read {path}: {error}"))
        }
        None if io::stdin().is_terminal() => Ok(None),
        // `-`, or nothing named and data piped in.
        _ => {
            let mut bytes = Vec::new();
            io::stdin()
                .read_to_end(&mut bytes)
                .map_err(|error| format!("cannot read standard input: {error}"))?;
            Ok(Some(bytes))
        }
    }
}

/// Input as text, without the newline that `echo` and most editors put at the end.
fn into_text(bytes: Vec<u8>) -> Result<String, Failure> {
    let mut text = String::from_utf8(bytes)
        .map_err(|_| "the input is not UTF-8 text, which this step needs".to_owned())?;
    if text.ends_with('\n') {
        text.pop();
        if text.ends_with('\r') {
            text.pop();
        }
    }
    Ok(text)
}

fn write_value(value: &Value) -> Result<(), Failure> {
    let mut stdout = io::stdout().lock();
    let bytes = graph::to_bytes(value);
    let is_text = !matches!(value, Value::Bytes(_));
    let result = stdout.write_all(&bytes).and_then(|()| {
        // A terminal gets a final newline; a pipe gets exactly the data.
        if is_text && stdout.is_terminal() && !bytes.ends_with(b"\n") {
            stdout.write_all(b"\n")
        } else {
            Ok(())
        }
    });
    match result {
        // The reader went away, as in `cybernohut ... | head`.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => other.map_err(|error| format!("cannot write output: {error}")),
    }
}

fn meta_of(id: &str) -> Result<OpMeta, Failure> {
    let entry = ops::find(id)
        .ok_or_else(|| format!("unknown operation \"{id}\"; see `cybernohut list`"))?;
    Ok((entry.meta)())
}

fn kebab(key: &str) -> String {
    let mut out = String::new();
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn list(filter: Option<&str>) -> Result<(), Failure> {
    let filter = filter.map(str::to_lowercase);
    for entry in ops::registry() {
        let meta = (entry.meta)();
        let haystack = format!("{} {} {}", meta.id, meta.name, meta.description).to_lowercase();
        if filter.as_ref().is_some_and(|word| !haystack.contains(word)) {
            continue;
        }
        println!("{:<24} {}", meta.id, meta.description);
    }
    Ok(())
}

fn describe(id: &str) -> Result<(), Failure> {
    let meta = meta_of(id)?;
    let ports = |ports: &[wasm::core::Port]| {
        ports.iter().map(|port| port.ty.label()).collect::<Vec<_>>().join(", ")
    };
    println!("{} ({})\n{}", meta.name, meta.id, meta.description);
    println!("\nTakes {}; gives {}.", ports(&meta.inputs), ports(&meta.outputs));
    if !meta.params.is_empty() {
        println!("\nSettings:");
    }
    for param in &meta.params {
        let kind = match &param.kind {
            ParamKind::Int { .. } => "a whole number".to_owned(),
            ParamKind::Bool => "on when given".to_owned(),
            ParamKind::Text => "text".to_owned(),
            ParamKind::Choice { options } => options.join(" | "),
        };
        let default = &meta.default_config[&param.key];
        println!("  --{:<14} {} ({kind}; default {default})", kebab(&param.key), param.label);
    }
    if let Some(link) = &meta.link {
        println!("\n{link}");
    }
    Ok(())
}

/// Turns `--setting value` pairs into the operation's config, typed by its params.
fn config_from(meta: &OpMeta, settings: &[(String, Option<String>)]) -> Result<Json, Failure> {
    let mut config = meta.default_config.clone();
    for (name, value) in settings {
        let param = meta
            .params
            .iter()
            .find(|param| param.key == *name || kebab(&param.key) == *name)
            .ok_or_else(|| format!("{} has no setting --{name}; see `cybernohut help {}`", meta.id, meta.id))?;
        let missing = || format!("--{name} needs a value");
        let parsed = match &param.kind {
            ParamKind::Bool => match value.as_deref() {
                None | Some("true") => json!(true),
                Some("false") => json!(false),
                Some(other) => return Err(format!("--{name} is true or false, not \"{other}\"")),
            },
            ParamKind::Int { .. } => {
                let value = value.as_ref().ok_or_else(missing)?;
                json!(value.parse::<i64>().map_err(|_| format!("--{name} needs a whole number, not \"{value}\""))?)
            }
            ParamKind::Choice { options } => {
                let value = value.as_ref().ok_or_else(missing)?;
                if !options.contains(value) {
                    return Err(format!("--{name} is one of: {}", options.join(", ")));
                }
                json!(value)
            }
            ParamKind::Text => json!(value.as_ref().ok_or_else(missing)?),
        };
        config[&param.key] = parsed;
    }
    Ok(config)
}

fn run_operation(id: &str, args: &[String]) -> Result<(), Failure> {
    let meta = meta_of(id)?;
    let text_settings: Vec<&str> = meta.params.iter().map(|param| param.key.as_str()).collect();
    let options = parse_options(args, &[], &text_settings)?;
    if let Some(extra) = options.positional.first() {
        return Err(format!("unexpected argument \"{extra}\"; settings are given as --name value"));
    }
    let [port] = meta.inputs.as_slice() else {
        return Err(format!("{id} takes {} inputs; build it as a graph in the web app", meta.inputs.len()));
    };
    let config = config_from(&meta, &options.settings)?;
    let bytes = read_input(options.input.as_deref())?
        .ok_or("no input: pipe data in or name a file with -i")?;
    let input = match port.ty {
        ValueType::Bytes => Value::Bytes(bytes),
        ValueType::Text => Value::Text(into_text(bytes)?),
        other => return Err(format!("{id} takes a {}, which cannot come from a file", other.label())),
    };
    let entry = ops::find(id).expect("looked up above");
    let outputs = (entry.run)(&config, &[&input]).map_err(|error| error.message)?;
    match outputs.as_slice() {
        [only] => write_value(only),
        many => {
            for (port, value) in meta.outputs.iter().zip(many) {
                println!("{}: {}", port.name, graph::display(value, usize::MAX).text);
            }
            Ok(())
        }
    }
}

/// Reads the web app's export format into the engine's graph. Positions, selection
/// and notes are the app's business and are ignored.
fn read_graph(path: &str) -> Result<Graph, Failure> {
    let text = std::fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let file: Json = serde_json::from_str(&text).map_err(|error| format!("{path} is not JSON: {error}"))?;
    let field = |value: &Json, key: &str| -> Result<String, Failure> {
        value[key].as_str().map(str::to_owned).ok_or_else(|| format!("{path}: a \"{key}\" is missing"))
    };
    let mut nodes = Vec::new();
    for node in file["nodes"].as_array().ok_or_else(|| format!("{path} has no nodes; is it a cybernohut export?"))? {
        let id = field(node, "id")?;
        let data = &node["data"];
        nodes.push(match data["kind"].as_str() {
            Some("input") => GraphNode::Input { id, text: data["text"].as_str().unwrap_or("").to_owned() },
            Some("file") => GraphNode::File { id },
            Some("op") => GraphNode::Op {
                id,
                op_id: field(data, "opId")?,
                config: data["config"].clone(),
                per_line: data["perLine"].as_bool().unwrap_or(false),
            },
            _ => return Err(format!("{path}: node {id} is of an unknown kind")),
        });
    }
    let mut edges = Vec::new();
    for edge in file["edges"].as_array().map(Vec::as_slice).unwrap_or_default() {
        edges.push(GraphEdge {
            source: field(edge, "source")?,
            source_port: field(edge, "sourceHandle")?,
            target: field(edge, "target")?,
            target_port: field(edge, "targetHandle")?,
        });
    }
    Ok(Graph { nodes, edges })
}

fn run_graph(args: &[String]) -> Result<(), Failure> {
    let options = parse_options(args, &[], &["node"])?;
    let [path] = options.positional.as_slice() else {
        return Err("give one graph file: cybernohut run GRAPH.json".into());
    };
    let wanted = match options.settings.as_slice() {
        [] => None,
        [(name, Some(id))] if name == "node" => Some(id.clone()),
        [(name, _), ..] => return Err(format!("unknown option --{name}")),
    };
    let mut graph = read_graph(path)?;
    let mut sources = Sources::new();

    // Only on request: a saved graph carries its own input, and silently swapping it
    // for whatever standard input happens to be would be a surprise.
    let replacement = match options.input.as_deref() {
        Some(path) => read_input(Some(path))?,
        None => None,
    };
    if let Some(bytes) = replacement {
        let mut starts = graph.nodes.iter_mut().filter(|node| !matches!(node, GraphNode::Op { .. }));
        match (starts.next(), starts.next()) {
            (Some(GraphNode::Input { text, .. }), None) => *text = into_text(bytes)?,
            (Some(GraphNode::File { id }), None) => {
                sources.insert(id.clone(), Rc::new(Value::Bytes(bytes)));
            }
            (None, _) => return Err("the graph has no input node to feed".into()),
            _ => return Err("the graph has several inputs, so it is unclear which one the data is for".into()),
        }
    }

    let evaluation = graph::evaluate(&graph, &sources);
    let label = |id: &str| match graph.nodes.iter().find(|node| node.id() == id) {
        Some(GraphNode::Op { op_id, .. }) => format!("{id} ({op_id})"),
        _ => id.to_owned(),
    };

    let mut failed = false;
    for id in &evaluation.order {
        if let Outcome::Error(error) = &evaluation.outcomes[id] {
            eprintln!("cybernohut: {}: {}", label(id), error.message);
            failed = true;
        }
    }

    let has_output: HashSet<&str> = graph.edges.iter().map(|edge| edge.source.as_str()).collect();
    let chosen: Vec<&String> = match &wanted {
        Some(id) => vec![evaluation.order.iter().find(|node| *node == id).ok_or_else(|| format!("the graph has no node {id}"))?],
        None => evaluation
            .order
            .iter()
            .filter(|id| !has_output.contains(id.as_str()) && matches!(evaluation.outcomes[*id], Outcome::Ok(_)))
            .collect(),
    };
    let first_output = |id: &str| match &evaluation.outcomes[id] {
        Outcome::Ok(outputs) => outputs.first().map(|(_, value)| value.as_ref()),
        _ => None,
    };
    match chosen.as_slice() {
        [] if failed => {}
        [] => return Err("no step produced a result".into()),
        [only] => match first_output(only) {
            Some(value) => write_value(value)?,
            None if failed => {}
            None => return Err(format!("{} did not run: an earlier step is missing or failed", label(only))),
        },
        several => {
            for id in several {
                if let Some(value) = first_output(id) {
                    println!("== {} ==\n{}", label(id), graph::display(value, usize::MAX).text);
                }
            }
        }
    }
    if failed {
        return Err("some steps failed".into());
    }
    Ok(())
}

/// A proposal as a graph the web app imports: the input, then one node per step.
fn proposal_graph(input: &Value, proposal: &auto::Proposal) -> Json {
    let source = match input {
        Value::Text(text) => json!({ "kind": "input", "text": text }),
        _ => json!({ "kind": "file" }),
    };
    let mut nodes = vec![json!({ "id": "input-1", "position": { "x": 250, "y": 10 }, "data": source })];
    let mut edges = Vec::new();
    for (index, step) in proposal.steps.iter().enumerate() {
        let id = format!("step-{}", index + 1);
        let previous = nodes.last().expect("the input is there")["id"].clone();
        edges.push(json!({
            "source": previous, "sourceHandle": INPUT_NODE_PORT,
            "target": id, "targetHandle": "input",
        }));
        nodes.push(json!({
            "id": id,
            "position": { "x": 250, "y": 170 + index * 210 },
            "data": {
                "kind": "op", "opId": step.op_id, "config": step.config,
                "perLine": false, "note": step.reason,
            },
        }));
    }
    let last = nodes.last().expect("the input is there")["id"].clone();
    json!({ "version": 2, "nodes": nodes, "edges": edges, "selectedInputId": "input-1", "selectedNodeId": last })
}

fn run_auto(args: &[String]) -> Result<(), Failure> {
    let options = parse_options(args, &["apply", "graph"], &[])?;
    if let Some((name, _)) = options.settings.first() {
        return Err(format!("unknown option --{name}"));
    }
    let bytes = read_input(options.input.as_deref())?
        .ok_or("no input: pipe data in or name a file with -i")?;
    // Text when it can be, as it would be when pasted into the web app.
    let input = match String::from_utf8(bytes) {
        Ok(text) => Value::Text(into_text(text.into_bytes())?),
        Err(error) => Value::Bytes(error.into_bytes()),
    };
    let proposals = auto::suggest(&input);
    let Some(best) = proposals.first() else {
        return Err("nothing to suggest: no steps make this more readable".into());
    };

    if options.flags.contains("graph") {
        println!("{}", proposal_graph(&input, best));
        return Ok(());
    }
    if options.flags.contains("apply") {
        let mut value = input;
        for step in &best.steps {
            let entry = ops::find(&step.op_id).ok_or("a suggested step no longer exists")?;
            value = (entry.run)(&step.config, &[&value])
                .map_err(|error| format!("{} failed on the full input: {}", step.op_id, error.message))?
                .remove(0);
        }
        return write_value(&value);
    }

    let defaults: HashMap<&str, Json> =
        ops::registry().iter().map(|entry| (entry.id, (entry.meta)().default_config)).collect();
    for (number, proposal) in proposals.iter().enumerate() {
        println!("{}. readability {:.2}", number + 1, proposal.score);
        for step in &proposal.steps {
            // Only the settings that differ from the defaults are worth showing.
            let changed: Vec<String> = step
                .config
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(key, value)| defaults[step.op_id.as_str()].get(key.as_str()) != Some(value))
                .map(|(key, value)| match value {
                    Json::Bool(true) => format!(" --{}", kebab(key)),
                    Json::String(text) => format!(" --{} '{text}'", kebab(key)),
                    other => format!(" --{} {other}", kebab(key)),
                })
                .collect();
            println!("   {}{}\n      {}", step.op_id, changed.concat(), step.reason);
        }
        println!("   gives: {}\n", proposal.preview.replace('\n', " "));
    }
    Ok(())
}
