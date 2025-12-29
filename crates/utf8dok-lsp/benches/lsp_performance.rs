//! LSP Performance Baseline Benchmarks
//!
//! MVP Performance Thresholds:
//! - Validate 50 documents: <100ms
//! - Validate 250 documents: <250ms
//! - Cold LSP start: <500ms
//! - Memory usage: <100MB for 250 documents

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use utf8dok_lsp::compliance::ComplianceEngine;
use utf8dok_lsp::config::Settings;
use utf8dok_lsp::domain::plugins::{DiagramPlugin, QualityPlugin};
use utf8dok_lsp::workspace::WorkspaceGraph;

/// Create a realistic test workspace with the specified number of documents
fn create_test_workspace(document_count: usize) -> WorkspaceGraph {
    let mut graph = WorkspaceGraph::new();

    // Create an index document
    let index_content = generate_index_document(document_count);
    graph.add_document("file:///workspace/index.adoc", &index_content);

    // Create individual ADR documents
    for i in 1..=document_count {
        let uri = format!("file:///workspace/adr/ADR-{:04}.adoc", i);
        let content = generate_adr_document(i, document_count);
        graph.add_document(&uri, &content);
    }

    graph
}

/// Generate an index document that links to all ADRs
fn generate_index_document(count: usize) -> String {
    let mut content = String::from(
        r#"= Architecture Decision Records
:doctype: book
:toc: left

== Overview

This document indexes all Architecture Decision Records.

== ADR Index

"#,
    );

    for i in 1..=count {
        content.push_str(&format!("* <<adr/ADR-{:04}.adoc#,ADR-{:04}>>\n", i, i));
    }

    content
}

/// Generate a realistic ADR document
fn generate_adr_document(number: usize, total: usize) -> String {
    let status = if number < total / 2 {
        "Accepted"
    } else if number < total * 3 / 4 {
        "Proposed"
    } else {
        "Superseded"
    };

    let supersedes = if number > 10 && number % 5 == 0 {
        format!(":supersedes: ADR-{:04}", number - 5)
    } else {
        String::new()
    };

    format!(
        r#"= ADR-{number:04}: Sample Architecture Decision
:status: {status}
{supersedes}

== Context

This is a sample architecture decision record for benchmarking purposes.
The decision was made to address performance requirements in the system.

Obviously, this document contains some weasel words that should be detected.
The implementation was completed by the development team.

== Decision

We will implement the following architecture:

[mermaid]
----
graph TD
    A[Client] --> B[API Gateway]
    B --> C[Service A]
    B --> D[Service B]
    C --> E[Database]
    D --> E
----

The system clearly needs this architecture for scalability.

== Consequences

* Positive: Better performance
* Negative: Increased complexity
* Neutral: More documentation needed

== References

* link:ADR-{prev:04}.adoc[Previous ADR]
* link:../index.adoc[Back to Index]
"#,
        number = number,
        status = status,
        supersedes = supersedes,
        prev = if number > 1 { number - 1 } else { 1 }
    )
}

