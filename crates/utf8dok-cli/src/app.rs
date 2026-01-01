//! CLI Application logic
//!
//! Contains the command-line interface implementation.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use glob::glob;

use utf8dok_core::diagnostics::Diagnostic;
use utf8dok_core::dual_nature::{
    parse_dual_nature, transform_for_format, validate_dual_nature, ContentSelector,
    OutputFormat as DualNatureFormat,
};
use utf8dok_core::parse;
use utf8dok_lsp::compliance::dashboard::ComplianceDashboard;
use utf8dok_lsp::compliance::ComplianceEngine;
use utf8dok_lsp::config::Settings;
use utf8dok_lsp::workspace::graph::WorkspaceGraph;
use utf8dok_ooxml::{
    AsciiDocExtractor, DocxWriter, OoxmlArchive, SourceOrigin, StyleSheet, Template,
};
use utf8dok_plugins::PluginEngine;
use utf8dok_pptx::{PotxTemplate, PptxWriter, SlideExtractor};
use utf8dok_validate::ValidationEngine;

/// Output format for diagnostics
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for LLM/tool consumption
    Json,
}

/// Output format for audit reports
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum AuditFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for CI/CD integration
    Json,
    /// Markdown output for PR comments
    Markdown,
}

/// Target format for dual-nature analysis
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum DualNatureTargetFormat {
    /// Show content for slide/presentation view
    #[default]
    Slide,
    /// Show content for document view
    Document,
    /// Show both views side by side
    Both,
}

/// Output format for render command
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum RenderFormat {
    /// Microsoft Word document (.docx)
    #[default]
    Docx,
    /// PowerPoint presentation (.pptx)
    Pptx,
}

#[derive(Parser)]
#[command(name = "utf8dok")]
#[command(author, version, about = "Plain text, powerful docs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Template type for init command
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum InitTemplate {
    /// Bridge Framework: ADR-focused documentation workspace
    #[default]
    Bridge,
    /// Basic: Simple AsciiDoc project structure
    Basic,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new documentation workspace
    Init {
        /// Path for the new workspace
        path: PathBuf,

        /// Template to use (bridge or basic)
        #[arg(short, long, value_enum, default_value = "bridge")]
        template: InitTemplate,
    },

    /// Extract AsciiDoc and template from a DOCX file
    Extract {
        /// Input DOCX file
        input: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "output")]
        output: PathBuf,

        /// Force parsing document.xml even if embedded source exists
        #[arg(long)]
        force_parse: bool,
    },

    /// Render AsciiDoc to DOCX or PPTX
    Render {
        /// Input AsciiDoc file
        input: PathBuf,

        /// Output file (default: input with .docx or .pptx extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format: docx or pptx
        #[arg(short, long, value_enum, default_value = "docx")]
        format: RenderFormat,

        /// Template file (DOTX for DOCX, POTX for PPTX)
        #[arg(short, long)]
        template: Option<PathBuf>,

        /// Cover image file (PNG, JPG) for title page (DOCX only)
        #[arg(long)]
        cover: Option<PathBuf>,
    },

    /// Check an AsciiDoc file for issues (validation)
    Check {
        /// Input AsciiDoc file
        input: PathBuf,

        /// Output format (text or json)
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Rhai plugin script(s) for custom validation rules
        #[arg(short, long)]
        plugin: Vec<PathBuf>,
    },

    /// Audit a documentation workspace for compliance (CI/CD)
    Audit {
        /// Input directory containing AsciiDoc files
        #[arg(default_value = ".")]
        input: PathBuf,

        /// Output format (text, json, or markdown)
        #[arg(short, long, value_enum, default_value = "text")]
        format: AuditFormat,

        /// Strict mode: exit with error code if any violations found
        #[arg(long)]
        strict: bool,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Generate a compliance dashboard HTML report
    Dashboard {
        /// Input directory containing AsciiDoc files
        #[arg(default_value = ".")]
        input: PathBuf,

        /// Output file path
        #[arg(short, long, default_value = "compliance-dashboard.html")]
        output: PathBuf,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Analyze dual-nature document (slide + document from single source)
    DualNature {
        /// Input AsciiDoc file with dual-nature annotations
        input: PathBuf,

        /// Target format to analyze (slide, document, or both)
        #[arg(short, long, value_enum, default_value = "both")]
        target: DualNatureTargetFormat,

        /// Output format (text or json)
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Only validate without showing content
        #[arg(long)]
        validate_only: bool,
    },
}

/// Run the CLI application
///
/// This is the main entry point for the command-line interface.
/// It parses arguments and dispatches to the appropriate command.
pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path, template } => {
            init_command(&path, template)?;
        }
        Commands::Extract {
            input,
            output,
            force_parse,
        } => {
            extract_command(&input, &output, force_parse)?;
        }
        Commands::Render {
            input,
            output,
            format,
            template,
            cover,
        } => {
            render_command(&input, output.as_deref(), format, template.as_deref(), cover.as_deref())?;
        }
        Commands::Check {
            input,
            format,
            plugin,
        } => {
            check_command(&input, format, &plugin)?;
        }
        Commands::Audit {
            input,
            format,
            strict,
            config,
        } => {
            audit_command(&input, format, strict, config.as_deref())?;
        }
        Commands::Dashboard {
            input,
            output,
            config,
        } => {
            dashboard_command(&input, &output, config.as_deref())?;
        }
        Commands::DualNature {
            input,
            target,
            format,
            validate_only,
        } => {
            dual_nature_command(&input, target, format, validate_only)?;
        }
    }

    Ok(())
}

// ==================== EMBEDDED TEMPLATES ====================

/// Default utf8dok.toml for Bridge template
const BRIDGE_CONFIG: &str = r#"# UTF8DOK Configuration
# Generated by: utf8dok init --template bridge

[workspace]
root = "."
entry_points = ["index.adoc", "README.adoc"]

[compliance.bridge]
# ADR compliance settings
orphans = "error"           # Error if documents are not reachable from entry points
superseded_status = "error" # Superseded ADRs must have correct status
missing_status = "warning"  # Warn if ADR is missing :status: attribute
missing_date = "warning"    # Warn if ADR is missing :date: attribute

[plugins]
# Enable built-in plugins
writing_quality = true
diagrams = true

# Optional: Custom weasel words to flag
# custom_weasel_words = ["obviously", "clearly", "simply"]
"#;

