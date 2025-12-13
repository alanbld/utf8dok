//! Example: Extract AsciiDoc from a DOCX file
//!
//! Usage: cargo run --example extract -- path/to/document.docx

use std::env;
use std::path::Path;
use utf8dok_ooxml::{AsciiDocExtractor, OoxmlArchive};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path/to/document.docx>", args[0]);
        eprintln!();
        eprintln!("Extracts AsciiDoc content from a DOCX file and prints:");
        eprintln!("  - Document structure");
        eprintln!("  - Style mappings (TOML format)");
        eprintln!("  - Converted AsciiDoc content");
        std::process::exit(1);
    }

    let docx_path = Path::new(&args[1]);

    if !docx_path.exists() {
        eprintln!("Error: File not found: {}", docx_path.display());
        std::process::exit(1);
    }

    println!("=== Extracting from: {} ===\n", docx_path.display());

    // Open the archive
    let archive = match OoxmlArchive::open(docx_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error opening DOCX: {}", e);
            std::process::exit(1);
        }
    };

    // List archive contents
    println!("--- Archive Contents ---");
    for name in archive.file_list() {
        println!("  {}", name);
    }
    println!();

    // Extract using AsciiDocExtractor
    let extractor = AsciiDocExtractor::new();
    let extracted = match extractor.extract_file(docx_path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error extracting document: {}", e);
            std::process::exit(1);
        }
    };

    // Print style mappings
    println!("--- Style Mappings (TOML) ---");
    println!("{}", extracted.style_mappings.to_toml());

    // Print extracted AsciiDoc
    println!("--- Extracted AsciiDoc ---");
    println!("{}", extracted.asciidoc);

    println!("--- Extraction Complete ---");
}