/// Benchmark compliance engine validation
fn bench_compliance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("compliance_validation");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    // MVP Threshold: 50 documents < 100ms
    group.bench_with_input(BenchmarkId::new("documents", 50), &50, |b, &count| {
        let graph = create_test_workspace(count);
        let engine = ComplianceEngine::new();
        b.iter(|| {
            let result = engine.run(black_box(&graph));
            black_box(result)
        });
    });

    // MVP Threshold: 250 documents < 250ms
    group.bench_with_input(BenchmarkId::new("documents", 250), &250, |b, &count| {
        let graph = create_test_workspace(count);
        let engine = ComplianceEngine::new();
        b.iter(|| {
            let result = engine.run(black_box(&graph));
            black_box(result)
        });
    });

    // Stress test: 500 documents
    group.bench_with_input(BenchmarkId::new("documents", 500), &500, |b, &count| {
        let graph = create_test_workspace(count);
        let engine = ComplianceEngine::new();
        b.iter(|| {
            let result = engine.run(black_box(&graph));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark workspace graph operations
fn bench_workspace_graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("workspace_graph");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    // Document addition (incremental updates)
    group.bench_function("add_document", |b| {
        let mut graph = WorkspaceGraph::new();
        let content = generate_adr_document(1, 100);
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let uri = format!("file:///workspace/doc_{}.adoc", counter);
            graph.add_document(&uri, black_box(&content));
        });
    });

    // Symbol search (workspace symbols)
    group.bench_function("symbol_search_250_docs", |b| {
        let graph = create_test_workspace(250);
        b.iter(|| {
            let results = graph.query_symbols(black_box("ADR"));
            black_box(results)
        });
    });

    // Cross-reference resolution
    group.bench_function("resolve_references_250_docs", |b| {
        let graph = create_test_workspace(250);
        b.iter(|| {
            let refs = graph.get_document_refs(black_box("file:///workspace/index.adoc"));
            black_box(refs)
        });
    });

    group.finish();
}

/// Benchmark content plugins
fn bench_content_plugins(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_plugins");
    group.warm_up_time(Duration::from_millis(200));
    group.measurement_time(Duration::from_secs(2));

    let sample_document = generate_adr_document(42, 100);

    // Quality plugin (weasel words, passive voice, readability)
    group.bench_function("quality_validation", |b| {
        let plugin = QualityPlugin::new();
        b.iter(|| {
            let diagnostics = plugin.validate_writing_quality(black_box(&sample_document));
            black_box(diagnostics)
        });
    });

    // Diagram plugin (Mermaid/PlantUML validation)
    group.bench_function("diagram_validation", |b| {
        let plugin = DiagramPlugin::new();
        b.iter(|| {
            let diagnostics = plugin.validate_diagrams(black_box(&sample_document));
            black_box(diagnostics)
        });
    });

    // Combined quality analysis
    group.bench_function("quality_summary", |b| {
        let plugin = QualityPlugin::new();
        b.iter(|| {
            let summary = plugin.quality_summary(black_box(&sample_document));
            black_box(summary)
        });
    });

    group.finish();
}

/// Benchmark settings and configuration
fn bench_configuration(c: &mut Criterion) {
    let mut group = c.benchmark_group("configuration");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));

    let sample_toml = r#"
[compliance.bridge]
orphans = "warning"
superseded_status = "error"

[plugins]
api_docs = false
writing_quality = true
diagrams = true
custom_weasel_words = ["clearly", "obviously", "basically"]

[workspace]
root = "/docs"
entry_points = ["index.adoc", "README.adoc"]
"#;

    // Settings parsing
    group.bench_function("parse_settings", |b| {
        b.iter(|| {
            let settings = Settings::from_toml_str(black_box(sample_toml)).unwrap();
            black_box(settings)
        });
    });

    // Engine creation with settings
    group.bench_function("engine_with_settings", |b| {
        let settings = Settings::from_toml_str(sample_toml).unwrap();
        b.iter(|| {
            let engine = ComplianceEngine::with_settings(black_box(&settings));
            black_box(engine)
        });
    });

    group.finish();
}

/// Benchmark cold start (LSP initialization simulation)
fn bench_cold_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_start");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(50); // Fewer samples for expensive operations

    // MVP Threshold: Cold start < 500ms
    group.bench_function("initialize_all_components", |b| {
        b.iter(|| {
            // Simulate full LSP initialization
            let settings = Settings::default();
            let compliance_engine = ComplianceEngine::with_settings(&settings);
            let quality_plugin = QualityPlugin::with_settings(&settings);
            let diagram_plugin = DiagramPlugin::with_settings(&settings);
            let workspace_graph = WorkspaceGraph::new();

            black_box((
                settings,
                compliance_engine,
                quality_plugin,
                diagram_plugin,
                workspace_graph,
            ))
        });
    });

    // Workspace initialization with 50 documents
    group.bench_function("initialize_workspace_50_docs", |b| {
        b.iter(|| {
            let graph = create_test_workspace(50);
            let engine = ComplianceEngine::new();
            let result = engine.run(&graph);
            black_box((graph, result))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_compliance_validation,
    bench_workspace_graph,
    bench_content_plugins,
    bench_configuration,
    bench_cold_start
);
criterion_main!(benches);