/// Default utf8dok.toml for Basic template
const BASIC_CONFIG: &str = r#"# UTF8DOK Configuration
# Generated by: utf8dok init --template basic

[workspace]
root = "."
entry_points = ["index.adoc", "README.adoc"]

[plugins]
writing_quality = true
diagrams = true
"#;

/// Default index.adoc for Bridge template
const BRIDGE_INDEX: &str = r#"= Documentation Hub
:toc: left
:toclevels: 3

== Overview

Welcome to your documentation workspace!

This project uses the *Bridge Framework* for architecture decision records (ADRs).

== Quick Start

. Run `utf8dok audit` to check documentation compliance
. Run `utf8dok dashboard` to generate an HTML report
. Edit ADRs in the `adr/` directory

== Documentation Index

=== Architecture Decision Records

* <<adr/0001-record-architecture-decisions.adoc#,ADR 0001: Record Architecture Decisions>>

== Compliance Status

NOTE: This documentation is validated by UTF8DOK. Run `utf8dok audit` to check compliance.
"#;

/// Default index.adoc for Basic template
const BASIC_INDEX: &str = r#"= Documentation
:toc: left

== Overview

Welcome to your documentation workspace!

== Getting Started

Edit this file to add your content.

== Sections

=== Section One

Add content here.

=== Section Two

Add more content here.
"#;

/// Sample ADR for Bridge template
const BRIDGE_ADR_0001: &str = r#"[[adr-0001]]
= ADR 0001: Record Architecture Decisions
:status: Accepted
:date: {date}

== Context

We need to record the architectural decisions made on this project.

== Decision

We will use Architecture Decision Records (ADRs) as described by Michael Nygard.
Each ADR will include:

* A unique identifier (ADR-XXXX)
* A descriptive title
* Status (:status: attribute)
* Date (:date: attribute)
* Context, Decision, and Consequences sections

== Consequences

*Positive*:

* Decisions are documented and discoverable
* New team members can understand past decisions
* We can track the evolution of our architecture

*Negative*:

* Requires discipline to maintain
* ADRs need to be updated as context changes
"#;

/// Execute the init command - scaffold a new documentation workspace
pub fn init_command(path: &PathBuf, template: InitTemplate) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Initializing: {}", path.display());

    // Check if directory already exists and is not empty
    if path.exists() {
        let has_files = fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        if has_files {
            anyhow::bail!(
                "Directory '{}' already exists and is not empty.\n\
                 Use an empty directory or a new path.",
                path.display()
            );
        }
    }

    // Create the directory structure
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;

    match template {
        InitTemplate::Bridge => init_bridge_template(path)?,
        InitTemplate::Basic => init_basic_template(path)?,
    }

    println!();
    println!("✓ Workspace initialized successfully!");
    println!();
    println!("Next steps:");
    println!("  cd {}", path.display());
    println!("  utf8dok audit           # Check compliance");
    println!("  utf8dok dashboard       # Generate HTML report");

    Ok(())
}

/// Initialize a Bridge Framework workspace
fn init_bridge_template(path: &std::path::Path) -> Result<()> {
    // Create adr/ directory
    let adr_dir = path.join("adr");
    fs::create_dir_all(&adr_dir)
        .with_context(|| format!("Failed to create adr directory: {}", adr_dir.display()))?;

    // Write utf8dok.toml
    let config_path = path.join("utf8dok.toml");
    fs::write(&config_path, BRIDGE_CONFIG)
        .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
    println!("  Created: {}", config_path.display());

    // Write index.adoc
    let index_path = path.join("index.adoc");
    fs::write(&index_path, BRIDGE_INDEX)
        .with_context(|| format!("Failed to write index: {}", index_path.display()))?;
    println!("  Created: {}", index_path.display());

    // Write sample ADR with current date
    let adr_path = adr_dir.join("0001-record-architecture-decisions.adoc");
    let today = chrono_lite_date();
    let adr_content = BRIDGE_ADR_0001.replace("{date}", &today);
    fs::write(&adr_path, adr_content)
        .with_context(|| format!("Failed to write ADR: {}", adr_path.display()))?;
    println!("  Created: {}", adr_path.display());

    println!();
    println!("Template: Bridge Framework (ADR-focused)");

    Ok(())
}

/// Initialize a Basic workspace
fn init_basic_template(path: &std::path::Path) -> Result<()> {
    // Write utf8dok.toml
    let config_path = path.join("utf8dok.toml");
    fs::write(&config_path, BASIC_CONFIG)
        .with_context(|| format!("Failed to write config: {}", config_path.display()))?;
    println!("  Created: {}", config_path.display());

    // Write index.adoc
    let index_path = path.join("index.adoc");
    fs::write(&index_path, BASIC_INDEX)
        .with_context(|| format!("Failed to write index: {}", index_path.display()))?;
    println!("  Created: {}", index_path.display());

    println!();
    println!("Template: Basic (simple AsciiDoc project)");

    Ok(())
}

/// Get current date in YYYY-MM-DD format (lightweight, no chrono dependency)
fn chrono_lite_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Simple date calculation (not perfect but good enough)
    let days_since_epoch = secs / 86400;
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days in days_in_months {
        if remaining_days < days {
            break;
        }
        remaining_days -= days;
        month += 1;
    }

    let day = remaining_days + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Execute the extract command
