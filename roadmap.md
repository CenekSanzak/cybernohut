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
- 75 operations, including AES, textbook RSA with weak-modulus factoring, HMAC, gzip and
  zlib, JSON, JWT, timestamps, frequency analysis and PNG/JPEG steganography
- Undo and redo
- A command-line tool that runs single operations, exported graphs and auto mode
- A memory budget for results: beyond it, finished results are released and steps
  rewrite their input in place

## Next

1. **Key recovery beyond English.** Finding an XOR or Vigenère key uses English letter
   frequencies, so a ciphertext of text in another language is not solved. Frequency
   tables for a few more languages would fix the common cases.
2. **More operations.** RSA with OAEP and key files (PEM), more compression formats
   (bzip2, zstd, zip listing), character set conversion, more hashes (SHA-3, BLAKE).
3. **More image formats for steganography.** LSB extraction reads 8-bit, non-interlaced
   PNG. 16-bit and interlaced PNG, BMP and LSB insertion are not there.
4. **More steps that work in place.** Only UTF-8 decode, XOR, ROT13 and Caesar take
   over their input's memory today. Case conversion, Atbash and Vigenère could as well.
5. **Streaming.** A run holds each step's whole input and output, which is what limits
   files to 256 MB. Hashes and compression could process their input in pieces.
6. **Auto mode on truncated data.** Auto mode tries steps on the first 64 KB. A
   compressed stream cut off there does not decompress, so compressed data in larger
   inputs is recognized but not followed further.

## Not planned

- Collaboration, team workspaces, real-time editing
- Forensics, memory analysis, network packet analysis, audio steganography
- Charts: frequency analysis is a text table with bars, not a drawn graph
- A third-party plugin system. The contract is shaped so that operations could later be
  packaged as WebAssembly components, but nothing is built for it.
