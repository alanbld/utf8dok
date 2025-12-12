# ADR-001: Why utf8dok

## Status
Accepted

## Context
The documentation ecosystem needs a modern, fast, and flexible document processor that:
- Handles UTF-8 text formats natively
- Provides excellent performance for large documents
- Supports multiple output formats
- Can run in browsers via WebAssembly
- Has a clean, well-documented codebase

Existing solutions like Asciidoctor (Ruby) are mature but:
- Have performance limitations for large-scale processing
- Cannot easily run in browsers
- Have complex codebases that are difficult to extend

## Decision
Create utf8dok as a new document processor written in Rust, focusing on:
1. **Performance**: Leverage Rust's zero-cost abstractions
2. **Portability**: Compile to native binaries and WASM
3. **Correctness**: Strong type system to prevent runtime errors
4. **Documentation**: Use BRIDGE framework to dogfood our own documentation

## Consequences

### Positive
- Fast processing of large documents
- Browser compatibility via WASM
- Memory safety without garbage collection
- Modern tooling and ecosystem

### Negative
- Smaller contributor pool compared to Ruby/JavaScript
- Initial development investment to reach feature parity
- Learning curve for contributors unfamiliar with Rust

## References
- [AsciiDoc Language Specification](https://docs.asciidoctor.org/asciidoc/latest/)
- [Rust Programming Language](https://www.rust-lang.org/)
