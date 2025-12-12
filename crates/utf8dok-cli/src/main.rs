//! utf8dok CLI - Command-line interface for the utf8dok document processor

use clap::Parser;

#[derive(Parser)]
#[command(name = "utf8dok")]
#[command(author, version, about = "Plain text, powerful docs", long_about = None)]
struct Cli {
    /// Input file to process
    #[arg(short, long)]
    input: Option<String>,

    /// Output file (defaults to stdout)
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    println!("utf8dok v{}", utf8dok_core::VERSION);
    println!("A blazing-fast document processor for UTF-8 text formats.");
    println!();
    println!("Coming soon - AsciiDoc parsing and conversion!");

    if let Some(input) = cli.input {
        println!("Input file: {}", input);
    }
    if let Some(output) = cli.output {
        println!("Output file: {}", output);
    }
}
