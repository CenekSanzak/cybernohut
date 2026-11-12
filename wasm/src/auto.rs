//! Auto mode: a search for the chain of decoding steps that turns a value into
//! something readable.
//!
//! The result is not the decoded value but the steps themselves, each with the reason
//! it was chosen. The UI adds them to the graph as ordinary nodes, so what was found
//! can be read, adjusted and reused on other input.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use serde::Serialize;
use tsify::Tsify;

use crate::core::Value;
use crate::graph;
use crate::ops;
use crate::score::value_score;

/// Detection and trial runs use the start of the input; the real graph then runs the
/// chosen steps on all of it.
const SAMPLE_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 6;
/// A cap on trial runs, so pathological input cannot stall the search.
const MAX_VISITED: usize = 300;
const MAX_PROPOSALS: usize = 3;
/// A proposal has to make the value this much more readable than it already is.
const MIN_GAIN: f64 = 0.1;
/// Each extra step costs a little, so the shortest explanation wins a tie.
const STEP_COST: f64 = 0.01;
const PREVIEW_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct ProposedStep {
    pub op_id: String,
    #[tsify(type = "Record<string, string | number | boolean>")]
    pub config: serde_json::Value,
    /// Why this step was chosen, in words meant for the user.
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Tsify)]
pub struct Proposal {
    pub steps: Vec<ProposedStep>,
    /// Readability of the result: about 0 to 1, higher when it contains a flag.
    pub score: f64,
    /// The start of what the steps produce.
    pub preview: String,
}

fn fingerprint(value: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    match value {
        Value::Text(text) => (0u8, text.as_bytes()).hash(&mut hasher),
        Value::Bytes(bytes) => (1u8, bytes.as_slice()).hash(&mut hasher),
        other => (2u8, format!("{other:?}")).hash(&mut hasher),
    }
    hasher.finish()
}

/// Cuts a large value down to its start, at a point where encoded data stays decodable:
/// after a separator if there is one nearby, otherwise on a group-of-eight boundary.
fn sample(value: &Value) -> Value {
    match value {
        Value::Text(text) if text.len() > SAMPLE_BYTES => {
            let mut end = SAMPLE_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            let window = &text.as_bytes()[end - 64..end];
            let end = match window.iter().rposition(u8::is_ascii_whitespace) {
                Some(position) => end - 64 + position,
                None => end - end % 8,
            };
            Value::Text(text[..end].to_owned())
        }
        Value::Bytes(bytes) if bytes.len() > SAMPLE_BYTES => {
            Value::Bytes(bytes[..SAMPLE_BYTES].to_vec())
        }
        other => other.clone(),
    }
}

