# Roadmap

## Done

- Operations defined once in Rust behind a typed contract; TypeScript types generated
- Graph evaluation in Rust, with values kept in wasm memory
- Evaluation in a Web Worker, with cancellation and crash recovery
- Typed ports, explicit bytes-to-text conversion, line-by-line mode
- Save, export, import and share links in one validated format
- File source nodes for binary input, and downloading any output
- A benchmark page that compares the engine with browser-native code

## Next

1. **Close the gap where it is cheap.** The benchmark shows the engine behind the
   browser's built-ins on hashing and Base64. Candidates: SIMD Base64 and hex, and
   skipping the copies between steps that only reinterpret data (text to bytes).
2. **More operations.** XOR, URL encoding, regex extract/replace, number-base
   conversions, magic-byte detection.
3. **Auto mode.** A search over operations that proposes decoding steps for unknown
   input. Its result is a graph fragment, not a value: each proposed step shows why it
   was chosen and becomes an ordinary node once accepted. Needs a per-operation
   `detect` hook, encode/decode inverse pairs in the catalog, and output scoring.
4. **Command line.** The engine is a plain Rust crate; a small binary can run a saved
   graph on a file.

## Not planned

- Collaboration, team workspaces, real-time editing
- Forensics, memory analysis, network packet analysis
- A third-party plugin system. The contract is shaped so that operations could later be
  packaged as WebAssembly components, but nothing is built for it.
