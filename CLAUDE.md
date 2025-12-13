# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

utf8dok is a high-performance UTF-8 document processor written in Rust, targeting **Eclipse AsciiDoc TCK (Technology Compatibility Kit) compliance** from day one. The project uses a TCK-first development strategy - meaning all parsing features are validated against the official AsciiDoc specification tests.

## Build Commands

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run a specific test
cargo test <test_name>

# Run tests for a specific crate
cargo test -p utf8dok-core

# Format code
cargo fmt --all

# Lint with clippy (CI runs with -D warnings)
cargo clippy --workspace -- -D warnings

# Build WASM target
wasm-pack build crates/utf8dok-wasm --target web

# Build documentation
cargo doc --workspace --no-deps
```

## Architecture

### Workspace Structure

The project is a Cargo workspace with four crates:

- **utf8dok-core**: Core parsing and processing library. Contains the main document parser logic.
- **utf8dok-ast**: AST (Abstract Syntax Tree) type definitions. Will implement ASG (Abstract Semantic Graph) output format for TCK compliance.
- **utf8dok-cli**: Command-line interface using clap.
- **utf8dok-wasm**: WebAssembly bindings using wasm-bindgen for browser/Node.js usage.

### Key Technologies

- **pest** (PEG parser generator) for parsing - see ADR-003
- MSRV: Rust 1.70.0

### Documentation Framework (BRIDGE)

The project uses the BRIDGE documentation framework:

- **ADRs**: Architecture Decision Records in `architecture/decisions/adr/`
- **C4 Models**: Software architecture diagrams in `architecture/software/workspace.dsl`
- **Technical Spec**: Detailed design in `architecture/TECHNICAL_SPEC.md`

When making significant architectural decisions, create an ADR following the template at `architecture/decisions/adr/template.md`.

## TCK Compliance

The project aims to pass the Eclipse AsciiDoc TCK. Development follows a TCK-first approach:
1. Implement parser features to match ASG JSON output format
2. Validate against official TCK test cases
3. Report any specification ambiguities back to Eclipse WG