/// Proposes up to three chains of steps, best first. Empty when nothing makes the
/// value more readable than it already is.
pub fn suggest(value: &Value) -> Vec<Proposal> {
    let root = sample(value);
    let baseline = value_score(&root);
    // Whether the search is looking at the start of the value rather than all of it.
    let cut_short = match (value, &root) {
        (Value::Text(full), Value::Text(start)) => start.len() < full.len(),
        (Value::Bytes(full), Value::Bytes(start)) => start.len() < full.len(),
        _ => false,
    };

    let mut seen = HashSet::from([fingerprint(&root)]);
    let mut frontier: Vec<(Vec<ProposedStep>, Value)> = vec![(Vec::new(), root)];
    let mut found: Vec<Proposal> = Vec::new();

    for _ in 0..MAX_DEPTH {
        let mut next = Vec::new();
        for (steps, value) in &frontier {
            for entry in ops::registry() {
                if seen.len() >= MAX_VISITED {
                    break;
                }
                let Some(detection) = (entry.detect)(value) else { continue };
                let output = match (entry.run)(&detection.config, &[value]) {
                    Ok(outputs) => outputs.into_iter().next(),
                    // A compressed stream cut off by sampling fails to decompress, but
                    // what it holds up to the cut is enough to carry on with.
                    Err(_) if cut_short => match value {
                        Value::Bytes(bytes) => {
                            ops::compress::inflate_prefix(entry.id, bytes).map(Value::Bytes)
                        }
                        _ => None,
                    },
                    Err(_) => None,
                };
                let Some(output) = output else { continue };
                if !seen.insert(fingerprint(&output)) {
                    continue;
                }

                let mut steps = steps.clone();
                steps.push(ProposedStep {
                    op_id: entry.id.to_owned(),
                    config: detection.config,
                    reason: detection.reason,
                });
                let score = value_score(&output) - STEP_COST * steps.len() as f64;
                // Bytes that are valid text are one step short of a result: decoding
                // them as UTF-8 comes next, and that longer chain is the one to offer.
                let unfinished =
                    matches!(&output, Value::Bytes(bytes) if std::str::from_utf8(bytes).is_ok());
                if score >= baseline + MIN_GAIN && !unfinished {
                    found.push(Proposal {
                        steps: steps.clone(),
                        score: (score * 100.0).round() / 100.0,
                        preview: graph::display(&output, PREVIEW_CHARS).text,
                    });
                }
                next.push((steps, output));
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }

    found.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.steps.len().cmp(&b.steps.len())));
    // A chain that only extends a better, shorter one adds nothing.
    let mut kept: Vec<Proposal> = Vec::new();
    for proposal in found {
        let redundant = kept.iter().any(|better| {
            proposal.steps.len() > better.steps.len()
                && proposal.steps[..better.steps.len()] == better.steps[..]
        });
        if !redundant && kept.len() < MAX_PROPOSALS {
            kept.push(proposal);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PROSE: &str = "The secret message is hidden in the last file that you will find here.";

    /// Runs `PROSE` through encoding steps and returns what auto mode is given.
    fn encode(steps: &[(&str, serde_json::Value)]) -> Value {
        steps.iter().fold(Value::Text(PROSE.into()), |value, (op, config)| {
            let entry = ops::find(op).expect("operation exists");
            (entry.run)(config, &[&value]).expect("encoding succeeds").remove(0)
        })
    }

    fn ops_of(proposal: &Proposal) -> Vec<&str> {
        proposal.steps.iter().map(|step| step.op_id.as_str()).collect()
    }

    /// Replays a proposal on the full value, the way the graph would.
    fn replay(proposal: &Proposal, input: &Value) -> Value {
        proposal.steps.iter().fold(input.clone(), |value, step| {
            let entry = ops::find(&step.op_id).unwrap();
            (entry.run)(&step.config, &[&value]).unwrap().remove(0)
        })
    }

    #[test]
    fn plain_text_needs_no_steps() {
        assert!(suggest(&Value::Text(PROSE.into())).is_empty());
    }

    #[test]
    fn a_single_encoding_is_undone_with_an_explicit_text_step() {
        let input = encode(&[("base64-standard-encode", json!({}))]);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base64-standard-decode", "utf8-decode"]);
        assert_eq!(replay(best, &input), Value::Text(PROSE.into()));
        assert!(best.preview.starts_with("The secret message"));
        assert!(best.steps.iter().all(|step| !step.reason.is_empty()));
    }

    #[test]
    fn layered_encodings_are_peeled_in_order() {
        let input = encode(&[
            ("rot13", json!({})),
            ("base16-encode", json!({})),
            ("base64-standard-encode", json!({})),
        ]);
        let best = &suggest(&input)[0];
        assert_eq!(
            ops_of(best),
            ["base64-standard-decode", "utf8-decode", "base16-decode", "utf8-decode", "rot13"]
        );
        assert_eq!(replay(best, &input), Value::Text(PROSE.into()));
    }

    #[test]
    fn a_flag_in_the_text_does_not_stop_the_search_at_the_ciphertext() {
        // A flag keeps its shape under ROT13, so the ciphertext looks promising too.
        let plain = format!("{PROSE} ctf{{n0hut_auto}}");
        let input = ["rot13", "base16-encode", "base64-standard-encode"].iter().fold(
            Value::Text(plain.clone()),
            |value, op| (ops::find(op).unwrap().run)(&json!({}), &[&value]).unwrap().remove(0),
        );
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best).last(), Some(&"rot13"));
        assert_eq!(replay(best, &input), Value::Text(plain));
    }

    #[test]
    fn a_caesar_shift_is_proposed_with_its_shift() {
        let input = encode(&[("caesar-cipher", json!({ "shift": 5 }))]);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["caesar-cipher"]);
        assert_eq!(best.steps[0].config["shift"], 21);
        assert_eq!(replay(best, &input), Value::Text(PROSE.into()));
    }

    #[test]
    fn single_byte_xor_under_hex_is_found() {
        let input = encode(&[
            ("xor", json!({ "key": "5a", "keyFormat": "hex" })),
            ("base16-encode", json!({})),
        ]);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base16-decode", "xor", "utf8-decode"]);
        assert_eq!(best.steps[1].config["key"], "5a");
        assert_eq!(replay(best, &input), Value::Text(PROSE.into()));
    }

    #[test]
    fn url_and_decimal_encodings_are_found() {
        for encoder in ["url-encode", "to-decimal", "to-binary", "base32-encode"] {
            let input = encode(&[(encoder, json!({}))]);
            let proposals = suggest(&input);
            let best = proposals.first().unwrap_or_else(|| panic!("no proposal for {encoder}"));
            assert_eq!(replay(best, &input), Value::Text(PROSE.into()), "{encoder}");
        }
    }

    #[test]
    fn a_word_that_happens_to_be_valid_base64_is_left_alone() {
        // Eight letters, so it decodes as Base64, to garbage.
        assert!(suggest(&Value::Text("password".into())).is_empty());
        assert!(suggest(&Value::Text("deadbeef".into())).is_empty());
    }

    #[test]
    fn a_recognizable_file_is_a_valid_destination() {
        let png: Vec<u8> = b"\x89PNG\r\n\x1a\n".iter().copied().chain((0..64).map(|i| i * 3)).collect();
        let input = (ops::find("base64-standard-encode").unwrap().run)(
            &json!({}),
            &[&Value::Bytes(png.clone())],
        )
        .unwrap()
        .remove(0);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base64-standard-decode"]);
        assert_eq!(replay(best, &input), Value::Bytes(png));
    }

    #[test]
    fn compressed_data_is_inflated_rather_than_offered_as_a_file() {
        for (compress, inflate) in [("gzip", "gunzip"), ("zlib-deflate", "zlib-inflate")] {
            let input = encode(&[(compress, json!({})), ("base64-standard-encode", json!({}))]);
            let best = &suggest(&input)[0];
            assert_eq!(ops_of(best), ["base64-standard-decode", inflate, "utf8-decode"]);
            assert_eq!(replay(best, &input), Value::Text(PROSE.into()));
        }
    }

    #[test]
    fn a_json_web_token_is_opened() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
            eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
            SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let proposals = suggest(&Value::Text(token.into()));
        assert_eq!(ops_of(&proposals[0]), ["jwt-decode"]);
    }

    const LONG: &str = "The secret message is hidden in the last file that you will find \
        here. Nobody has opened it for years, and the key was written on the back of a map.";

    fn encode_text(text: &str, steps: &[(&str, serde_json::Value)]) -> Value {
        steps.iter().fold(Value::Text(text.into()), |value, (op, config)| {
            (ops::find(op).unwrap().run)(config, &[&value]).unwrap().remove(0)
        })
    }

    #[test]
    fn a_repeating_xor_key_is_recovered_under_base64() {
        let input = encode_text(
            LONG,
            &[("xor", json!({ "key": "nohut" })), ("base64-standard-encode", json!({}))],
        );
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base64-standard-decode", "xor", "utf8-decode"]);
        assert_eq!(best.steps[1].config, json!({ "key": "nohut", "keyFormat": "text" }));
        assert_eq!(replay(best, &input), Value::Text(LONG.into()));
    }

    #[test]
    fn a_vigenere_key_is_recovered() {
        let input = encode_text(LONG, &[("vigenere", json!({ "key": "nohut" }))]);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["vigenere"]);
        assert_eq!(best.steps[0].config, json!({ "key": "NOHUT", "decrypt": true }));
        assert_eq!(replay(best, &input), Value::Text(LONG.into()));
    }

    #[test]
    fn text_in_another_language_is_a_destination_too() {
        let turkish = "Gizli mesaj, burada bulacağın son dosyanın içinde saklı duruyor.";
        for encoder in ["base64-standard-encode", "base16-encode", "url-encode"] {
            let input = encode_text(turkish, &[(encoder, json!({}))]);
            let proposals = suggest(&input);
            let best = proposals.first().unwrap_or_else(|| panic!("no proposal for {encoder}"));
            assert_eq!(replay(best, &input), Value::Text(turkish.into()), "{encoder}");
        }
        assert!(suggest(&Value::Text(turkish.into())).is_empty());
    }

    #[test]
    fn compressed_data_larger_than_the_sample_is_still_followed() {
        // Words in an order that does not compress away, so the stream is long.
        let words = ["the", "message", "is", "hidden", "in", "a", "file", "that", "you", "will", "find", "here", "and", "nobody", "has", "opened", "it", "for", "years"];
        let mut state = 12345u32;
        let text: String = (0..120_000)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                words[(state >> 16) as usize % words.len()]
            })
            .collect::<Vec<_>>()
            .join(" ");
        let input = encode_text(&text, &[("gzip", json!({})), ("base64-standard-encode", json!({}))]);
        let Value::Text(encoded) = &input else { panic!("base64 is text") };
        assert!(encoded.len() > 2 * SAMPLE_BYTES, "the test needs input well past the sample");
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base64-standard-decode", "gunzip", "utf8-decode"]);
        assert_eq!(replay(best, &input), Value::Text(text));
    }

    #[test]
    fn large_input_is_sampled_and_the_steps_still_apply_to_all_of_it() {
        let long = PROSE.repeat(4000);
        let entry = ops::find("base64-standard-encode").unwrap();
        let input = (entry.run)(&json!({}), &[&Value::Text(long.clone())]).unwrap().remove(0);
        let best = &suggest(&input)[0];
        assert_eq!(ops_of(best), ["base64-standard-decode", "utf8-decode"]);
        assert_eq!(replay(best, &input), Value::Text(long));
    }
}
