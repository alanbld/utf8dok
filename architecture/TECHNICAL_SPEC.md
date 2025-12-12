# Technical Specification for utf8dok

> **Note**: This document was originally created as "AsciiDoc-RS" before the project was renamed to utf8dok.
> It represents the initial technical design and architecture decisions.
> Some details may have evolved during implementation.

---

# AsciiDoc-RS: Rust-based AsciiDoc Interpreter
## Project Specification using BRIDGE Framework

---

## Executive Summary

A high-performance AsciiDoc parser and renderer written in Rust, using BRIDGE framework for its own documentation. This creates a perfect demonstration of BRIDGE while solving real performance and deployment challenges with current AsciiDoc implementations.

---

## Part 1: Technical Architecture

### System Context (C4 Level 1)

```
┌─────────────────────────────────────────────────────┐
│                    Users                             │
├───────────────┬────────────┬────────────────────────┤
│  Developers   │  CI/CD     │  Documentation Sites   │
└───────┬───────┴─────┬──────┴──────┬─────────────────┘
        │             │              │
        v             v              v
┌─────────────────────────────────────────────────────┐
│               asciidoc-rs                           │
│  ┌────────────────────────────────────────────┐    │
│  │  Parser → AST → Transformer → Renderer     │    │
│  └────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
        │             │              │
        v             v              v
┌───────────────┬────────────┬────────────────────────┐
│     HTML      │    PDF     │    DocBook/Other       │
└───────────────┴────────────┴────────────────────────┘
```

### Container Architecture (C4 Level 2)

```rust
// Core containers and their responsibilities

pub mod asciidoc_rs {
    pub mod parser {
        // Tokenizer: Text → Tokens
        // Parser: Tokens → AST
        // Uses: pest or nom
    }
    
    pub mod ast {
        // Document model
        // Visitor pattern for traversal
        // Immutable by default
    }
    
    pub mod transformer {
        // AST → AST transformations
        // Include processor
        // Macro expansion
    }
    
    pub mod renderer {
        // AST → Output format
        // Pluggable backends
        // HTML, PDF, DocBook
    }
    
    pub mod cli {
        // Command-line interface
        // File watching
        // Batch processing
    }
}
```

### Component Design (C4 Level 3)

```yaml
Parser Component:
  ├── Lexer
  │   ├── TokenStream
  │   ├── SourceMap
  │   └── ErrorReporter
  ├── Grammar
  │   ├── BlockRules
  │   ├── InlineRules
  │   └── TableRules
  └── ASTBuilder
      ├── NodeFactory
      ├── AttributeResolver
      └── ReferenceLinker

Renderer Component:
  ├── HTMLRenderer
  │   ├── TemplateEngine
  │   ├── SyntaxHighlighter
  │   └── DiagramRenderer
  ├── PDFRenderer (via wkhtmltopdf or pure Rust)
  └── ExtensionAPI
```

---

## Part 2: Implementation Specification (spec-kit style)

### Parser Specification

```rust
// Behavioral specification for LLM implementation

spec! {
    describe "AsciiDoc Parser" {
        context "Document Header" {
            it "parses single-line title" {
                input: "= Document Title"
                expected: Document { 
                    title: Some("Document Title"), 
                    level: 0 
                }
            }
            
            it "parses document attributes" {
                input: ":author: John Doe\n:version: 1.0"
                expected: Document {
                    attributes: HashMap::from([
                        ("author", "John Doe"),
                        ("version", "1.0")
                    ])
                }
            }
        }
        
        context "Include Directive" {
            it "processes file includes" {
                input: "include::other.adoc[leveloffset=+1]"
                behavior: {
                    - Read file "other.adoc"
                    - Parse content
                    - Apply leveloffset to headers
                    - Insert AST nodes at current position
                }
            }
        }
    }
}
```

### Renderer Specification

```rust
spec! {
    describe "HTML Renderer" {
        it "renders document with TOC" {
            given: Document with `:toc:` attribute
            when: render_html() called
            then: {
                - Generate table of contents
                - Inject TOC at specified position
                - Add anchor links to headers
                - Include navigation JavaScript
            }
        }
    }
}
```

---

## Part 3: BRIDGE Documentation Structure

### ADR-001: Why Rust for AsciiDoc

**Status**: Accepted

**Context**: Current AsciiDoc implementations have limitations:
- Ruby version: Slow startup, requires Ruby runtime
- JavaScript version: Variable performance, large bundle size
- No efficient native binary option

**Decision**: Implement AsciiDoc parser/renderer in Rust

**Consequences**:
- ✅ Single binary distribution
- ✅ 10-50x faster parsing
- ✅ Native and WASM targets
- ✅ Memory safety
- ⚠️ Larger implementation effort
- ⚠️ Need to maintain compatibility

