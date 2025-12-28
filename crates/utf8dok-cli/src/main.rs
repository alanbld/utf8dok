//! utf8dok CLI - Command-line interface for the utf8dok document processor
//!
//! # Usage
//!
//! ```bash
//! # Extract AsciiDoc from a DOCX file
//! utf8dok extract document.docx --output result/
//!
//! # Render AsciiDoc to DOCX (coming soon)
//! utf8dok render document.adoc --output result.docx
//! ```

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use utf8dok_core::diagnostics::Diagnostic;
use utf8dok_core::parse;
use utf8dok_ooxml::{
    AsciiDocExtractor, DocxWriter, OoxmlArchive, SourceOrigin, StyleSheet, Template,
};
use utf8dok_plugins::PluginEngine;
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

#[derive(Parser)]
#[command(name = "utf8dok")]
#[command(author, version, about = "Plain text, powerful docs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    /// Render AsciiDoc to DOCX using a template (coming soon)
    Render {
        /// Input AsciiDoc file
        input: PathBuf,

        /// Output DOCX file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Template DOTX file
        #[arg(short, long)]
        template: Option<PathBuf>,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
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
            template,
        } => {
            render_command(&input, output.as_deref(), template.as_deref())?;
        }
        Commands::Check {
            input,
            format,
            plugin,
        } => {
            check_command(&input, format, &plugin)?;
        }
    }

    Ok(())
}

/// Execute the extract command
fn extract_command(input: &PathBuf, output_dir: &PathBuf, force_parse: bool) -> Result<()> {
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
fn render_command(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    template: Option<&std::path::Path>,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Rendering: {}", input.display());

    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

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

/// Execute the check command
fn check_command(input: &std::path::Path, format: OutputFormat, plugins: &[PathBuf]) -> Result<()> {
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
                template,
            } => {
                assert_eq!(input, PathBuf::from("doc.adoc"));
                assert_eq!(output, Some(PathBuf::from("out.docx")));
                assert_eq!(template, Some(PathBuf::from("tmpl.dotx")));
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
}
