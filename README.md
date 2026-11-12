# Cyber Nohut 🌰

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A visual toolkit for transforming data: encodings, hashes and classic ciphers, wired together as a graph. Inspired by CyberChef, with the operations and the graph engine written in Rust and compiled to WebAssembly. Everything runs in your browser.

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
- **Files in, files out.** A file node feeds raw bytes into a graph without going
  through text, and any output can be downloaded. File content is not saved with the
  graph.
- **Shareable.** A graph is plain data: it is saved locally, exported as JSON, or packed
  into a link.

### Operations

Base64 (standard and URL-safe), Base32, Base85 and hexadecimal encode/decode; MD5, SHA-1
and SHA-256; Caesar cipher and ROT13; reverse, case conversion, slice, split, join,
concatenate, word count and text statistics. Any single-input text operation can also be
run line by line.

## How it fits together

```
wasm/src/core    Value, the Op trait, errors: the contract every operation implements
wasm/src/ops     one file per family of operations, plus the list that registers them
wasm/src/graph   evaluates a whole graph: ordering, type checks, per-node reports
wasm/src/api     the wasm boundary: catalog(), evaluate(), output_text(), can_connect()
src/engine       the generated types, the worker and the page's client for it
src/graph        the graph as data, persistence, and the evaluation hooks
src/components   the React UI, driven entirely by the catalog
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

The engine is not faster than the browser's own built-ins, and the app does not claim
to be. The [benchmark page](bench.html) runs each pipeline in the engine and with the
fastest native equivalent, checks that both produce identical bytes, and reports the
median. One run on a 10 MB input, in a Chromium-based browser on macOS:

| Pipeline | Engine (ms) | Native (ms) | Engine is |
| --- | ---: | ---: | --- |
| SHA-256 | 19.3 | 3.8 | 5.1× slower |
| Base64 encode | 5.3 | 0.9 | 5.9× slower |
| Base64 decode | 12.2 | 4.4 | 2.8× slower |
| Hex encode | 6.0 | 3.2 | 1.9× slower |
| Caesar shift 3 | 19.7 | 16.5 | 1.2× slower |
| Uppercase, then reverse | 16.8 | 62.9 | 3.7× faster |
| Base64 encode → decode → hex → SHA-256 | 60.2 | 32.0 | 1.9× slower |

Web Crypto and the `Uint8Array` Base64/hex methods are native code, often using CPU
instructions that WebAssembly cannot reach. What the Rust core buys is one typed
definition per operation, correct handling of binary data, an engine that runs off the
main thread and survives running out of memory, and operations the browser does not
have at all (MD5, Base32, Base85).

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

Rebuild the wasm module with `wasm-pack build wasm` after changing Rust code, and run
the engine's tests with `cargo test --manifest-path wasm/Cargo.toml`.

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
