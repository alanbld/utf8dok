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
│   ├── utf8dok-pptx/      # PowerPoint generation (Dual-Nature)
│   ├── utf8dok-data/      # Data sources (Excel/XLSX) [In Progress]
│   ├── utf8dok-diagrams/  # Diagram rendering (Kroki, Mermaid, native)
│   ├── utf8dok-validate/  # Document validation engine
│   ├── utf8dok-plugins/   # Rhai plugin system
│   └── utf8dok-lsp/       # Language Server Protocol implementation
├── architecture/
│   ├── decisions/adr/     # Architecture Decision Records
│   ├── roadmap/           # Product vision & launch strategy
│   └── TECHNICAL_SPEC.md  # Detailed design
├── demo/                  # Demo documents and templates
├── ROADMAP_SCHEDULE.md    # 90-day development roadmap
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

> **See `ROADMAP_SCHEDULE.md` for detailed 90-day roadmap and checkpoint tracking.**

### Completed
- Phase 0: Compiler Foundation (AsciiDoc → IR)
- Phase 1-13: Core Validation, LSP, Compliance Platform
- Phase 20: Workspace Intelligence
- Phase 22: PPTX Generation Crate (`utf8dok-pptx`)
- Phase 23: Presentation Bridge (Dual-Nature Documents)
- OOXML template injection with cover page support
- Round-trip editing (embedded source in DOCX)

### In Progress
- **Phase 24: Data Engine** (`utf8dok-data`) - Excel/XLSX integration

### Upcoming (90-Day Roadmap)
- Month 1: Data Engine - `include::file.xlsx[range=A1:C10]`
- Month 2: Publishing Engine - Confluence/SharePoint integration
- Month 3: PDF Engine - Native PDF generation

### Key CLI Commands
```bash
# Render to DOCX (default)
utf8dok render doc.adoc --output doc.docx

# Render to PPTX (Dual-Nature)
utf8dok render slides.adoc --format pptx --output presentation.pptx

# Coming soon: Excel includes, PDF output, publishing
```

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

## DOCX Polish Sprints (Active)

**Goal:** Increase test coverage for `utf8dok-ooxml` crate systematically.

**Sprint Pattern:**
1. Explore coverage opportunities (find files with low test density)
2. Add tests for untested/undertested functions
3. Run tests and fix any errors
4. Commit with message: `test(ooxml): Sprint N - <description>`

**Completed Sprints:**

| Sprint | File | Tests Added | Commit |
|--------|------|-------------|--------|
| 18 | writer.rs | +20 (block generation) | `05e79a2` |
| 19 | style_map.rs | +17 (StyleContract, enums) | `3358ef7` |
| 20 | style_contract_validator.rs | +15 | `b1a22f7` |
| 21 | writer.rs | +19 (comments, content types, cover) | `fc9b8f3` |

**Current Status (after Sprint 21):**
- Total workspace tests: ~1,420
- `utf8dok-ooxml` tests: 508
- `writer.rs`: 72 tests / 4113 lines

**Next: Sprint 22**

To continue, run:
```bash
# Check test density per file
for f in crates/utf8dok-ooxml/src/*.rs; do
  tests=$(grep -c "#\[test\]" "$f" 2>/dev/null || echo 0)
  lines=$(wc -l < "$f")
  echo "$tests tests / $lines lines - $f"
done | sort -t/ -k1 -n

# Files with potential for more coverage:
# - document.rs: 43 tests / 1944 lines (~1 per 45 lines)
# - styles.rs: 34 tests / 1321 lines (~1 per 39 lines)
# - relationships.rs: 26 tests / 840 lines (~1 per 32 lines)
```

**Key test utilities:**
- `crate::test_utils::create_minimal_template()` - basic template without styles
- `crate::test_utils::create_template_with_styles()` - template with word/styles.xml
- `crate::test_utils::extract_document_xml(&result)` - extract document.xml from DOCX bytes
