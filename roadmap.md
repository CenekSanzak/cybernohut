# Roadmap

## Done

- Operations defined once in Rust behind a typed contract; TypeScript types generated
- Graph evaluation in Rust, with values kept in wasm memory
- Evaluation in a Web Worker, with cancellation and crash recovery
- Typed ports, explicit bytes-to-text conversion, line-by-line mode
- Save, export, import and share links in one validated format
- File source nodes for binary input, and downloading any output
- A benchmark page that compares the engine with browser-native code
- Auto mode: proposed decoding steps become labelled nodes in the graph
- 42 operations, including XOR, URL encoding, regex, number bases and file type detection

## Next

1. **Share buffers between steps.** Steps that only reinterpret data, such as decoding
   bytes as UTF-8 text, copy it today. Sharing one buffer would remove that cost, which
   shows up in the Base64 decode benchmark. SHA-256 stays behind Web Crypto, which has
   CPU instructions WebAssembly cannot use.
2. **More operations.** AES and RSA, compression (gzip, zlib), JSON and timestamp
   helpers, CRC checksums, HMAC.
3. **A wider auto mode.** It finds layered encodings, letter shifts and single-byte
   XOR, and judges results by how much they read like English. Next: repeating-key XOR
   and Vigenère key recovery, compressed data once there are decompression operations,
   and scoring that is not tied to English.
4. **Command line.** The engine is a plain Rust crate; a small binary can run a saved
   graph on a file.

## Not planned

- Collaboration, team workspaces, real-time editing
- Forensics, memory analysis, network packet analysis
- A third-party plugin system. The contract is shaped so that operations could later be
  packaged as WebAssembly components, but nothing is built for it.
