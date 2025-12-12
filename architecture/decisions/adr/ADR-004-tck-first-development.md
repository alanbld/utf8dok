# ADR-004: TCK-First Development Strategy

## Status
Accepted

## Context
Multiple Rust AsciiDoc parsers exist but none are fully compliant with the emerging Eclipse AsciiDoc standard. The Eclipse Technology Compatibility Kit (TCK) provides a standard way to verify compliance.

## Decision
Develop utf8dok with TCK compliance as the primary goal:
1. Build TCK adapter mode first
2. Generate ASG JSON before HTML
3. Use TCK test suite as primary tests

## Consequences

### Positive
- Standards compliance from day one
- Clear success metrics (TCK pass rate)
- Contribute to ecosystem standardization
- Easier to maintain long-term

### Negative
- Slower initial feature development
- Must track evolving specification
- Less flexibility in design choices

## References
- [Eclipse AsciiDoc TCK](https://gitlab.eclipse.org/eclipse-wg/asciidoc)
- [ASG Schema](https://gitlab.eclipse.org/eclipse-wg/asciidoc/asciidoc-lang/-/blob/main/asg/schema.json)
- Related projects: asciidocr, asciidork
