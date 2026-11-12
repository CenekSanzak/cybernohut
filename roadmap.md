# Roadmap

## Done

- Operations defined once in Rust behind a typed contract; TypeScript types generated
- Graph evaluation in Rust, with values kept in wasm memory
- Evaluation in a Web Worker, with cancellation and crash recovery
- Typed ports, explicit bytes-to-text conversion, line-by-line mode
- Save, export, import and share links in one validated format
- File source nodes for binary input, by picker or drag and drop, and downloading any
  output
- A benchmark page that compares the engine with browser-native code
- Auto mode: proposed decoding steps become labelled nodes in the graph. It finds
  layered encodings, compressed data, JSON Web Tokens, letter shifts, and XOR and
  Vigenère keys, and recognizes prose in languages other than English
- 94 operations, including AES, textbook RSA with PEM keys and weak-modulus factoring,
  HMAC, SHA-3, gzip and zlib, JSON, JWT, timestamps, frequency analysis, strings, hex
  dumps, Base58, Morse and LSB steganography in PNG and BMP
- Undo and redo
- A command-line tool that runs single operations, exported graphs and auto mode
- A memory budget for results: beyond it, finished results are released and steps
  rewrite their input in place

## Next

1. **Streaming.** A run holds each step's whole input and output, which is what limits
   files to 256 MB. Hashes and compression could process their input in pieces.
2. **More operations.** RSA with OAEP and key generation, more compression formats
   (bzip2, zstd, zip listing), character set conversion, BLAKE hashes.
3. **Images with fewer than eight bits per value.** LSB steganography reads 8- and
   16-bit PNG, interlaced or not, and 24- and 32-bit BMP. 1-, 2- and 4-bit PNG are not
   decoded.
4. **Key recovery in more languages.** XOR keys are found over text in any language;
   Vigenère keys for five. Each further language is a table of letter frequencies.
5. **A drawn frequency chart.** Frequency analysis is a text table with bars.

## Not planned

- Collaboration, team workspaces, real-time editing
- Forensics, memory analysis, network packet analysis, audio steganography
- A third-party plugin system. The contract is shaped so that operations could later be
  packaged as WebAssembly components, but nothing is built for it.
