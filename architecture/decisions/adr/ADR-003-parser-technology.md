# ADR-003: Parser Technology Selection

## Status
Accepted

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
