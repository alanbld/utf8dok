//! Eclipse AsciiDoc TCK conformance tests.
//!
//! Data-driven against the vendored TCK fixtures under `tests/tck/` (see
//! `tests/tck/ATTRIBUTION.md`). For each `*-input.adoc` we parse the source via
//! the ASG adapter and deep-compare the emitted JSON against the expected
//! `*-output.json`.
//!
//! This mirrors the official TCK harness contract:
//! - fixtures under `block/` are compared as a full `document` ASG;
//! - fixtures under `inline/` are compared as a bare array of inline nodes;
//! - the input has a single trailing newline stripped before parsing (the TCK's
//!   default test-loader behaviour).
//!
//! Add coverage by vendoring more upstream pairs into `tests/tck/`; this test
//! discovers them automatically.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use utf8dok_core::asg;

fn tck_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/tck")
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

/// Parse `input` into the ASG JSON the TCK expects for this fixture's mode.
fn actual_asg(input_path: &Path, input: &str) -> Value {
    let is_inline = input_path.components().any(|c| c.as_os_str() == "inline");
    if is_inline {
        serde_json::to_value(asg::parse_inlines(input)).unwrap()
    } else {
        serde_json::to_value(asg::parse_document(input)).unwrap()
    }
}

#[test]
fn tck_fixtures_match_expected_asg() {
    let base = tck_dir();
    let mut inputs = Vec::new();
    collect_inputs(&base, &mut inputs);
    inputs.sort();

    assert!(
        !inputs.is_empty(),
        "no TCK fixtures found under {}",
        base.display()
    );

    let mut failures = Vec::new();
    let total = inputs.len();

    for input_path in &inputs {
        let rel = input_path.strip_prefix(&base).unwrap_or(input_path);
        let output_path = PathBuf::from(
            input_path
                .to_string_lossy()
                .replace("-input.adoc", "-output.json"),
        );

        let raw_input = fs::read_to_string(input_path).expect("read input fixture");
        let input = strip_trailing_newline(&raw_input);
        let expected_str = fs::read_to_string(&output_path).expect("read expected output");
        let expected: Value = serde_json::from_str(&expected_str).expect("parse expected JSON");

        let actual = actual_asg(input_path, &input);

        if actual != expected {
            failures.push(format!(
                "  {}\n    expected: {}\n    actual:   {}",
                rel.display(),
                expected,
                actual
            ));
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
