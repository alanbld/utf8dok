# Vendored Eclipse AsciiDoc TCK fixtures

The `*-input.adoc` / `*-output.json` files under this directory are a vendored
subset of the **Eclipse AsciiDoc TCK** test suite, used to drive utf8dok's ASG
adapter conformance tests (`crates/utf8dok-core/tests/tck.rs`). See ADR-004.

- Upstream: https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-tck
- License: Eclipse Public License 2.0 (EPL-2.0)
- Pinned commit: `3490153d3eb2ef5984497428b75364f49749dfc7`

Only the fixtures for currently-supported constructs are vendored. As the
adapter grows (sections, lists, headers, inline markup, …), copy the matching
upstream `tests/<path>-input.adoc` + `-output.json` pairs here, preserving the
`block/` and `inline/` top-level directories (the test harness uses that prefix
to choose block vs. inline comparison mode).

Currently vendored (trivial tier):
- `block/document/body-only`
- `block/paragraph/single-line`
- `block/paragraph/multiple-lines`
- `block/paragraph/sibling-paragraphs`
- `block/paragraph/paragraph-empty-lines-paragraph`
- `inline/no-markup/single-word`
