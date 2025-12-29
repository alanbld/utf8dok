# Changelog

All notable changes to UTF8DOK are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-12-29

### Initial Release

UTF8DOK is a universal documentation platform with compliance validation, diagram support, and seamless editor integration.

### Added

#### Core Platform
- **AsciiDoc Parser**: High-performance pest-based parser with full block/inline support
- **AST Types**: Rich document model supporting headings, paragraphs, lists, tables, code blocks
- **Diagnostics System**: Structured error reporting with severity levels and code locations

#### Language Server Protocol (LSP)
- Real-time validation as you type
- Go-to-definition for cross-references (`[[id]]` and `<<id>>`)
- Find all references across workspace
- Workspace symbol search
- Document symbols (outline view)
- Semantic token highlighting

#### Compliance Engine
- **Bridge Framework** support for Architecture Decision Records (ADRs)
- Orphan detection (documents must be reachable from entry points)
- Status validation (superseded ADRs require correct status)
- Missing attribute warnings (`:status:`, `:date:`)
- Configurable severity levels (error, warning, info, off)

#### Content Intelligence
- **Diagram Support**: Mermaid, PlantUML, D2 syntax highlighting
- **Writing Quality**: Weasel word detection, passive voice warnings
- Cross-reference validation

#### Command-Line Interface
- `utf8dok init` - Scaffold new documentation workspace
- `utf8dok audit` - CI/CD compliance checking (text, JSON, markdown output)
- `utf8dok dashboard` - Generate HTML compliance reports
- `utf8dok check` - Validate single files
- `utf8dok extract` - Extract AsciiDoc from DOCX
- `utf8dok render` - Render AsciiDoc to DOCX with template

#### VS Code Extension
- LSP client integration
- Status bar with compliance status
- Compliance dashboard webview
- Commands: Show Dashboard, Run Audit, Fix All, Restart Server

#### CI/CD Integration
- GitHub Actions workflow for automated audits
- Markdown output for PR comments
- Strict mode for blocking merges
- Release workflow for multi-platform binaries

#### Documentation
- User Guide (`docs/manual.adoc`)
- Architecture Decision Records
- Dogfooding: UTF8DOK validates its own documentation

### Templates

Two scaffolding templates available via `utf8dok init`:

- **Bridge**: ADR-focused workspace with compliance rules
- **Basic**: Simple AsciiDoc project structure

### Platforms

Pre-built binaries available for:
- Linux (x64)
- macOS (x64, ARM64)
- Windows (x64)

### Minimum Supported Rust Version

Rust 1.70.0

---

## Development Phases

This release represents 20 phases of development:

| Phase | Feature |
|-------|---------|
| 1-5 | Core parser and AST |
| 6 | DOCX round-trip conversion |
| 7-9 | Validation engine and plugins |
| 10 | Universal platform foundation |
| 11 | Workspace intelligence |
| 12 | Compliance engine |
| 13 | Compliance dashboard |
| 14 | Configuration engine |
| 15 | Active assistance |
| 16 | CI/CD auditor |
| 17 | Rich content intelligence |
| 18 | VS Code extension |
| 19 | Production readiness |
| 20 | Day 1 experience |

[1.0.0]: https://github.com/utf8dok/utf8dok/releases/tag/v1.0.0
