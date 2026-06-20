# Vendored Eclipse AsciiDoc ASG schema

`asg-schema.json` is the authoritative **Abstract Semantic Graph (ASG) JSON
Schema** (JSON Schema draft 2020-12) used to structurally validate the ASG our
adapter emits (`crates/utf8dok-core/tests/tck.rs`). See ADR-004.

- Upstream: https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang
  (`asg/schema.json`)
- License: Eclipse Public License 2.0 (EPL-2.0)
- Pinned commit: `d335f56572b656a7c9f84a5e0c76ea6f41f281e1`

Note this comes from the **asciidoc-lang** repository, distinct from the
**asciidoc-tck** repository that supplies the fixtures under `tests/tck/`.