pub fn extract_command(input: &PathBuf, output_dir: &PathBuf, force_parse: bool) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Extracting: {}", input.display());

    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    // Open the DOCX archive
    let archive = OoxmlArchive::open(input)
        .with_context(|| format!("Failed to open DOCX file: {}", input.display()))?;

    // Use extractor with embedded source priority
    let extractor = AsciiDocExtractor::new().with_force_parse(force_parse);
    let extracted = extractor
        .extract_archive(&archive)
        .with_context(|| format!("Failed to extract document: {}", input.display()))?;

    // Report source origin
    match extracted.source_origin {
        SourceOrigin::Embedded => {
            println!("  Source: embedded utf8dok/source.adoc (round-trip document)");
        }
        SourceOrigin::Parsed => {
            println!("  Source: parsed from document.xml");
        }
    }

    let asciidoc = &extracted.asciidoc;

    // Parse styles for config generation
    let styles_xml = archive
        .styles_xml()
        .context("Failed to read styles.xml from archive")?;
    let styles = StyleSheet::parse(styles_xml).context("Failed to parse styles")?;

    // Create output directory
    fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            output_dir.display()
        )
    })?;

    // Write AsciiDoc file
    let adoc_path = output_dir.join("document.adoc");
    fs::write(&adoc_path, asciidoc)
        .with_context(|| format!("Failed to write AsciiDoc file: {}", adoc_path.display()))?;
    println!("  Created: {}", adoc_path.display());

    // Copy input as template (simple copy for now)
    let template_path = output_dir.join("template.dotx");
    fs::copy(input, &template_path)
        .with_context(|| format!("Failed to copy template: {}", template_path.display()))?;
    println!("  Created: {}", template_path.display());

    // Extract media files (images)
    let media_files: Vec<String> = archive
        .file_list()
        .filter(|f| f.starts_with("word/media/"))
        .map(|s| s.to_string())
        .collect();

    if !media_files.is_empty() {
        let media_dir = output_dir.join("media");
        fs::create_dir_all(&media_dir).with_context(|| {
            format!("Failed to create media directory: {}", media_dir.display())
        })?;

        let mut copied = 0;
        for media_file in &media_files {
            if let Some(data) = archive.get(media_file) {
                let filename = media_file.strip_prefix("word/media/").unwrap_or(media_file);
                let dest_path = media_dir.join(filename);
                if fs::write(&dest_path, data).is_ok() {
                    copied += 1;
                }
            }
        }
        if copied > 0 {
            println!(
                "  Copied: {} media files to {}",
                copied,
                media_dir.display()
            );
        }
    }

    // Generate style mappings TOML
    let toml_path = output_dir.join("utf8dok.toml");
    let toml_content = generate_config_toml(&styles, input);
    fs::write(&toml_path, toml_content)
        .with_context(|| format!("Failed to write config file: {}", toml_path.display()))?;
    println!("  Created: {}", toml_path.display());

    println!();
    println!("Extraction complete!");
    // Count non-empty lines as a rough indicator
    let line_count = asciidoc.lines().filter(|l| !l.trim().is_empty()).count();
    println!("  {} content lines", line_count);

    Ok(())
}

/// Generate configuration TOML from styles
fn generate_config_toml(styles: &StyleSheet, input: &std::path::Path) -> String {
    let mut output = String::new();

    output.push_str("# utf8dok configuration\n");
    output.push_str("# Generated from: ");
    output.push_str(&input.display().to_string());
    output.push_str("\n\n");

    output.push_str("[template]\n");
    output.push_str("path = \"template.dotx\"\n\n");

    output.push_str("[styles]\n");

    // Add heading styles
    for style in styles.heading_styles() {
        if let Some(level) = style.outline_level {
            output.push_str(&format!("heading{} = \"{}\"\n", level + 1, style.id));
        }
    }

    // Add default paragraph style
    if let Some(ref para) = styles.default_paragraph {
        output.push_str(&format!("paragraph = \"{}\"\n", para));
    }

    // Add table styles
    let table_styles: Vec<_> = styles.table_styles().collect();
    if !table_styles.is_empty() {
        output.push_str(&format!("table = \"{}\"\n", table_styles[0].id));
    }

    output
}

/// Execute the render command
pub fn render_command(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    format: RenderFormat,
    template: Option<&std::path::Path>,
    cover: Option<&std::path::Path>,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Rendering: {}", input.display());

    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    match format {
        RenderFormat::Docx => render_docx(input, output, template, cover),
        RenderFormat::Pptx => render_pptx(input, output, template),
    }
}

/// Render AsciiDoc to DOCX
fn render_docx(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    template: Option<&std::path::Path>,
    cover: Option<&std::path::Path>,
) -> Result<()> {
    println!("  Format: DOCX");

    // Determine output path (default: input with .docx extension)
    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => input.with_extension("docx"),
    };

    // Determine template path (default: template.dotx in current directory)
    let template_path = match template {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from("template.dotx"),
    };

    // Check template exists
    if !template_path.exists() {
        anyhow::bail!(
            "Template file not found: {}\n\
             \n\
             To create a template, you can:\n\
             1. Use 'utf8dok extract' on an existing DOCX to generate a template\n\
             2. Create a new Word document and save it as .dotx\n\
             3. Specify a different template with --template <path>",
            template_path.display()
        );
    }

    // Step 1: Read input AsciiDoc file
    println!("  Reading: {}", input.display());
    let source_content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // Step 2: Parse AsciiDoc to AST
    println!("  Parsing AsciiDoc...");
    let ast = parse(&source_content).context("Failed to parse AsciiDoc content")?;
    println!("    {} blocks parsed", ast.blocks.len());

    // Step 3: Load template using Template API
    println!("  Loading template: {}", template_path.display());
    let template_obj = Template::load(&template_path)
        .with_context(|| format!("Failed to load template: {}", template_path.display()))?;

    // Step 4: Load or generate config
    let config_path = input
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("utf8dok.toml");
    let config_content = if config_path.exists() {
        println!("  Loading config: {}", config_path.display());
        fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config: {}", config_path.display()))?
    } else {
        // Generate minimal config
        format!(
            "# utf8dok configuration\n\
             # Auto-generated during render\n\n\
             [template]\n\
             path = \"{}\"\n",
            template_path.display()
        )
    };

    // Step 5: Create writer with embedded content for self-contained DOCX
    println!("  Generating self-contained DOCX...");
    let mut writer = DocxWriter::new();
    writer.set_source(&source_content);
    writer.set_config(&config_content);

    // Step 5b: Add cover image if specified
    if let Some(cover_path) = cover {
        if cover_path.exists() {
            println!("  Adding cover image: {}", cover_path.display());
            let cover_bytes = fs::read(cover_path)
                .with_context(|| format!("Failed to read cover image: {}", cover_path.display()))?;
            let cover_filename = cover_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "cover.png".to_string());
            writer.set_cover_image(cover_filename, cover_bytes);
        } else {
            eprintln!("  Warning: Cover image not found: {}", cover_path.display());
        }
    }

    // Step 6: Generate DOCX
    let docx_bytes = writer
        .generate_with_template(&ast, template_obj)
        .context("Failed to generate DOCX from AST")?;

    // Step 7: Write output
    println!("  Writing: {}", output_path.display());
    fs::write(&output_path, &docx_bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!();
    println!("Render complete!");
    println!("  Output: {}", output_path.display());
    println!("  Size: {} bytes", docx_bytes.len());
    println!("  Self-contained: yes (source + config embedded)");

    Ok(())
}

