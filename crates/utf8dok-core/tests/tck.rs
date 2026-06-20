//! Eclipse AsciiDoc TCK conformance tests.
//!
//! Two layers of checking, data-driven over fixtures:
//!
//! 1. **Deep-equal** — parse each `*-input.adoc` via the ASG adapter and compare
//!    the emitted JSON against the expected `*-output.json`.
//! 2. **Schema validation** — validate every emitted block-mode (document) ASG
//!    against the authoritative ASG JSON Schema (`tests/schema/asg-schema.json`).
//!
//! Fixture roots:
//! - `tests/tck/` — the official Eclipse AsciiDoc TCK fixtures (EPL-2.0, vendored
//!   verbatim; see `tests/tck/ATTRIBUTION.md`).
//! - `tests/tck-local/` — utf8dok-authored, **non-official** fixtures for
//!   constructs the official suite does not yet cover (e.g. block-level inline
//!   markup); see `tests/tck-local/README.md`.
//!
//! This mirrors the official TCK harness contract: `block/` fixtures compare as a
//! full `document`; `inline/` fixtures compare as a bare array of inline nodes;
//! the input has a single trailing newline stripped before parsing.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::Value;
use utf8dok_core::asg;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// The official and local fixture roots.
fn fixture_roots() -> Vec<PathBuf> {
    let base = tests_dir();
    vec![base.join("tck"), base.join("tck-local")]
}

/// The compiled ASG schema validator, loaded once.
fn schema_validator() -> &'static jsonschema::Validator {
    static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let path = tests_dir().join("schema/asg-schema.json");
        let schema: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read ASG schema"))
                .expect("parse ASG schema");
        jsonschema::validator_for(&schema).expect("compile ASG schema")
    })
}

/// Recursively collect every `*-input.adoc` fixture under `dir`.
fn collect_inputs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_inputs(&path, out);
        } else if path.to_string_lossy().ends_with("-input.adoc") {
            out.push(path);
        }
    }
}

/// Strip exactly one trailing newline, matching the TCK test-loader default.
fn strip_trailing_newline(s: &str) -> String {
    s.strip_suffix('\n').unwrap_or(s).to_string()
}

fn is_inline(input_path: &Path) -> bool {
    input_path.components().any(|c| c.as_os_str() == "inline")
}

/// Parse `input` into the ASG JSON the TCK expects for this fixture's mode.
fn actual_asg(input_path: &Path, input: &str) -> Value {
    if is_inline(input_path) {
        serde_json::to_value(asg::parse_inlines(input)).unwrap()
    } else {
        serde_json::to_value(asg::parse_document(input)).unwrap()
    }
}

#[test]
fn tck_fixtures_match_expected_asg() {
    let roots = fixture_roots();
    let mut inputs = Vec::new();
    for root in &roots {
        collect_inputs(root, &mut inputs);
    }
    inputs.sort();

    assert!(!inputs.is_empty(), "no TCK fixtures found");

    let mut failures = Vec::new();
    let total = inputs.len();

    for input_path in &inputs {
        let label = label_for(input_path, &roots);
        let output_path = PathBuf::from(
            input_path
                .to_string_lossy()
                .replace("-input.adoc", "-output.json"),
        );

        let raw_input = fs::read_to_string(input_path).expect("read input fixture");
        let input = strip_trailing_newline(&raw_input);
        let expected: Value =
            serde_json::from_str(&fs::read_to_string(&output_path).expect("read expected output"))
                .expect("parse expected JSON");

        let actual = actual_asg(input_path, &input);

        if actual != expected {
            failures.push(format!(
                "  [deep-equal] {label}\n    expected: {expected}\n    actual:   {actual}"
            ));
            continue;
        }

        // Schema-validate block-mode (document) outputs against the authoritative
        // ASG schema. Inline-mode outputs are bare arrays the document schema does
        // not describe, so they are covered by deep-equal only.
        if !is_inline(input_path) {
            let errors: Vec<String> = schema_validator()
                .iter_errors(&actual)
                .map(|e| format!("{e} (at {})", e.instance_path))
                .collect();
            if !errors.is_empty() {
                failures.push(format!("  [schema] {label}\n    {}", errors.join("\n    ")));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "{}/{} TCK fixtures failed:\n{}",
            failures.len(),
            total,
            failures.join("\n")
        );
    }
}

/// A short label like `tck/block/paragraph/single-line` for diagnostics.
fn label_for(input_path: &Path, roots: &[PathBuf]) -> String {
    for root in roots {
        if let Ok(rel) = input_path.strip_prefix(root) {
            let root_name = root.file_name().unwrap().to_string_lossy();
            return format!("{root_name}/{}", rel.display());
        }
    }
    input_path.display().to_string()
}
