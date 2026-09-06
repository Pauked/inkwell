mod clippings;
mod config;
mod constants;
mod markdown;
mod parser;

use anyhow::{Context, Result, bail};
use clap::Parser as ClapParser;
use parser::Book;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(ClapParser, Debug)]
#[command(name = constants::CRATE_NAME)]
#[command(author = constants::CRATE_AUTHORS)]
#[command(version = constants::CRATE_VERSION)]
#[command(
    help_template = "{about-section}Version : {version}\nAuthor  : {author} \n\n{usage-heading} {usage} \n\n{all-args} {tab}"
)]
#[command(about, long_about = None)]
struct Args {
    /// Kindle HTML notebook export, or a "My Clippings.txt" file (.txt)
    #[arg(value_name = "INPUT_FILE")]
    input: PathBuf,

    /// Output file path (optional, defaults to config default_export_folder).
    /// Only valid when the input yields a single book.
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,

    /// Override the default export folder for this conversion
    #[arg(short = 'd', long, value_name = "DIRECTORY")]
    export_dir: Option<PathBuf>,
}

/// Where the generated notes go: a single named file, or a folder that gets
/// one "<Author> - <Title>.md" per book.
enum Destination {
    File(PathBuf),
    Folder(PathBuf),
}

impl Destination {
    fn path_for(&self, book: &Book) -> PathBuf {
        match self {
            Destination::File(path) => path.clone(),
            Destination::Folder(dir) => dir.join(sanitize_filename(&format!(
                "{} - {}.md",
                book.author, book.title
            ))),
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = config::Config::load().context("Failed to load configuration")?;

    let content = fs::read_to_string(&args.input).map_err(|e| {
        anyhow::anyhow!("Failed to read file {:?}: {} ({})", args.input, e, e.kind())
    })?;

    let books = parse_input(&args.input, &content)?;

    let destination = match args.output {
        Some(_) if books.len() > 1 => bail!(
            "--output names a single file but {:?} contains {} books; use --export-dir instead",
            args.input,
            books.len()
        ),
        Some(path) => Destination::File(path),
        None => Destination::Folder(args.export_dir.unwrap_or(config.default_export_folder)),
    };

    println!("✓ Converted successfully!");
    println!("  Input:  {:?}", args.input);
    for book in &books {
        let output_path = destination.path_for(book);
        write_note(book, &output_path, config.enable_painter_highlights)?;
        println!("  Output: {:?}", output_path);
    }

    Ok(())
}

/// Pick the parser by file type: `.txt` is a Kindle-device clippings file
/// (many books), anything else is a Kindle HTML notebook export (one book).
fn parse_input(input: &Path, content: &str) -> Result<Vec<Book>> {
    if is_clippings_file(input) {
        let parsed = clippings::parse_clippings(content);
        for warning in &parsed.warnings {
            eprintln!("warning: {}", warning);
        }
        if parsed.books.is_empty() {
            bail!("No clippings found in {:?}", input);
        }
        Ok(parsed.books)
    } else {
        let book = parser::parse_html(content).context("Failed to parse HTML content")?;
        Ok(vec![book])
    }
}

fn is_clippings_file(input: &Path) -> bool {
    input
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
}

fn write_note(book: &Book, output_path: &Path, enable_painter: bool) -> Result<()> {
    let markdown =
        markdown::generate_markdown(book, enable_painter).context("Failed to generate Markdown")?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).context("Failed to create output directory")?;
    }

    fs::write(output_path, markdown).map_err(|e| {
        anyhow::anyhow!(
            "Failed to write file {:?}: {} ({})",
            output_path,
            e,
            e.kind()
        )
    })
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txt_extension_selects_clippings_parser_case_insensitively() {
        assert!(is_clippings_file(Path::new("/x/My Clippings.txt")));
        assert!(is_clippings_file(Path::new("/x/My Clippings.TXT")));
        assert!(!is_clippings_file(Path::new("/x/Book - Notebook.html")));
        assert!(!is_clippings_file(Path::new("/x/noextension")));
    }

    #[test]
    fn folder_destination_names_file_from_author_and_title() {
        let book = Book {
            title: "Title: Sub/Part?".to_string(),
            author: "An Author".to_string(),
            citation: String::new(),
            source: parser::Source::CrossinkClippings,
            sections: vec![],
        };

        let path = Destination::Folder(PathBuf::from("/vault")).path_for(&book);

        assert_eq!(
            path,
            PathBuf::from("/vault/An Author - Title_ Sub_Part_.md")
        );
    }
}
