# Cyber Nohut 🌰

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A visual toolkit for transforming data: encodings, hashes, ciphers and compression, wired together as a graph. Inspired by CyberChef, with the operations and the graph engine written in Rust and compiled to WebAssembly. Everything runs in your browser, and the same Rust core is also a [command-line tool](#command-line).

## Features

- **Visual graph instead of a linear recipe.** Operations are nodes; an output can feed
  several branches, and an operation can take several inputs. Every intermediate result
  is visible on its node.
- **One engine, written in Rust.** Operations, their types and their settings are
  defined once in Rust and compiled to WebAssembly. The whole graph is evaluated in one
  call, so intermediate values never leave wasm memory.
- **Typed connections.** Ports are `text`, `bytes`, `number` or lists. Text feeds a
  bytes port as its UTF-8 encoding; going the other way can fail, so connecting bytes to
  text adds a visible "Decode Text (UTF-8)" step rather than converting silently.
- **Runs off the main thread.** Evaluation happens in a Web Worker. A newer run cancels
  the one in flight, and an out-of-memory crash restarts the engine instead of the page.
- **Auto mode that shows its work.** "Suggest steps" searches for the chain of
  operations that turns a value into something readable: layered Base64, hex, URL and
  number encodings, gzip and zlib, JSON Web Tokens, Caesar and ROT13 shifts, and XOR and
  Vigenère with the key recovered. The answer is added to the graph as ordinary nodes,
  each labelled with why it was chosen, so it can be read, changed and reused on other
  input. Readable means prose in any language written with spaces; recovering a cipher
  key relies on English letter frequencies.
- **Large inputs.** Up to 512 MB of results are kept so any step's output shows at
  once. Past that, results no later step needs are released, steps that can work in
  place reuse their input's memory, and looking at a released step runs the graph again.
- **Files in, files out.** A file dropped on the canvas becomes a node that feeds its
  raw bytes into the graph without going through text, and any output can be downloaded. File content is not saved with the
  graph.
- **Shareable.** A graph is plain data: it is saved locally, exported as JSON, or packed
  into a link.

### Operations

57 operations:

- **Encoding:** Base64 (standard and URL-safe), Base32, Base85, hexadecimal, URL
  percent-encoding; bytes as binary, octal or decimal numbers
- **Compression:** gzip, zlib and raw DEFLATE, both ways
- **Encryption:** AES in CBC, CTR, GCM and ECB with 128, 192 or 256-bit keys; XOR with a
  repeating key
- **Hashing:** MD5, SHA-1, SHA-256, SHA-512, HMAC, CRC32
- **Classic ciphers:** Caesar, ROT13, Atbash, Vigenère
- **Data formats:** JSON format and minify, JWT decode, Unix timestamps to and from
  ISO 8601
- **Text:** regex extract and replace, reverse, case conversion, slice, split, join,
  concatenate, word count, statistics, UTF-8 decoding
- **Analysis:** file type detection from signature bytes, Shannon entropy

Any single-input operation that produces text can also be run line by line.

## How it fits together

```
wasm/src/core    Value, the Op trait, errors: the contract every operation implements
wasm/src/ops     one file per family of operations, plus the list that registers them
wasm/src/graph   evaluates a whole graph: ordering, type checks, per-node reports
wasm/src/auto    auto mode: the search over operations' detect() hooks
wasm/src/score   how readable a value is, which is what the search optimizes
wasm/src/api     the wasm boundary: catalog(), evaluate(), suggest(), output_text()
src/engine       the generated types, the worker and the page's client for it
src/graph        the graph as data, persistence, and the evaluation hooks
src/components   the React UI, driven entirely by the catalog
src/nodes        what a node shows: ports, settings form, result
cli              the command-line tool, a thin layer over the wasm crate built natively
```

### Adding an operation

Implement `Op` in `wasm/src/ops` and add one line to the `ops!` list in
`wasm/src/ops/mod.rs`. Port types come from the `run` signature, and the settings form
is generated from `params()`, so there is nothing to write in TypeScript: the sidebar
entry, the node, its settings and its TypeScript config type all follow.

```rust
pub struct Reverse;

impl Op for Reverse {
    const ID: &'static str = "reverse";
    const NAME: &'static str = "Reverse";
    const DESCRIPTION: &'static str = "Reverses the order of the characters";
    const TAGS: &'static [Tag] = &[Tag::Text];

    type Config = NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &NoConfig, input: &str) -> Result<String, OpError> {
        Ok(input.chars().rev().collect())
    }
}
```

## Performance

The engine is faster than the browser's own built-ins in some places and slower in
others, and the app does not claim more than that. The [benchmark page](bench.html)
runs each pipeline in the engine and with the fastest native equivalent, checks that
both produce identical bytes, and reports the median. One run on a 10 MB input, in a
Chromium-based browser on macOS:

| Pipeline | Engine (ms) | Native (ms) | Engine is |
| --- | ---: | ---: | --- |
| SHA-256 | 18.6 | 3.8 | 4.9× slower |
| Base64 encode | 1.3 | 1.1 | 1.2× slower |
| Base64 decode | 7.5 | 4.1 | 1.8× slower |
| Hex encode | 0.7 | 3.7 | 5.3× faster |
| Caesar shift 3 | 5.4 | 13.9 | 2.6× faster |
| Uppercase, then reverse | 13.7 | 65.4 | 4.8× faster |
| Base64 encode → decode → hex → SHA-256 | 46.0 | 29.5 | 1.6× slower |

SHA-256 in Web Crypto uses dedicated CPU instructions that WebAssembly cannot reach,
and it dominates the chained pipeline. Base64 and hex use WebAssembly SIMD. Beyond
speed, the Rust core gives one typed definition per operation, correct handling of
binary data, an engine that runs off the main thread and survives running out of
memory, and operations the browser does not have at all (MD5, Base32, Base85, XOR, the classic ciphers).

## Command line

The operations, the graph engine and auto mode are a plain Rust crate, so the same code
that runs in the browser as WebAssembly also builds into a native binary:

```bash
cargo install --path cli
```

One operation is a filter, with its settings as options:

```bash
echo 'hello' | cybernohut base64-standard-encode
cybernohut sha256 -i photo.jpg
cybernohut aes-decrypt --mode gcm --key-format hex --key 00112233445566778899aabbccddeeff --iv 000000000000000000000000 -i secret.bin
```

Auto mode explains what it found, applies it, or writes it as a graph:

```bash
cybernohut auto -i mystery.txt            # the suggested steps and why
cybernohut auto --apply -i mystery.txt    # the decoded result
cybernohut auto --graph -i mystery.txt > steps.json
```

A graph, whether written by `auto --graph` or exported from the web app, runs on other
data, which makes a decoding worked out once repeatable:

```bash
cybernohut run steps.json -i other.txt
```

`cybernohut list` shows every operation and `cybernohut help <operation>` its settings.

## Getting Started

### Prerequisites

- Node.js (v20 or higher recommended)
- [Rust](https://www.rust-lang.org/tools/install) (stable) with the `wasm32-unknown-unknown` target
- [wasm-pack](https://github.com/rustwasm/wasm-pack)

### Installation

The WebAssembly package has to be built before installing the npm dependencies, because the app depends on `wasm/pkg`.

```bash
# Clone the repository
git clone https://github.com/CenekSanzak/cybernohut.git
cd cybernohut

# Build the wasm module (creates wasm/pkg)
wasm-pack build wasm

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

Rebuild the wasm module with `wasm-pack build wasm` after changing Rust code. The
engine's tests run with `cargo test --manifest-path wasm/Cargo.toml` and the command-line
tool's with `cargo test --manifest-path cli/Cargo.toml`.

## Tech Stack

- **Frontend**: React 18 with TypeScript
- **Build Tool**: Vite
- **Styling**: TailwindCSS
- **Graph Visualization**: @xyflow/react
- **Core Operations**: Rust (WebAssembly)
- **UI Components**: Custom components with Radix UI primitives

## Roadmap

See the [roadmap](roadmap.md).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by [CyberChef](https://github.com/gchq/CyberChef)
- Built with [React Flow](https://reactflow.dev/)
- WebAssembly core powered by Rust