/// Render AsciiDoc to PPTX
fn render_pptx(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    template: Option<&std::path::Path>,
) -> Result<()> {
    println!("  Format: PPTX");

    // Determine output path (default: input with .pptx extension)
    let output_path = match output {
        Some(p) => p.to_path_buf(),
        None => input.with_extension("pptx"),
    };

    // Step 1: Read input AsciiDoc file
    println!("  Reading: {}", input.display());
    let source_content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // Step 2: Parse AsciiDoc to AST
    println!("  Parsing AsciiDoc...");
    let ast = parse(&source_content).context("Failed to parse AsciiDoc content")?;
    println!("    {} blocks parsed", ast.blocks.len());

    // Step 3: Extract slides from AST using SlideExtractor
    println!("  Extracting slides...");
    let deck = SlideExtractor::extract(&ast);
    println!("    {} slides extracted", deck.slides.len());

    // Step 4: Create PPTX writer with title from deck
    let mut writer = if let Some(ref title) = deck.title {
        PptxWriter::default().with_title(title)
    } else {
        PptxWriter::default()
    };

    // Step 5: Load template if specified
    if let Some(template_path) = template {
        if template_path.exists() {
            println!("  Loading template: {}", template_path.display());
            let potx = PotxTemplate::from_file(template_path)
                .with_context(|| format!("Failed to parse template: {}", template_path.display()))?;
            writer = writer.with_template(potx);
        } else {
            eprintln!("  Warning: Template not found: {}", template_path.display());
            eprintln!("  Continuing without template...");
        }
    }

    // Step 6: Add slides from deck to writer
    writer.add_slides(deck.slides.clone());

    // Step 7: Generate PPTX
    println!("  Generating PPTX...");
    let pptx_bytes = writer
        .generate()
        .context("Failed to generate PPTX from slides")?;

    // Step 8: Write output
    println!("  Writing: {}", output_path.display());
    fs::write(&output_path, &pptx_bytes)
        .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

    println!();
    println!("Render complete!");
    println!("  Output: {}", output_path.display());
    println!("  Size: {} bytes", pptx_bytes.len());
    println!("  Slides: {}", deck.slides.len());

    Ok(())
}