### ADR-002: Parser Technology Choice

**Status**: Accepted

**Options Considered**:
1. **nom** - Parser combinators
2. **pest** - PEG grammar
3. **lalrpop** - LR(1) parser generator
4. Hand-written recursive descent

**Decision**: Use **pest** for grammar definition with nom for performance-critical sections

**Rationale**:
- Pest provides clear grammar definition
- Easier to maintain and understand
- Can optimize hot paths with nom later

### ADR-003: AST Design

**Status**: Accepted

**Decision**: Immutable AST with visitor pattern

```rust
pub enum Node {
    Document(Document),
    Section(Section),
    Block(Block),
    Inline(Inline),
}

pub trait Visitor {
    fn visit_document(&mut self, doc: &Document);
    fn visit_section(&mut self, section: &Section);
    // ...
}
```

---

## Part 4: Project Structure

```
asciidoc-rs/
├── architecture/           # BRIDGE documentation
│   ├── decisions/         
│   │   ├── adr/          # ADRs as shown above
│   │   └── principles.md 
│   ├── software/
│   │   ├── workspace.dsl  # C4 model in Structuriz
│   │   └── components/    # Detailed designs
│   ├── specifications/
│   │   └── parser.spec    # spec-kit behavioral specs
│   └── infrastructure/
│       └── ci-cd.py       # Diagrams.py for build pipeline
│
├── crates/
│   ├── asciidoc-parser/   # Core parsing logic
│   ├── asciidoc-ast/      # AST definitions
│   ├── asciidoc-render/   # Rendering backends
│   ├── asciidoc-cli/      # CLI application
│   └── asciidoc-wasm/     # WASM bindings
│
├── tests/
│   ├── fixtures/          # AsciiDoc test files
│   ├── compatibility/     # Asciidoctor compatibility tests
│   └── bridge/           # Dogfooding - BRIDGE docs as tests
│
├── docs/                  # Generated from architecture/
│   ├── index.html
│   └── decisions/
│
└── benches/              # Performance benchmarks
    └── parser.rs
```

---

## Part 5: Implementation Roadmap

### Phase 1: Core Parser (MVP)
- [ ] Basic grammar definition in pest
- [ ] Document structure parsing
- [ ] Paragraph and basic blocks
- [ ] Simple HTML renderer
- [ ] ADR-004: Document parsing architecture

**Deliverable**: Parse and render BRIDGE's README.adoc

### Phase 2: Full Block Support
- [ ] Tables with spans
- [ ] Lists (ordered, unordered, description)
- [ ] Code blocks with syntax highlighting
- [ ] Admonitions
- [ ] ADR-005: Table parsing strategy

**Deliverable**: Render complete BRIDGE specification

### Phase 3: Advanced Features
- [ ] Include directive
- [ ] Conditionals (ifdef)
- [ ] Variables and attributes
- [ ] Cross-references
- [ ] ADR-006: Include processing architecture

**Deliverable**: Full BRIDGE documentation with includes

### Phase 4: Diagram Integration
- [ ] PlantUML support
- [ ] Mermaid support
- [ ] GraphViz support
- [ ] ADR-007: Diagram rendering strategy

**Deliverable**: C4 diagrams in documentation

### Phase 5: Output Formats
- [ ] PDF renderer
- [ ] DocBook output
- [ ] WASM module
- [ ] ADR-008: PDF generation approach

---

## Part 6: Performance Targets

### Benchmarks Against Asciidoctor

| **Document** | **Asciidoctor Ruby** | **Target** | **Speedup** |
|-------------|---------------------|-----------|------------|
| Small (10KB) | 800ms | 10ms | 80x |
| Medium (100KB) | 2.5s | 50ms | 50x |
| Large (1MB) | 15s | 300ms | 50x |
| With includes | +200ms/include | +5ms/include | 40x |

### Memory Usage

```rust
// Target memory characteristics
- Streaming parser for large documents
- Lazy include processing
- Incremental rendering
- Max memory: 2x document size
```

---

## Part 7: Dogfooding Strategy

### Using BRIDGE to Document Itself

1. **All architecture decisions as ADRs** in AsciiDoc
2. **C4 models** embedded in AsciiDoc using PlantUML
3. **Parser specifications** in AsciiDoc with spec-kit
4. **Generated documentation** from AsciiDoc source

### Test Suite = Documentation

```rust
#[test]
fn test_parse_bridge_readme() {
    let input = include_str!("../architecture/README.adoc");
    let doc = parse_asciidoc(input).unwrap();
    
    // The test is that our own docs parse correctly
    assert!(doc.validate().is_ok());
    
    // Render and verify
    let html = render_html(&doc);
    assert!(html.contains("BRIDGE Framework"));
}
```

