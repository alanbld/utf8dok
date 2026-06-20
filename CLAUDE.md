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
│   ├── utf8dok-data/      # Data sources (Excel/XLSX, CSV) via calamine
│   ├── utf8dok-pdf/       # PDF generation via Typst backend
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

- **Hand-written line-based state machine** for AsciiDoc parsing
  (`crates/utf8dok-core/src/parser.rs`), with regex-based inline parsing.
  Note: pest was originally selected in ADR-003 but never implemented — ADR-003
  is superseded.
- **tower-lsp** for Language Server Protocol
- **rhai** for plugin scripting
- **calamine** for Excel/XLSX, **Typst** for PDF generation
- Edition: 2021 (no `rust-version`/MSRV is currently enforced in Cargo)

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

**Status (started 2026-06-20):** A dedicated, location-aware ASG adapter lives in
`crates/utf8dok-core/src/asg/` — separate from the OOXML `parser` (which discards
source positions). It emits the Eclipse ASG node model (`document`/`paragraph`/
`text`, every node with a `location`).

- **Adapter CLI**: `utf8dok asg` reads the TCK request envelope
  `{ "contents", "path", "type": "block"|"inline" }` from stdin and writes ASG
  JSON to stdout — the official adapter contract (`ASCIIDOC_TCK_ADAPTER`).
- **Conformance tests**: `crates/utf8dok-core/tests/tck.rs` deep-compares emitted
  ASG against vendored upstream fixtures under `tests/tck/` (EPL-2.0, see
  `ATTRIBUTION.md`). Grow coverage by vendoring more `-input.adoc`/`-output.json`
  pairs (preserve the `block/`/`inline/` prefix — it selects comparison mode).
- **Current coverage**: **13/13 vendored fixtures green** — documents,
  paragraphs, plain text, header (title+attributes), recursive sections
  (`level`), unordered lists, `----` listing + `****` sidebar delimited blocks,
  and inline constrained `strong` spans (`*…*`). The full vendored TCK subset
  passes via both the Rust harness and the real `utf8dok asg` binary.
- **Beyond the vendored subset**: the upstream TCK has many more tests; vendor
  more `-input.adoc`/`-output.json` pairs to expand coverage. Known gaps: ordered
  lists, nested/`-` list markers, list continuations, admonitions, tables,
  block-level inline markup (paragraphs currently emit a single `text` node),
  and other inline forms (emphasis, code, links).

## Current Implementation Status

> **See `ROADMAP_SCHEDULE.md` for detailed 90-day roadmap and checkpoint tracking.**

### Completed
- Phase 0: Compiler Foundation (AsciiDoc → IR)
- Phase 1-13: Core Validation, LSP, Compliance Platform
- Phase 20: Workspace Intelligence
- Phase 22: PPTX Generation Crate (`utf8dok-pptx`)
- Phase 23: Presentation Bridge (Dual-Nature Documents)
- **Phase 24: Data Engine** (`utf8dok-data`) - Excel/XLSX + CSV via calamine,
  wired into core (`include::file.xlsx[range=A1:C10]`)
- OOXML template injection with cover page support
- Round-trip editing (embedded source in DOCX)

### In Progress
- **Phase 25: PDF Engine** (`utf8dok-pdf`) - crate scaffolded with a Typst
  backend (`Transpiler` + `Compiler`); wiring `utf8dok render --format pdf`
- Parser feature track toward Eclipse AsciiDoc TCK (links, anchors,
  ifdef/preprocessor, page breaks, attribute substitution, `a|` cells)

### Upcoming (90-Day Roadmap)
- PDF Engine completion: `utf8dok render --format pdf` (Checkpoint 3)
- Publishing Engine - Confluence/SharePoint integration
- DOCX Polish - cover images, table styling, diagram embedding

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
| 22 | document.rs | +28 (formatting, images, DrawingML) | `8541e69` |
| 23 | extract.rs | +41 (style contract, hyperlinks, images, conversions) | `37e6411` |
| 24 | writer.rs | +27 (block gen, inlines, lists, tables) | `232467d` |
| 25 | styles.rs | +30 (StyleMap, StyleSheet, from_stylesheet) | `e94e3ff` |

**Current Status (verified 2026-06-19):**
- Total workspace tests: ~1,366 (`#[test]` attributes across all crates)
- `utf8dok-ooxml` tests: 748
- `styles.rs`: 64 tests / 1854 lines (~28 lines/test)

> Note: after Sprint 25 the work shifted from the `test(ooxml)` unit-test
> cadence to a feature track (URL links, heading anchors, quality-metrics
> integration tests, and parser hardening). The numbered "Sprint 26" was never
> started. If resuming the unit-test cadence, the priority targets below stand.

**Next (if resuming the ooxml unit-test cadence): writer.rs**

Priority files for coverage (highest lines/test first):
- `writer.rs`: 99 tests / 4800 lines (~48 lines/test)
- `document.rs`: 71 tests / 2818 lines (~39 lines/test)
- `extract.rs`: 86 tests / 3258 lines (~37 lines/test)
- `conversion.rs`: 44 tests / 1487 lines (~33 lines/test)

To continue, run:
```bash
# Check test density per file
for f in crates/utf8dok-ooxml/src/*.rs; do
  tests=$(grep -c "#\[test\]" "$f" 2>/dev/null || echo 0)
  lines=$(wc -l < "$f")
  if [ "$tests" -gt 0 ]; then
    ratio=$((lines / tests))
    echo "$ratio lines/test | $tests tests / $lines lines - $(basename $f)"
  fi
done | sort -t'|' -k1 -rn | head -10
```

**Key test utilities:**
- `crate::test_utils::create_minimal_template()` - basic template without styles
- `crate::test_utils::create_template_with_styles()` - template with word/styles.xml
- `crate::test_utils::extract_document_xml(&result)` - extract document.xml from DOCX bytes
