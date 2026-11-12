//! Runs the built binary the way a shell would.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cybernohut"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    child.stdin.take().expect("stdin is piped").write_all(stdin).expect("stdin accepts input");
    child.wait_with_output().expect("the binary exits")
}

fn stdout(args: &[&str], stdin: &[u8]) -> String {
    let output = run(args, stdin);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("text output")
}

const PROSE: &str = "The secret message is hidden in the last file that you will find here. \
    Nobody has opened it for years.";

/// `PROSE` XORed with "nohut", then Base64.
fn secret() -> String {
    let xored = stdout(&["xor", "--key", "nohut"], PROSE.as_bytes());
    // The XOR output is raw bytes, so it goes back in as bytes.
    let output = run(&["base64-standard-encode"], xored.as_bytes());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn an_operation_filters_standard_input() {
    assert_eq!(stdout(&["sha256"], b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    // Bytes are taken as they are; text loses the newline echo adds.
    assert_eq!(stdout(&["base64-standard-encode"], b"hi\n"), "aGkK");
    assert_eq!(stdout(&["to-uppercase"], b"hi\n"), "HI");
}

#[test]
fn settings_are_typed_by_the_operation() {
    assert_eq!(stdout(&["caesar-cipher", "--shift", "3"], b"abc"), "def");
    assert_eq!(stdout(&["vigenere", "--key", "lemon", "--decrypt"], b"LXFOPVEFRNHR"), "ATTACKATDAWN");
    assert_eq!(stdout(&["hmac", "--key=6b", "--key-format", "hex"], b"data"), stdout(&["hmac", "--key", "k"], b"data"));
    assert_eq!(stdout(&["json-format", "--indent", "0"], b"[1]"), "[\n1\n]");
}

#[test]
fn mistakes_are_reported_and_fail() {
    for (args, expected) in [
        (vec!["nope"], "unknown operation"),
        (vec!["xor"], "Enter a key"),
        (vec!["caesar-cipher", "--shift", "abc"], "whole number"),
        (vec!["caesar-cipher", "--speed", "3"], "no setting --speed"),
        (vec!["xor", "--key-format", "base64", "--key", "k"], "one of: text, hex"),
        (vec!["to-uppercase"], "not UTF-8"),
        (vec!["run", "/nonexistent.json"], "cannot read"),
    ] {
        let output = run(&args, &[0xff, 0xfe]);
        assert!(!output.status.success(), "{args:?} should fail");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn auto_explains_applies_and_exports_a_graph_that_runs() {
    let secret = secret();
    let explained = stdout(&["auto"], secret.as_bytes());
    assert!(explained.contains("xor --key 'nohut'"), "{explained}");
    assert!(explained.contains("gives: The secret message"), "{explained}");

    assert_eq!(stdout(&["auto", "--apply"], secret.as_bytes()), PROSE);

    let graph = stdout(&["auto", "--graph"], secret.as_bytes());
    let path = std::env::temp_dir().join(format!("cybernohut-test-{}.json", std::process::id()));
    std::fs::write(&path, &graph).unwrap();
    let path = path.to_str().unwrap();

    // The graph carries its input, and standard input is ignored unless asked for.
    assert_eq!(stdout(&["run", path], b"ignored"), PROSE);
    // The same steps on other data, encrypted with the same key.
    let other = run(&["xor", "--key", "nohut"], b"reusable steps").stdout;
    let other = run(&["base64-standard-encode"], &other).stdout;
    assert_eq!(stdout(&["run", path, "-i", "-"], &other), "reusable steps");
    // An intermediate step can be asked for by its id.
    assert_eq!(stdout(&["run", path, "--node", "step-3"], b""), PROSE);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn readable_input_has_nothing_to_suggest() {
    let output = run(&["auto"], PROSE.as_bytes());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("nothing to suggest"));
}
