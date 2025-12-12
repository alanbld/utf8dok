# ADR-002: Rust for AsciiDoc Processing

## Status
Accepted

## Context
Choosing the implementation language for a document processor involves trade-offs between:
- Development speed
- Runtime performance
- Memory safety
- Ecosystem maturity
- Target platform support

Languages considered:
- **Ruby**: Current Asciidoctor implementation
- **JavaScript/TypeScript**: Wide adoption, browser-native
- **Go**: Good performance, simple syntax
- **Rust**: Maximum performance, memory safety, WASM support

## Decision
Use Rust as the primary implementation language for utf8dok.

### Rationale
1. **Performance**: Rust provides C-level performance without sacrificing safety
2. **WASM**: First-class WebAssembly support via `wasm-bindgen`
3. **Memory Safety**: No garbage collector, no null pointer exceptions
4. **Concurrency**: Fearless concurrency for parallel document processing
5. **Type System**: Algebraic data types perfect for AST representation
6. **Tooling**: Cargo provides excellent dependency management and testing

## Consequences

### Positive
- Predictable, low-latency performance
- Single codebase for CLI, library, and WASM targets
- Strong compile-time guarantees
- Growing ecosystem with excellent parsing libraries (pest, nom, etc.)

### Negative
- Steeper learning curve than scripting languages
- Longer compilation times
- Smaller pool of potential contributors

## References
- [Rust and WebAssembly](https://rustwasm.github.io/docs/book/)
- [The Rust Programming Language](https://doc.rust-lang.org/book/)
