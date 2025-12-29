# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**utf8dok** is a high-performance template-aware document processor written in Rust, targeting **Eclipse AsciiDoc TCK (Technology Compatibility Kit) compliance**. The project transforms AsciiDoc into corporate-compliant DOCX with round-trip editing capability.

**Key Differentiator**: Unlike tools that generate DOCX from scratch, utf8dok injects content into `.dotx` templates, producing documents that match corporate standards with embedded sources for lossless round-trips.

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

# Run CLI
cargo run -p utf8dok-cli -- <args>
```

## Architecture

### Workspace Structure

```
utf8dok/
├── crates/
│   ├── utf8dok-core/      # Core parsing, AST, diagnostics
│   ├── utf8dok-ast/       # AST type definitions (ASG output format)
│   ├── utf8dok-cli/       # Command-line interface (clap)
│   ├── utf8dok-wasm/      # WebAssembly bindings
│   ├── utf8dok-ooxml/     # OOXML (.docx/.dotx) reading/writing
│   ├── utf8dok-diagrams/  # Diagram rendering (Kroki, Mermaid, native)
│   ├── utf8dok-validate/  # Document validation engine
│   ├── utf8dok-plugins/   # Rhai plugin system
│   └── utf8dok-lsp/       # Language Server Protocol implementation
├── architecture/
│   ├── decisions/adr/     # Architecture Decision Records
│   ├── roadmap/           # Product vision & launch strategy
│   └── TECHNICAL_SPEC.md  # Detailed design
└── Cargo.toml             # Workspace manifest
```

### Crate Dependency Flow

```
utf8dok-core (traits, parser, diagnostics)
    ↑
utf8dok-ast (data structures)
    ↑
utf8dok-ooxml, utf8dok-diagrams (implementations)
    ↑
utf8dok-validate, utf8dok-plugins (analysis)
    ↑
utf8dok-cli, utf8dok-wasm, utf8dok-lsp (interfaces)
```

### Key Technologies

- **pest** (PEG parser generator) for AsciiDoc parsing - see ADR-003
- **tower-lsp** for Language Server Protocol
- **rhai** for plugin scripting
- MSRV: Rust 1.70.0

## Core Workflows

### Extract (DOCX → AsciiDoc)
Bootstrap AsciiDoc authoring from existing documents:
```bash
utf8dok extract document.docx --output project/
```

### Render (AsciiDoc → DOCX)
Generate corporate-compliant documents:
```bash
utf8dok render document.adoc --output final.docx
```

### Validate
Check documents against rules and plugins:
```bash
utf8dok validate document.adoc --config utf8dok.toml
```

## AI Collaboration Protocol

See `SYSTEM_INSTRUCTIONS.md` for the complete AI Collaboration Protocol v3.0 (VO-Native).

**Core concepts:**
- **Session ID**: `YYYY-MM-DD | utf8dok-{hash} | Turn: N`
- **Task Prefixes**: `survey:`, `feature:`, `fix:`, `improve:`, `document:`, `test:`, `explore:`, `zoom:`
- **VO Integration**: Use `explore_with_intent`, `get_context`, `zoom` before development
- **Context Budgets**: 5k (quick), 15k (architecture), 20k (debug)
- **Session Handoff**: Always end with summary + pending tasks

## Documentation Framework (BRIDGE)

- **ADRs**: Architecture Decision Records in `architecture/decisions/adr/`
- **C4 Models**: Software architecture diagrams in `architecture/software/workspace.dsl`
- **Technical Spec**: Detailed design in `architecture/TECHNICAL_SPEC.md`
- **Product Vision**: `architecture/roadmap/PRODUCT_VISION.md`
- **Launch Strategy**: `architecture/roadmap/LAUNCH_STRATEGY.md`

When making significant architectural decisions, create an ADR following the template at `architecture/decisions/adr/template.md`.

## Code Standards

### Rust Conventions

- **Error Handling**: Use `thiserror` for library errors, `anyhow` for CLI
- **Traits**: Define in `utf8dok-core`, implement in format crates
- **Testing**: Unit tests in modules, integration tests in `tests/`
- **Documentation**: Doc comments on all public APIs

### Key Traits (utf8dok-core)

```rust
/// Main extraction trait
pub trait DocumentExtractor {
    fn extract(&self, input: &[u8]) -> Result<Document>;
    fn supported_extensions(&self) -> &[&str];
}

/// Format detection
pub trait FormatDetector {
    fn detect(&self, input: &[u8]) -> Option<DocumentFormat>;
}
```

## TCK Compliance

The project aims to pass the Eclipse AsciiDoc TCK. Development follows a TCK-first approach:
1. Implement parser features to match ASG JSON output format
2. Validate against official TCK test cases
3. Report any specification ambiguities back to Eclipse WG

## Current Implementation Status

### Completed
- Phase 0: Compiler Foundation (AsciiDoc → IR)
- Phase 1: Core Validation (native validators)
- Phase 2: LLM Integration (`--llm-check`)
- Phase 3: Rhai Plugin System
- Phase 3.5: Diagnostic-Only LSP
- Tier 1 & 2: Diagram engines (Kroki, native Mermaid)

### In Progress
- OOXML template injection
- Round-trip editing support

## Code Coverage Convention ("rosebud")

The keyword **"rosebud"** triggers a code coverage workflow:

1. **Run tarpaulin**: `cargo tarpaulin --config tarpaulin.toml`
2. **Report coverage**: Provide full coverage statistics per crate
3. **Bold action**: Suggest specific, high-impact tests to extend coverage

### Tarpaulin Configuration

Coverage is configured via `tarpaulin.toml`:
- Output: HTML + XML reports in `coverage/`
- Excludes: Test files (`**/tests/*`, `**/test_*.rs`)
- Timeout: 120s per test
- Engine: Ptrace

### Coverage Targets

| Crate | Target | Priority |
|-------|--------|----------|
| `utf8dok-core` | 80%+ | High |
| `utf8dok-validate` | 80%+ | High |
| `utf8dok-lsp` | 70%+ | Medium |
| `utf8dok-cli` | 60%+ | Low (integration) |

### Running Coverage

```bash
# Full workspace coverage
cargo tarpaulin --config tarpaulin.toml

# HTML report
open coverage/tarpaulin-report.html

# Specific crate
cargo tarpaulin -p utf8dok-core --config tarpaulin.toml
```

## Session Handoff

For cross-session continuity:

```bash
# 1. Survey first
vo . --survey composition

# 2. Check git status
git status && git log -5 --oneline

# 3. Run tests
cargo test --workspace
```