/// Execute the check command
pub fn check_command(
    input: &std::path::Path,
    format: OutputFormat,
    plugins: &[PathBuf],
) -> Result<()> {
    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    // Step 1: Read input AsciiDoc file
    let content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // Step 2: Parse AsciiDoc to AST
    let ast = parse(&content).context("Failed to parse AsciiDoc content")?;

    // Step 3: Run built-in validation engine
    let engine = ValidationEngine::with_defaults();
    let mut diagnostics: Vec<Diagnostic> = engine
        .validate(&ast)
        .into_iter()
        .map(|d| d.with_file(input.display().to_string()))
        .collect();

    // Step 4: Run plugin scripts
    if !plugins.is_empty() {
        let plugin_engine = PluginEngine::new();

        for plugin_path in plugins {
            if !plugin_path.exists() {
                anyhow::bail!("Plugin script not found: {}", plugin_path.display());
            }

            // Compile the script
            let script_ast = plugin_engine
                .compile_file(plugin_path)
                .with_context(|| format!("Failed to compile plugin: {}", plugin_path.display()))?;

            // Run validation
            let plugin_diagnostics = plugin_engine
                .run_validation(&ast, &script_ast)
                .with_context(|| format!("Failed to run plugin: {}", plugin_path.display()))?;

            // Add file info and merge diagnostics
            for diag in plugin_diagnostics {
                diagnostics.push(diag.with_file(input.display().to_string()));
            }
        }
    }

    // Step 5: Output based on format
    match format {
        OutputFormat::Json => {
            // JSON output for LLM consumption
            let json = serde_json::to_string_pretty(&diagnostics)
                .context("Failed to serialize diagnostics to JSON")?;
            println!("{}", json);
        }
        OutputFormat::Text => {
            // Human-readable output
            if diagnostics.is_empty() {
                println!("✓ No issues found in {}", input.display());
            } else {
                for diag in &diagnostics {
                    println!("{}", diag);
                    println!();
                }
                let error_count = diagnostics.iter().filter(|d| d.is_error()).count();
                let warning_count = diagnostics.iter().filter(|d| d.is_warning()).count();
                println!(
                    "Found {} error(s) and {} warning(s)",
                    error_count, warning_count
                );
            }
        }
    }

    // Exit with error code if there are errors
    let has_errors = diagnostics.iter().any(|d| d.is_error());
    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Execute the audit command (CI/CD compliance check)
pub fn audit_command(
    input: &std::path::Path,
    format: AuditFormat,
    strict: bool,
    config_path: Option<&std::path::Path>,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Auditing: {}", input.display());

    // Load settings from config file if provided
    let settings = load_settings(config_path)?;

    // Load workspace graph from directory
    let graph = load_workspace_graph(input)?;

    println!("  Found {} documents", graph.document_count());

    // Create compliance engine with settings
    let engine = ComplianceEngine::with_settings(&settings);

    // Run compliance checks
    let result = engine.run_with_stats(&graph);

    // Output based on format
    match format {
        AuditFormat::Text => {
            println!();
            println!("=== Compliance Report ===");
            println!();
            println!("Score: {}%", result.compliance_score);
            println!("Documents: {}", result.total_documents);
            println!("Errors: {}", result.errors);
            println!("Warnings: {}", result.warnings);
            println!("Info: {}", result.info);
            println!();

            if result.violations.is_empty() {
                println!("✓ No compliance violations found");
            } else {
                println!("Violations:");
                for v in &result.violations {
                    let severity = match v.severity {
                        utf8dok_lsp::compliance::ViolationSeverity::Error => "ERROR",
                        utf8dok_lsp::compliance::ViolationSeverity::Warning => "WARN",
                        utf8dok_lsp::compliance::ViolationSeverity::Info => "INFO",
                    };
                    println!("  [{}] {}: {}", severity, v.code, v.message);
                    println!("         at {}", v.uri);
                }
            }
        }
        AuditFormat::Json => {
            let dashboard = ComplianceDashboard::new(&engine, &graph);
            let json = dashboard.generate_json();
            println!("{}", json);
        }
        AuditFormat::Markdown => {
            let dashboard = ComplianceDashboard::new(&engine, &graph);
            let markdown = dashboard.generate_markdown();
            println!("{}", markdown);
        }
    }

    // Exit with error code in strict mode if there are violations
    if strict && !result.is_clean() {
        std::process::exit(1);
    }

    Ok(())
}

/// Execute the dashboard command (HTML report generation)
pub fn dashboard_command(
    input: &std::path::Path,
    output: &std::path::Path,
    config_path: Option<&std::path::Path>,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Generating dashboard for: {}", input.display());

    // Load settings from config file if provided
    let settings = load_settings(config_path)?;

    // Load workspace graph from directory
    let graph = load_workspace_graph(input)?;

    println!("  Found {} documents", graph.document_count());

    // Create compliance engine with settings
    let engine = ComplianceEngine::with_settings(&settings);

    // Generate HTML dashboard
    let dashboard = ComplianceDashboard::new(&engine, &graph);
    let html = dashboard.generate_html();

    // Write output
    fs::write(output, &html)
        .with_context(|| format!("Failed to write dashboard: {}", output.display()))?;

    // Run checks for summary
    let result = engine.run_with_stats(&graph);

    println!();
    println!("Dashboard generated!");
    println!("  Output: {}", output.display());
    println!("  Score: {}%", result.compliance_score);
    println!("  Documents: {}", result.total_documents);
    println!("  Violations: {}", result.violations.len());

    Ok(())
}

/// Execute the dual-nature command (analyze slide/document dual-nature documents)
pub fn dual_nature_command(
    input: &std::path::Path,
    target: DualNatureTargetFormat,
    format: OutputFormat,
    validate_only: bool,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Analyzing dual-nature document: {}", input.display());

    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    // Read the input file
    let content = fs::read_to_string(input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    // Parse as dual-nature document
    let doc = parse_dual_nature(&content);

    // Validate the document
    let validation = validate_dual_nature(&doc);

    // Output validation results
    if validate_only || !validation.is_valid || validation.has_issues() {
        match format {
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "file": input.display().to_string(),
                    "title": doc.title,
                    "is_valid": validation.is_valid,
                    "errors": validation.errors.iter().map(|e| {
                        serde_json::json!({
                            "code": e.code,
                            "message": e.message,
                            "line": e.line,
                            "suggestion": e.suggestion
                        })
                    }).collect::<Vec<_>>(),
                    "warnings": validation.warnings.iter().map(|w| {
                        serde_json::json!({
                            "code": w.code,
                            "message": w.message,
                            "line": w.line,
                            "suggestion": w.suggestion
                        })
                    }).collect::<Vec<_>>(),
                    "info": validation.info.iter().map(|i| {
                        serde_json::json!({
                            "code": i.code,
                            "message": i.message,
                            "line": i.line,
                            "suggestion": i.suggestion
                        })
                    }).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output)?);
                if validate_only {
                    return Ok(());
                }
            }
            OutputFormat::Text => {
                println!();
                println!("=== Validation Results ===");
                println!();

                if validation.errors.is_empty() && validation.warnings.is_empty() {
                    println!("✓ Document is valid");
                } else {
                    for err in &validation.errors {
                        print!("[ERROR] {}: {}", err.code, err.message);
                        if let Some(line) = err.line {
                            print!(" (line {})", line);
                        }
                        println!();
                        if let Some(ref suggestion) = err.suggestion {
                            println!("  └─ {}", suggestion);
                        }
                    }
                    for warn in &validation.warnings {
                        print!("[WARN] {}: {}", warn.code, warn.message);
                        if let Some(line) = warn.line {
                            print!(" (line {})", line);
                        }
                        println!();
                        if let Some(ref suggestion) = warn.suggestion {
                            println!("  └─ {}", suggestion);
                        }
                    }
                    for info in &validation.info {
                        print!("[INFO] {}: {}", info.code, info.message);
                        if let Some(line) = info.line {
                            print!(" (line {})", line);
                        }
                        println!();
                        if let Some(ref suggestion) = info.suggestion {
                            println!("  └─ {}", suggestion);
                        }
                    }
                }

                if validate_only {
                    return Ok(());
                }
            }
        }
    }

    // Transform and display content based on target format
    match format {
        OutputFormat::Json => {
            output_dual_nature_json(&doc, target)?;
        }
        OutputFormat::Text => {
            output_dual_nature_text(&doc, target);
        }
    }

    Ok(())
}

/// Output dual-nature analysis as JSON
fn output_dual_nature_json(
    doc: &utf8dok_core::dual_nature::DualNatureDocument,
    target: DualNatureTargetFormat,
) -> Result<()> {
    let slide_blocks = transform_for_format(doc, DualNatureFormat::Slide);
    let doc_blocks = transform_for_format(doc, DualNatureFormat::Document);

    let json_output = match target {
        DualNatureTargetFormat::Slide => {
            serde_json::json!({
                "format": "slide",
                "title": doc.title,
                "block_count": slide_blocks.len(),
                "blocks": format_blocks_json(&slide_blocks),
            })
        }
        DualNatureTargetFormat::Document => {
            serde_json::json!({
                "format": "document",
                "title": doc.title,
                "block_count": doc_blocks.len(),
                "blocks": format_blocks_json(&doc_blocks),
            })
        }
        DualNatureTargetFormat::Both => {
            serde_json::json!({
                "title": doc.title,
                "slide": {
                    "block_count": slide_blocks.len(),
                    "blocks": format_blocks_json(&slide_blocks),
                },
                "document": {
                    "block_count": doc_blocks.len(),
                    "blocks": format_blocks_json(&doc_blocks),
                },
            })
        }
    };

    println!("{}", serde_json::to_string_pretty(&json_output)?);
    Ok(())
}

/// Format blocks as JSON value
fn format_blocks_json(
    blocks: &[utf8dok_core::dual_nature::DualNatureBlock],
) -> Vec<serde_json::Value> {
    blocks
        .iter()
        .map(|b| {
            serde_json::json!({
                "selector": format!("{:?}", b.selector),
                "content_type": get_content_type(&b.content),
                "line": b.source_line,
            })
        })
        .collect()
}