### CI/CD Pipeline (Diagrams.py)

```python
from diagrams import Diagram, Cluster
from diagrams.generic.vcs import Github
from diagrams.generic.ci import GithubActions
from diagrams.onprem.client import Users

with Diagram("AsciiDoc-RS CI/CD", show=False):
    source = Github("asciidoc-rs")
    
    with Cluster("Build"):
        ci = GithubActions("Test & Build")
        
    with Cluster("Documentation"):
        bridge = GithubActions("Generate BRIDGE Docs")
        
    with Cluster("Artifacts"):
        binary = Storage("Binaries")
        docs = Storage("Documentation Site")
        
    source >> ci >> [binary, bridge]
    bridge >> docs >> Users("Developers")
```

---

## Part 8: Success Metrics

### Technical Metrics
- Parse speed: >50x faster than Asciidoctor
- Binary size: <10MB
- WASM size: <2MB
- Memory usage: <2x document size
- Compatibility: >95% Asciidoctor features

### BRIDGE Documentation Metrics
- All ADRs in AsciiDoc
- 100% of architecture documented
- Documentation builds in <1s
- Zero external dependencies for viewing docs

### Community Metrics
- Contributors using BRIDGE
- Projects adopting asciidoc-rs
- Documentation feedback loop time

---

## Part 9: Risk Analysis

### Technical Risks

| **Risk** | **Probability** | **Impact** | **Mitigation** |
|---------|---------------|-----------|---------------|
| Grammar complexity | High | High | Start with subset, iterate |
| Performance goals unmet | Medium | Medium | Profile early, optimize hot paths |
| Compatibility issues | High | Medium | Extensive test suite |
| WASM bundle too large | Medium | Low | Tree-shaking, lazy loading |

### Documentation Risks

| **Risk** | **Impact** | **Mitigation** |
|---------|-----------|---------------|
| BRIDGE too complex | High | Start with ADRs only |
| Circular dependency | Medium | Bootstrap with Markdown |
| Documentation drift | High | Tests use doc files |

---

## Part 10: Example AsciiDoc for BRIDGE

### Sample ADR in AsciiDoc

```asciidoc
= ADR-009: AsciiDoc as Primary Documentation Format
:status: Proposed
:date: 2024-01-15
:deciders: Architecture Team
:consulted: Development Team, Documentation Team
:informed: All Stakeholders

== Status
{status}

== Context

We need a documentation format that:

* Supports includes for modular documentation
* Generates multiple output formats
* Integrates with our C4 diagrams
* Provides better structure than Markdown

.Current State
[cols="1,2,1"]
|===
| Format | Usage | Problems

| Markdown
| README files, basic docs
| No includes, limited tables

| Confluence
| Design documents
| Not version controlled

| Word
| Formal documents
| Binary format, no diff
|===

== Decision

Adopt AsciiDoc as the primary documentation format.

[plantuml, adr-009-decision, svg]
----
@startuml
!include C4_Container.puml

Person(dev, "Developer")
System(asciidoc, "AsciiDoc")
System_Ext(outputs, "Multiple Formats")

dev -> asciidoc : Writes
asciidoc -> outputs : Generates
@enduml
----

== Consequences

.Positive
* [✓] Single source, multiple outputs
* [✓] Include directive for modular docs
* [✓] Native diagram support
* [✓] Better tables and cross-references

.Negative
* [✗] Learning curve for team
* [✗] Need to build/select processor
* [✗] Migration effort from Markdown

== Compliance

To verify compliance:

[source,bash]
----
# All ADRs should be in AsciiDoc
find architecture/decisions -name "*.adoc" | wc -l

# Should render without errors
asciidoc-rs architecture/decisions/*.adoc
----

== Related Decisions

* <<ADR-001,ADR-001: Rust Implementation>> - Influences processor choice
* <<ADR-010,ADR-010: Documentation as Code>> - Requires text format
```

---

## Conclusion

Building asciidoc-rs with BRIDGE documentation creates:

1. **A valuable tool** - Fast, native AsciiDoc processing
2. **A perfect demonstration** - BRIDGE framework in practice
3. **Dogfooding excellence** - Tool documents itself
4. **Community contribution** - Both tool and framework

The project is technically feasible and would provide significant value to both the Rust and documentation communities while perfectly demonstrating BRIDGE principles.

**Next Steps**:
1. Create repository with BRIDGE structure
2. Implement minimal parser (ADRs + basic blocks)
3. Document first ADR about the project itself
4. Build incrementally, documenting each decision

This creates a virtuous cycle where improving the tool improves its own documentation.