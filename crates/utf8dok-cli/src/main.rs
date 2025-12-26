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
use clap::{Parser, Subcommand};

use utf8dok_core::generate;
use utf8dok_ooxml::{convert_document_with_styles, Document, OoxmlArchive, StyleSheet};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { input, output } => {
            extract_command(&input, &output)?;
        }
        Commands::Render {
            input,
            output,
            template,
        } => {
            render_command(&input, output.as_deref(), template.as_deref())?;
        }
    }

    Ok(())
}

/// Execute the extract command
fn extract_command(input: &PathBuf, output_dir: &PathBuf) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("Extracting: {}", input.display());

    // Check input file exists
    if !input.exists() {
        anyhow::bail!("Input file not found: {}", input.display());
    }

    // Open the DOCX archive
    let archive = OoxmlArchive::open(input)
        .with_context(|| format!("Failed to open DOCX file: {}", input.display()))?;

    // Parse document content
    let document_xml = archive
        .document_xml()
        .context("Failed to read document.xml from archive")?;
    let document =
        Document::parse(document_xml).context("Failed to parse document content")?;

    // Parse styles
    let styles_xml = archive
        .styles_xml()
        .context("Failed to read styles.xml from archive")?;
    let styles = StyleSheet::parse(styles_xml).context("Failed to parse styles")?;

    // Convert to AST
    let ast_doc = convert_document_with_styles(&document, &styles);

    // Generate AsciiDoc
    let asciidoc = generate(&ast_doc);

    // Create output directory
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    // Write AsciiDoc file
    let adoc_path = output_dir.join("document.adoc");
    fs::write(&adoc_path, &asciidoc)
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
    println!("  {} blocks extracted", ast_doc.blocks.len());

    Ok(())
}

/// Generate configuration TOML from styles
fn generate_config_toml(styles: &StyleSheet, input: &PathBuf) -> String {
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

/// Execute the render command (placeholder)
fn render_command(
    input: &PathBuf,
    output: Option<&std::path::Path>,
    template: Option<&std::path::Path>,
) -> Result<()> {
    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!();
    println!("Render command is not yet implemented.");
    println!();
    println!("Planned usage:");
    println!("  Input:    {}", input.display());
    if let Some(out) = output {
        println!("  Output:   {}", out.display());
    }
    if let Some(tmpl) = template {
        println!("  Template: {}", tmpl.display());
    }
    println!();
    println!("This feature will:");
    println!("  1. Parse the AsciiDoc input file");
    println!("  2. Load the DOTX template");
    println!("  3. Inject content into the template");
    println!("  4. Generate a styled DOCX file");

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
            Commands::Extract { input, output } => {
                assert_eq!(input, PathBuf::from("test.docx"));
                assert_eq!(output, PathBuf::from("result"));
            }
            _ => panic!("Expected Extract command"),
        }
    }

    #[test]
    fn test_cli_parse_extract_default_output() {
        let args = vec!["utf8dok", "extract", "test.docx"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Extract { input, output } => {
                assert_eq!(input, PathBuf::from("test.docx"));
                assert_eq!(output, PathBuf::from("output"));
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
}
