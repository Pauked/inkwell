mod config;
mod constants;
mod parser;
mod markdown;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use std::fs;
use std::path::PathBuf;

#[derive(ClapParser, Debug)]
#[command(name = constants::CRATE_NAME)]
#[command(author = constants::CRATE_AUTHORS)]
#[command(version = constants::CRATE_VERSION)]
#[command(
    help_template = "{about-section}Version : {version}\nAuthor  : {author} \n\n{usage-heading} {usage} \n\n{all-args} {tab}"
)]
#[command(about, long_about = None)]
struct Args {
    /// Path to the Kindle HTML export file
    #[arg(value_name = "HTML_FILE")]
    input: PathBuf,

    /// Output file path (optional, defaults to config default_export_folder)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,

    /// Override the default export folder for this conversion
    #[arg(short = 'd', long, value_name = "DIRECTORY")]
    export_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load config
    let config = config::Config::load()
        .context("Failed to load configuration")?;

    // Read HTML file
    let html_content = fs::read_to_string(&args.input)
        .context(format!("Failed to read HTML file: {:?}", args.input))?;

    // Parse HTML
    let book = parser::parse_html(&html_content)
        .context("Failed to parse HTML content")?;

    // Generate Markdown
    let markdown = markdown::generate_markdown(&book, config.enable_painter_highlights)
        .context("Failed to generate Markdown")?;

    // Determine output path
    let output_path = if let Some(output) = args.output {
        output
    } else {
        let export_dir = args.export_dir.unwrap_or(config.default_export_folder);

        // Create filename from book title and author
        let filename = sanitize_filename(&format!("{} - {}.md", book.author, book.title));
        export_dir.join(filename)
    };

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create output directory")?;
    }

    // Write output file
    fs::write(&output_path, markdown)
        .context(format!("Failed to write output file: {:?}", output_path))?;

    println!("✓ Converted successfully!");
    println!("  Input:  {:?}", args.input);
    println!("  Output: {:?}", output_path);

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