/// Get content type string for JSON output
fn get_content_type(content: &utf8dok_core::dual_nature::BlockContent) -> &'static str {
    use utf8dok_core::dual_nature::BlockContent;
    match content {
        BlockContent::Section(_) => "section",
        BlockContent::Paragraph(_) => "paragraph",
        BlockContent::BulletList(_) => "bullet_list",
        BlockContent::NumberedList(_) => "numbered_list",
        BlockContent::Code(_) => "code",
        BlockContent::Image(_) => "image",
        BlockContent::Table(_) => "table",
        BlockContent::Include(_) => "include",
        BlockContent::Raw(_) => "raw",
    }
}

/// Output dual-nature analysis as text
fn output_dual_nature_text(
    doc: &utf8dok_core::dual_nature::DualNatureDocument,
    target: DualNatureTargetFormat,
) {
    println!();
    if let Some(ref title) = doc.title {
        println!("Title: {}", title);
    }
    println!();

    match target {
        DualNatureTargetFormat::Slide => {
            let blocks = transform_for_format(doc, DualNatureFormat::Slide);
            println!("=== Slide View ({} blocks) ===", blocks.len());
            println!();
            print_blocks_text(&blocks);
        }
        DualNatureTargetFormat::Document => {
            let blocks = transform_for_format(doc, DualNatureFormat::Document);
            println!("=== Document View ({} blocks) ===", blocks.len());
            println!();
            print_blocks_text(&blocks);
        }
        DualNatureTargetFormat::Both => {
            let slide_blocks = transform_for_format(doc, DualNatureFormat::Slide);
            let doc_blocks = transform_for_format(doc, DualNatureFormat::Document);

            println!("=== Slide View ({} blocks) ===", slide_blocks.len());
            println!();
            print_blocks_text(&slide_blocks);

            println!();
            println!("=== Document View ({} blocks) ===", doc_blocks.len());
            println!();
            print_blocks_text(&doc_blocks);
        }
    }
}

/// Print blocks in text format
fn print_blocks_text(blocks: &[utf8dok_core::dual_nature::DualNatureBlock]) {
    use utf8dok_core::dual_nature::BlockContent;

    for block in blocks {
        let selector_str = match block.selector {
            ContentSelector::Both => "[both]",
            ContentSelector::Slide => "[slide]",
            ContentSelector::SlideOnly => "[slide-only]",
            ContentSelector::Document => "[document]",
            ContentSelector::DocumentOnly => "[document-only]",
            ContentSelector::Conditional(_) => "[conditional]",
        };

        match &block.content {
            BlockContent::Section(s) => {
                let prefix = "=".repeat(s.level + 1);
                println!("{} {} {}", selector_str, prefix, s.title);
            }
            BlockContent::Paragraph(text) => {
                let preview = if text.len() > 60 {
                    format!("{}...", &text[..60])
                } else {
                    text.clone()
                };
                println!("{} [para] {}", selector_str, preview);
            }
            BlockContent::BulletList(items) => {
                println!("{} [list] {} items", selector_str, items.len());
                for (i, item) in items.iter().take(3).enumerate() {
                    let preview = if item.len() > 40 {
                        format!("{}...", &item[..40])
                    } else {
                        item.clone()
                    };
                    println!("         {}. {}", i + 1, preview);
                }
                if items.len() > 3 {
                    println!("         ... and {} more", items.len() - 3);
                }
            }
            BlockContent::NumberedList(items) => {
                println!("{} [numbered] {} items", selector_str, items.len());
            }
            BlockContent::Code(c) => {
                println!(
                    "{} [code] {} ({} lines)",
                    selector_str,
                    c.language.as_deref().unwrap_or("text"),
                    c.code.lines().count()
                );
            }
            BlockContent::Image(img) => {
                println!("{} [image] {}", selector_str, img.path);
            }
            BlockContent::Table(_) => {
                println!("{} [table]", selector_str);
            }
            BlockContent::Include(inc) => {
                println!("{} [include] {}", selector_str, inc.path);
            }
            BlockContent::Raw(text) => {
                println!("{} [raw] {} chars", selector_str, text.len());
            }
        }
    }
}

/// Load settings from a config file or use defaults
fn load_settings(config_path: Option<&std::path::Path>) -> Result<Settings> {
    match config_path {
        Some(path) => {
            if !path.exists() {
                anyhow::bail!("Config file not found: {}", path.display());
            }
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read config: {}", path.display()))?;
            Settings::from_toml_str(&content)
                .with_context(|| format!("Failed to parse config: {}", path.display()))
        }
        None => {
            // Try to find utf8dok.toml in common locations
            let candidates = ["utf8dok.toml", ".utf8dok.toml"];
            for candidate in candidates {
                if std::path::Path::new(candidate).exists() {
                    let content = fs::read_to_string(candidate)?;
                    if let Ok(settings) = Settings::from_toml_str(&content) {
                        return Ok(settings);
                    }
                }
            }
            Ok(Settings::default())
        }
    }
}

