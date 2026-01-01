# UTF8DOK

[![CI](https://github.com/utf8dok/utf8dok/actions/workflows/ci.yml/badge.svg)](https://github.com/utf8dok/utf8dok/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**Plain text, powerful docs.** A universal documentation platform with compliance validation, diagram support, and seamless editor integration.

## Quick Start

```bash
# Install (or download from Releases)
cargo install utf8dok-cli

# Create a new documentation workspace
utf8dok init my-docs

# Check compliance
cd my-docs
utf8dok audit
```

**5 seconds to a compliant workspace.**

## What It Does

UTF8DOK combines **three superpowers**:

### 1. Compliance Engine
Validate Architecture Decision Records (ADRs) against the Bridge Framework:
- Orphan detection (documents must be linked)
- Status validation (superseded ADRs need correct status)
- Required sections (Context, Decision, Consequences)

### 2. Content Intelligence
Real-time feedback as you write:
- Diagram syntax highlighting (Mermaid, PlantUML, D2)
- Writing quality suggestions (weasel words, passive voice)
- Cross-reference validation

### 3. CI/CD Integration
Automated documentation governance:
```yaml
# .github/workflows/ci.yml
- name: Documentation Audit
  run: utf8dok audit docs/ --strict --format markdown >> $GITHUB_STEP_SUMMARY
```

## Installation

### CLI

```bash
# From cargo
cargo install utf8dok-cli

# From source
git clone https://github.com/utf8dok/utf8dok
cd utf8dok
cargo build --release
```

### VS Code Extension

1. Download `utf8dok-vscode.vsix` from [Releases](https://github.com/utf8dok/utf8dok/releases)
2. Install: `code --install-extension utf8dok-vscode.vsix`

## Commands

| Command | Description |
|---------|-------------|
| `utf8dok init <path>` | Scaffold a new documentation workspace |
| `utf8dok audit [dir]` | Check compliance (CI/CD) |
| `utf8dok dashboard [dir]` | Generate HTML compliance report |
| `utf8dok check <file>` | Validate a single file |
| `utf8dok extract <docx>` | Extract AsciiDoc from DOCX |
| `utf8dok render <adoc>` | Render AsciiDoc to DOCX/PPTX |
| `utf8dok list-includes <file>` | List data includes in a document |

## Data Includes (Excel/CSV)

Embed data from Excel or CSV files directly in your documents:

```asciidoc
= Sales Report

== Quarterly Data

include::data/sales.xlsx[sheet=Q1,range=A1:D10,header]

== Regional Breakdown

include::data/regions.csv[header,delimiter=;]
```

Render with data resolution:

```bash
utf8dok render report.adoc --data-dir data/
```

Supported formats: `.xlsx`, `.xls`, `.csv`, `.tsv`

Attributes:
- `sheet=NAME` - Excel sheet name (defaults to first sheet)
- `range=A1:D10` - Cell range (supports `A:C`, `1:10`, `*`)
- `header` - Treat first row as header
- `delimiter=;` - Field delimiter (CSV only)

## Configuration

```toml
# utf8dok.toml
[workspace]
root = "."
entry_points = ["index.adoc", "README.adoc"]

[compliance.bridge]
orphans = "error"           # Documents must be reachable
superseded_status = "error" # Superseded ADRs need correct status
missing_status = "warning"  # Warn if :status: missing

[plugins]
writing_quality = true
diagrams = true
```

## Project Structure

```
crates/
├── utf8dok-core/      # Parser, diagnostics, traits
├── utf8dok-ast/       # AST type definitions
├── utf8dok-lsp/       # Language Server Protocol
├── utf8dok-cli/       # Command-line interface
├── utf8dok-ooxml/     # DOCX reading/writing
├── utf8dok-pptx/      # PPTX reading/writing
├── utf8dok-data/      # Data includes (Excel, CSV)
├── utf8dok-diagrams/  # Diagram rendering
├── utf8dok-validate/  # Validation engine
├── utf8dok-plugins/   # Rhai plugin system
└── utf8dok-wasm/      # WebAssembly bindings
editors/
└── vscode/            # VS Code extension
```

## Documentation

- **[User Guide](docs/manual.adoc)** - Installation, commands, configuration
- **[Architecture Decisions](docs/adr/)** - Design rationale

## Development

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace -- -D warnings

# Run audit on our own docs (dogfooding)
cargo run -p utf8dok-cli -- audit docs/
```

## Status

- [x] AsciiDoc parser (pest-based)
- [x] LSP server with real-time validation
- [x] Bridge Framework compliance engine
- [x] Diagram syntax highlighting
- [x] Writing quality suggestions
- [x] VS Code extension
- [x] CI/CD audit command
- [x] HTML compliance dashboard
- [x] Project scaffolding (`init` command)
- [x] DOCX round-trip (extract/render)
- [x] PPTX generation
- [x] Data includes (Excel, CSV, TSV)

## License

MIT OR Apache-2.0
