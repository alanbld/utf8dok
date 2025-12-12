# BRIDGE Documentation Framework

utf8dok uses the BRIDGE framework for comprehensive documentation that bridges
the gap between code and understanding.

## What is BRIDGE?

BRIDGE is a documentation methodology that ensures documentation remains:
- **B**alanced: Right level of detail for each audience
- **R**elevant: Always up-to-date and useful
- **I**ntegrated: Part of the development workflow
- **D**iscoverable: Easy to find and navigate
- **G**rounded: Based on actual code and decisions
- **E**volvable: Grows and adapts with the project

## Documentation Structure

### Architecture Decisions

Located in `architecture/decisions/adr/`, these documents capture significant
technical decisions using the ADR (Architecture Decision Record) format.

Current ADRs:
- [ADR-001: Why utf8dok](architecture/decisions/adr/ADR-001-why-utf8dok.md)
- [ADR-002: Rust for AsciiDoc](architecture/decisions/adr/ADR-002-rust-for-asciidoc.md)
- [ADR-003: Parser Technology](architecture/decisions/adr/ADR-003-parser-technology.md)

### Software Architecture

Located in `architecture/software/`, using C4 model diagrams defined in
Structurizr DSL format.

- [workspace.dsl](architecture/software/workspace.dsl) - System architecture

### Infrastructure

Located in `architecture/infrastructure/`, documenting CI/CD and deployment.

- [ci-cd.py](architecture/infrastructure/ci-cd.py) - CI/CD configuration generator

## Dogfooding

utf8dok will eventually process its own documentation, demonstrating:
1. AsciiDoc → HTML conversion for GitHub Pages
2. AsciiDoc → PDF for downloadable docs
3. Cross-reference validation
4. Documentation coverage metrics

## Contributing to Documentation

1. Follow the ADR template for new decisions
2. Keep documentation close to the code it describes
3. Update docs as part of feature PRs
4. Use AsciiDoc format for all documentation (coming soon)
