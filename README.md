# utf8dok 🦀

> Plain text, powerful docs. A blazing-fast UTF-8 document processor written in Rust.

[![Crates.io](https://img.shields.io/crates/v/utf8dok-core.svg)](https://crates.io/crates/utf8dok-core)
[![Documentation](https://docs.rs/utf8dok-core/badge.svg)](https://docs.rs/utf8dok-core)
[![Build Status](https://github.com/alanbld/utf8dok/workflows/CI/badge.svg)](https://github.com/alanbld/utf8dok/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](LICENSE-MIT)

## What is utf8dok?

A high-performance UTF-8 document processor targeting **Eclipse AsciiDoc TCK compliance** from day one.

### Goals

- 🎯 **Eclipse AsciiDoc TCK compliant** - Standards-first approach
- 🚀 **50x faster** than Ruby implementations
- 📦 **Single binary** - No runtime dependencies
- 🦀 **Pure Rust** - Memory safe, parallel processing
- 📚 **Multi-format future** - AsciiDoc today, Markdown tomorrow
- 🏗️ **BRIDGE documented** - Practicing what we preach

## Status

🚧 **Early Development** - Building TCK adapter and ASG implementation

## Architecture

Documented using [BRIDGE Framework](./BRIDGE.md):
- Architecture decisions: [`architecture/decisions/adr/`](./architecture/decisions/adr/)
- C4 models: [`architecture/software/`](./architecture/software/)
- All documentation is tested

## Standards Compliance

Building against Eclipse AsciiDoc specification:
- TCK (Technology Compatibility Kit) adapter mode
- ASG (Abstract Semantic Graph) JSON output
- Contributing learnings back to specification

## Related Projects

Learning from and potentially contributing to:
- [`asciidocr`](https://github.com/asciidoc-rust/asciidocr) - TCK implementation
- [`asciidork`](https://github.com/jirutka/asciidork) - Performance focused
- [Eclipse AsciiDoc WG](https://gitlab.eclipse.org/eclipse-wg/asciidoc) - Standards work

## Installation
```bash
cargo install utf8dok-cli
```

## Usage
```bash
# Coming soon
utf8dok input.adoc -o output.html
```

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
