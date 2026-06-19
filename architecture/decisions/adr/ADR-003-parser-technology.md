# ADR-003: Parser Technology Selection

## Status
**Superseded** (2026-06-19) — the pest decision below was never implemented.

## Amendment (2026-06-19): pest abandoned in favor of a hand-written state machine

The pest-based approach described in this ADR was **not implemented**. The
shipping parser in `crates/utf8dok-core/src/parser.rs` is a **hand-written
line-based state machine** (`ParserState` enum + `process_line`), with
regex-based inline parsing (`parse_inlines_with_attrs`). There is no
`grammar.pest` file and no pest API usage anywhere in the codebase; the `pest`
and `pest_derive` dependencies have been removed as dead.

### Why the change
- AsciiDoc's block structure is fundamentally line-oriented, which maps cleanly
  onto a per-line state machine and avoids the impedance mismatch of feeding a
  whole-document PEG grammar.
- Context-sensitive features (header vs. body, `ifdef`/`ifndef` preprocessing,
  `a|` AsciiDoc-in-cell, attribute substitution) are simpler to express as
  imperative state transitions than as PEG semantic actions.
- Targeting Eclipse AsciiDoc TCK compliance favors incremental, testable
  line-level handling over a monolithic grammar.

The original pest rationale is retained below for historical context.

---

## Context
AsciiDoc is a complex markup language with:
- Context-sensitive grammar
- Multiple block types with different parsing rules
- Inline formatting with nesting
- Macro expansion
- Include directives

Parser options considered:
1. **Hand-written recursive descent**: Maximum control, tedious
2. **pest (PEG)**: Declarative grammar, good error messages
3. **nom (parser combinators)**: Composable, steep learning curve
4. **tree-sitter**: Incremental parsing, complex setup
5. **LALRPOP**: LR parser generator, less flexible for context-sensitive grammars

## Decision
Use **pest** (Parsing Expression Grammar) as the primary parsing technology.

### Rationale
1. **Declarative Grammar**: Grammar defined in `.pest` files, separate from Rust code
2. **Good Error Messages**: Built-in support for meaningful parse errors
3. **Maintainability**: Grammar files are readable and maintainable
4. **Performance**: Compiles to efficient Rust code
5. **Flexibility**: PEG can handle context-sensitive constructs with semantic actions

## Implementation Strategy
1. Define core grammar in `grammar.pest`
2. Use pest's `#[derive(Parser)]` for code generation
3. Build AST from pest's parse tree in a separate pass
4. Handle context-sensitive features in semantic analysis

## Consequences

### Positive
- Clear separation between grammar and processing logic
- Excellent tooling for grammar development
- Good documentation and community support

### Negative
- PEG can have performance issues with highly ambiguous grammars
- Some AsciiDoc features may require workarounds
- Two-pass parsing (pest → AST) adds complexity

## References
- [pest documentation](https://pest.rs/)
- [Parsing Expression Grammars](https://en.wikipedia.org/wiki/Parsing_expression_grammar)
