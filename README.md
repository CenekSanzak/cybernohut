# Cyber Nohut 🌰

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A modern, visual toolkit for data transformation, encoding, encryption and CTF challenges. Inspired by CyberChef, but built with modern web technologies and a focus on visual programming. Leveraging Rust and WebAssembly for high-performance operations, it provides blazing-fast processing capabilities right in your browser.

## Features

- 🎨 **Visual Programming Interface**
  - Create data processing pipelines using an intuitive drag-and-drop interface
  - Connect operations with visual flows
  - Real-time preview of results at each step

- ⚡ **High Performance**
  - Core operations implemented in Rust and compiled to WebAssembly
  - Efficient processing of large data sets
  - Client-side processing for data privacy

- 🧩 **Modular Operation System**
  - Easy to add new operations
  - Type-safe connections between operations
  - Categorized operation sidebar for easy discovery

- 🔄 **Current Operations**
  - String manipulations (reverse, case conversion, whitespace removal)
  - More coming soon!

## Getting Started

### Prerequisites

- Node.js (v18 or higher recommended)
- npm or yarn

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/cyber-nohut.git

# Navigate to project directory
cd cyber-nohut

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build
```

### Building the Rust WebAssembly Part

You need to have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash

# Install wasm-pack
cargo install wasm-pack

# Go to wasm directory
cd wasm

# Build the wasm module (No target because the default is loader (like webpack))
wasm-pack build 

# Go back to the root directory
cd ..

```

## Tech Stack

- **Frontend**: React 18 with TypeScript
- **Build Tool**: Vite
- **Styling**: TailwindCSS
- **Graph Visualization**: @xyflow/react
- **Core Operations**: Rust (WebAssembly)
- **UI Components**: Custom components with Radix UI primitives

## Differences from CyberChef

- **Modern Tech Stack**: Built with React, TypeScript, and Rust/WebAssembly
- **Visual Programming**: True visual programming interface instead of linear recipe list
- **Type Safety**: Built-in type checking between operations
- **Performance**: Core operations implemented in Rust for maximum performance
- **Real-time Preview**: See results update in real-time as you build your pipeline

## Roadmap

See our detailed [roadmap](roadmap.md) for planned features and improvements.

Quick overview:

- v0.1.x: Core CTF essentials (encodings, ciphers, hashing)
- v0.2.x: File handling & basic steganography
- v0.3.x: Advanced features (RSA, AES, forensics)

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