/// Load all AsciiDoc files from a directory into a WorkspaceGraph
fn load_workspace_graph(dir: &std::path::Path) -> Result<WorkspaceGraph> {
    let mut graph = WorkspaceGraph::new();

    // Find all .adoc and .asciidoc files
    let patterns = [
        dir.join("**/*.adoc").display().to_string(),
        dir.join("**/*.asciidoc").display().to_string(),
    ];

    for pattern in &patterns {
        for entry in glob(pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))? {
            match entry {
                Ok(path) => {
                    // Read file content
                    if let Ok(content) = fs::read_to_string(&path) {
                        // Convert path to file:// URI
                        let uri = format!(
                            "file://{}",
                            path.canonicalize().unwrap_or(path.clone()).display()
                        );
                        graph.add_document(&uri, &content);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not read {}", e);
                }
            }
        }
    }

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_extract() {
        let args = vec!["utf8dok", "extract", "test.docx", "--output", "result"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Extract {
                input,
                output,
                force_parse,
            } => {
                assert_eq!(input, PathBuf::from("test.docx"));
                assert_eq!(output, PathBuf::from("result"));
                assert!(!force_parse);
            }
            _ => panic!("Expected Extract command"),
        }
    }

    #[test]
    fn test_cli_parse_extract_default_output() {
        let args = vec!["utf8dok", "extract", "test.docx"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Extract {
                input,
                output,
                force_parse,
            } => {
                assert_eq!(input, PathBuf::from("test.docx"));
                assert_eq!(output, PathBuf::from("output"));
                assert!(!force_parse);
            }
            _ => panic!("Expected Extract command"),
        }
    }

    #[test]
    fn test_cli_parse_extract_force_parse() {
        let args = vec!["utf8dok", "extract", "test.docx", "--force-parse"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Extract {
                input,
                output,
                force_parse,
            } => {
                assert_eq!(input, PathBuf::from("test.docx"));
                assert_eq!(output, PathBuf::from("output"));
                assert!(force_parse);
            }
            _ => panic!("Expected Extract command"),
        }
    }

    #[test]
    fn test_cli_parse_render() {
        let args = vec![
            "utf8dok",
            "render",
            "doc.adoc",
            "--output",
            "out.docx",
            "--template",
            "tmpl.dotx",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Render {
                input,
                output,
                format,
                template,
                cover,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert_eq!(output, Some(PathBuf::from("out.docx")));
                assert!(matches!(format, RenderFormat::Docx)); // default
                assert_eq!(template, Some(PathBuf::from("tmpl.dotx")));
                assert_eq!(cover, None);
            }
            _ => panic!("Expected Render command"),
        }
    }

    #[test]
    fn test_cli_parse_render_pptx() {
        let args = vec![
            "utf8dok",
            "render",
            "slides.adoc",
            "--format",
            "pptx",
            "--output",
            "slides.pptx",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Render {
                input,
                output,
                format,
                template,
                cover: _,
            } => {
                assert_eq!(input, PathBuf::from("slides.adoc"));
                assert_eq!(output, Some(PathBuf::from("slides.pptx")));
                assert!(matches!(format, RenderFormat::Pptx));
                assert_eq!(template, None);
            }
            _ => panic!("Expected Render command"),
        }
    }

    #[test]
    fn test_cli_parse_render_pptx_with_template() {
        let args = vec![
            "utf8dok",
            "render",
            "slides.adoc",
            "--format",
            "pptx",
            "--template",
            "corporate.potx",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Render {
                input,
                output,
                format,
                template,
                cover: _,
            } => {
                assert_eq!(input, PathBuf::from("slides.adoc"));
                assert_eq!(output, None);
                assert!(matches!(format, RenderFormat::Pptx));
                assert_eq!(template, Some(PathBuf::from("corporate.potx")));
            }
            _ => panic!("Expected Render command"),
        }
    }

    #[test]
    fn test_cli_parse_check() {
        let args = vec!["utf8dok", "check", "doc.adoc"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Check {
                input,
                format,
                plugin,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(format, OutputFormat::Text));
                assert!(plugin.is_empty());
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_cli_parse_check_json() {
        let args = vec!["utf8dok", "check", "doc.adoc", "--format", "json"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Check {
                input,
                format,
                plugin,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(format, OutputFormat::Json));
                assert!(plugin.is_empty());
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_cli_parse_check_with_plugin() {
        let args = vec![
            "utf8dok",
            "check",
            "doc.adoc",
            "--plugin",
            "rules/test.rhai",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Check {
                input,
                format,
                plugin,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(format, OutputFormat::Text));
                assert_eq!(plugin.len(), 1);
                assert_eq!(plugin[0], PathBuf::from("rules/test.rhai"));
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_cli_parse_check_multiple_plugins() {
        let args = vec![
            "utf8dok",
            "check",
            "doc.adoc",
            "--plugin",
            "rules/a.rhai",
            "--plugin",
            "rules/b.rhai",
        ];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Check {
                input,
                format: _,
                plugin,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert_eq!(plugin.len(), 2);
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_cli_parse_audit() {
        let args = vec!["utf8dok", "audit", "docs/"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Audit {
                input,
                format,
                strict,
                config,
            } => {
                assert_eq!(input, PathBuf::from("docs/"));
                assert!(matches!(format, AuditFormat::Text));
                assert!(!strict);
                assert!(config.is_none());
            }
            _ => panic!("Expected Audit command"),
        }
    }

    #[test]
    fn test_cli_parse_audit_strict() {
        let args = vec!["utf8dok", "audit", "--strict", "--format", "json"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Audit {
                input,
                format,
                strict,
                config: _,
            } => {
                assert_eq!(input, PathBuf::from(".")); // default
                assert!(matches!(format, AuditFormat::Json));
                assert!(strict);
            }
            _ => panic!("Expected Audit command"),
        }
    }

    #[test]
    fn test_cli_parse_dashboard() {
        let args = vec!["utf8dok", "dashboard", "docs/", "--output", "report.html"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Dashboard {
                input,
                output,
                config,
            } => {
                assert_eq!(input, PathBuf::from("docs/"));
                assert_eq!(output, PathBuf::from("report.html"));
                assert!(config.is_none());
            }
            _ => panic!("Expected Dashboard command"),
        }
    }

    #[test]
    fn test_cli_parse_dashboard_with_config() {
        let args = vec!["utf8dok", "dashboard", ".", "--config", "custom.toml"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Dashboard {
                input,
                output,
                config,
            } => {
                assert_eq!(input, PathBuf::from("."));
                assert_eq!(output, PathBuf::from("compliance-dashboard.html")); // default
                assert_eq!(config, Some(PathBuf::from("custom.toml")));
            }
            _ => panic!("Expected Dashboard command"),
        }
    }

    #[test]
    fn test_load_workspace_graph_empty() {
        // Create a temp directory with no files
        let temp_dir = tempfile::tempdir().unwrap();
        let graph = load_workspace_graph(temp_dir.path()).unwrap();
        assert_eq!(graph.document_count(), 0);
    }

    #[test]
    fn test_load_settings_default() {
        // Should return default settings when no config exists
        let settings = load_settings(None).unwrap();
        // Just verify it doesn't panic and returns something
        assert_eq!(settings.workspace.entry_points.len(), 2);
    }

    // ==================== INIT COMMAND TESTS ====================

    #[test]
    fn test_init_bridge_scaffolds_project() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("my-docs");

        init_command(&project_path, InitTemplate::Bridge).unwrap();

        // Check all expected files exist
        assert!(project_path.join("utf8dok.toml").exists());
        assert!(project_path.join("index.adoc").exists());
        assert!(project_path
            .join("adr/0001-record-architecture-decisions.adoc")
            .exists());
    }

    #[test]
    fn test_init_basic_scaffolds_project() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("basic-docs");

        init_command(&project_path, InitTemplate::Basic).unwrap();

        // Check expected files exist
        assert!(project_path.join("utf8dok.toml").exists());
        assert!(project_path.join("index.adoc").exists());
        // Basic template should NOT have adr/ directory
        assert!(!project_path.join("adr").exists());
    }

    #[test]
    fn test_init_bridge_config_content() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("test-docs");

        init_command(&project_path, InitTemplate::Bridge).unwrap();

        let config = fs::read_to_string(project_path.join("utf8dok.toml")).unwrap();
        assert!(config.contains("[compliance.bridge]"));
        assert!(config.contains("orphans = \"error\""));
        assert!(config.contains("writing_quality = true"));
    }

    #[test]
    fn test_init_bridge_index_links_to_adr() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("test-docs");

        init_command(&project_path, InitTemplate::Bridge).unwrap();

        let index = fs::read_to_string(project_path.join("index.adoc")).unwrap();
        assert!(index.contains("<<adr/0001-record-architecture-decisions.adoc#"));
        assert!(index.contains("= Documentation Hub"));
    }

    #[test]
    fn test_init_bridge_adr_has_required_attributes() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("test-docs");

        init_command(&project_path, InitTemplate::Bridge).unwrap();

        let adr =
            fs::read_to_string(project_path.join("adr/0001-record-architecture-decisions.adoc"))
                .unwrap();

        assert!(adr.contains(":status: Accepted"));
        assert!(adr.contains(":date:"));
        assert!(adr.contains("[[adr-0001]]"));
        assert!(adr.contains("== Context"));
        assert!(adr.contains("== Decision"));
        assert!(adr.contains("== Consequences"));
    }

    #[test]
    fn test_init_fails_on_non_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("existing-docs");

        // Create directory with a file
        fs::create_dir_all(&project_path).unwrap();
        fs::write(project_path.join("existing.txt"), "content").unwrap();

        // Init should fail
        let result = init_command(&project_path, InitTemplate::Bridge);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not empty"));
    }

    #[test]
    fn test_init_succeeds_on_empty_directory() {
        let temp = tempfile::tempdir().unwrap();
        let project_path = temp.path().join("empty-docs");

        // Create empty directory
        fs::create_dir_all(&project_path).unwrap();

        // Init should succeed
        let result = init_command(&project_path, InitTemplate::Bridge);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_cli_parse() {
        let args = vec!["utf8dok", "init", "my-project"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Init { path, template } => {
                assert_eq!(path, PathBuf::from("my-project"));
                assert!(matches!(template, InitTemplate::Bridge)); // default
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_cli_parse_with_template() {
        let args = vec!["utf8dok", "init", "my-project", "--template", "basic"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Init { path, template } => {
                assert_eq!(path, PathBuf::from("my-project"));
                assert!(matches!(template, InitTemplate::Basic));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_chrono_lite_date_format() {
        let date = chrono_lite_date();
        // Should be YYYY-MM-DD format
        assert_eq!(date.len(), 10);
        assert_eq!(&date[4..5], "-");
        assert_eq!(&date[7..8], "-");
        // Year should be reasonable (2020-2100)
        let year: u32 = date[0..4].parse().unwrap();
        assert!(year >= 2020 && year <= 2100);
    }

    // ==================== DUAL-NATURE COMMAND TESTS ====================

    #[test]
    fn test_cli_parse_dual_nature() {
        let args = vec!["utf8dok", "dual-nature", "doc.adoc"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::DualNature {
                input,
                target,
                format,
                validate_only,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(target, DualNatureTargetFormat::Both)); // default
                assert!(matches!(format, OutputFormat::Text)); // default
                assert!(!validate_only);
            }
            _ => panic!("Expected DualNature command"),
        }
    }

    #[test]
    fn test_cli_parse_dual_nature_slide_target() {
        let args = vec!["utf8dok", "dual-nature", "doc.adoc", "--target", "slide"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::DualNature {
                input,
                target,
                format: _,
                validate_only: _,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(target, DualNatureTargetFormat::Slide));
            }
            _ => panic!("Expected DualNature command"),
        }
    }

    #[test]
    fn test_cli_parse_dual_nature_document_target() {
        let args = vec!["utf8dok", "dual-nature", "doc.adoc", "--target", "document"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::DualNature {
                input,
                target,
                format: _,
                validate_only: _,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(target, DualNatureTargetFormat::Document));
            }
            _ => panic!("Expected DualNature command"),
        }
    }

    #[test]
    fn test_cli_parse_dual_nature_json_format() {
        let args = vec!["utf8dok", "dual-nature", "doc.adoc", "--format", "json"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::DualNature {
                input,
                target: _,
                format,
                validate_only: _,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(matches!(format, OutputFormat::Json));
            }
            _ => panic!("Expected DualNature command"),
        }
    }

    #[test]
    fn test_cli_parse_dual_nature_validate_only() {
        let args = vec!["utf8dok", "dual-nature", "doc.adoc", "--validate-only"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::DualNature {
                input,
                target: _,
                format: _,
                validate_only,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert!(validate_only);
            }
            _ => panic!("Expected DualNature command"),
        }
    }

    #[test]
    fn test_dual_nature_command_with_file() {
        let temp = tempfile::tempdir().unwrap();
        let adoc_path = temp.path().join("dual.adoc");

        let content = r#"= Dual Nature Test
:slide-bullets: 3

[.slide]
== Executive Summary

* Point 1
* Point 2
* Point 3

[.document-only]
== Detailed Analysis

This section appears only in the document.
"#;

        fs::write(&adoc_path, content).unwrap();

        // Run dual-nature command
        let result = dual_nature_command(
            &adoc_path,
            DualNatureTargetFormat::Both,
            OutputFormat::Text,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_dual_nature_command_validate_only() {
        let temp = tempfile::tempdir().unwrap();
        let adoc_path = temp.path().join("validate.adoc");

        let content = r#"= Validation Test

[.slide-only]
== Slides Only Section

* Point 1
"#;

        fs::write(&adoc_path, content).unwrap();

        // Run with validate-only
        let result = dual_nature_command(
            &adoc_path,
            DualNatureTargetFormat::Slide,
            OutputFormat::Text,
            true,
        );
        assert!(result.is_ok());
    }
}
