# inkwell

Convert e-reader highlights to Markdown notes for Obsidian. One note per book, highlights grouped by chapter, with page and location references and YAML frontmatter.

Reads two inputs:

- **Kindle macOS app HTML notebook exports** (one book per file). This is the only export route for sideloaded ePubs, which Kindle's sync and notebook tools don't cover.
- **`My Clippings.txt`** (many books per file), as written by a physical Kindle or by any reader using the same format, such as the Xteink X3 running [CrossInk](https://github.com/uxjulia/CrossInk).

## Features

- Converts Kindle macOS HTML notebook exports to clean Markdown
- Imports `My Clippings.txt` (Kindle device or CrossInk firmware): one note per book, chapters as sections, exact duplicates dropped, notes rebuilt from the whole file on every run
- Preserves all highlight metadata:
  - Highlight colors (yellow, pink, blue, orange, green, aqua, red)
  - Optional color-coded metadata labels using [Obsidian Painter](https://github.com/KraXen72/obsidian-painter) classes (off by default)
  - Page numbers and locations
  - Section headings and subheadings
  - Notes and bookmarks
- YAML frontmatter with book metadata and creation timestamp
- Supports all Kindle citation formats: MLA, APA, Chicago Style, or None
- Configurable default export folder (supports multiple config locations)
- Automatic file naming based on author and title
- Fully unit tested (42 tests) with cargo clippy compliance

## Installation

Build from source. Requires a [Rust toolchain](https://rustup.rs).

```bash
git clone https://github.com/Pauked/inkwell.git
cd inkwell
cargo install --path .
```

## Usage

### Basic Usage

Convert a Kindle HTML export to Markdown:

```bash
inkwell "/path/to/Book Title - Notebook.html"
```

This will create a Markdown file in your configured default export folder.

### Import a Clippings File

Any `.txt` input is treated as a Kindle-device `My Clippings.txt`. Every book in the file gets its own `<Author> - <Title>.md` in the export folder, overwritten on each run, so re-running after new highlights is safe:

```bash
inkwell "/Volumes/KINDLE/documents/My Clippings.txt"
```

Both dialects are handled. A physical Kindle writes `Location 123-125` (the note records the first number) and also emits notes and bookmarks; CrossInk writes the chapter title instead, which becomes the section heading. The frontmatter `source` field says which: `kindle-clippings` or `crossink-clippings`. Malformed or empty clippings are skipped with a warning on stderr.

### Specify Output File

Only valid when the input holds a single book.

```bash
inkwell "/path/to/Book Title - Notebook.html" -o "/path/to/output.md"
```

### Override Export Directory

```bash
inkwell "/path/to/Book Title - Notebook.html" -d "/different/folder"
```

## Configuration

Inkwell looks for configuration files in this order:
1. `./config.toml` (current directory)
2. `config.toml` in the same directory as the binary
3. `~/.config/inkwell/config.toml` (user config directory)

Configuration file format:

```toml
default_export_folder = "/path/to/your/obsidian/vault/highlights"

# Enable Obsidian Painter plugin color highlighting (optional, off by default)
# Set to true if you have the Obsidian Painter plugin installed
enable_painter_highlights = false
```

If no config file is found, inkwell defaults to `~/Documents/Inkwell`.

## Exporting Notebooks

1. Open the Kindle app on macOS
2. Select the book you want to export
3. Export the notebook as HTML (File → Export Notebook)
4. Choose your preferred citation format: MLA, APA, Chicago Style, or None
5. Save the HTML file
6. Run inkwell on the exported file

**Note:** This tool is specifically designed for sideloaded ePub files in Kindle where the official Kindle API and sync tools don't provide export functionality. All four citation formats are fully supported.

## Output Format

The generated Markdown includes:

- YAML frontmatter with title, author, citation and source (`kindle-export`, `kindle-clippings` or `crossink-clippings`)
- Book metadata section
- Highlights organized by chapter/section
- All notes and bookmarks preserved with their context
- Page numbers and locations for easy reference (location omitted when the source has none)

Example output:

```markdown
---
title: "Book Title"
author: "Author Name"
citation: "Citation (MLA): Author, Name. Book Title. , 2025. Kindle file."
source: kindle-export
created: "2025-11-02 17:15:01"
---

# Book Title

## Metadata
* Author: Author Name
* Citation: Citation (MLA): Author, Name. Book Title. , 2025. Kindle file.

## Highlights

### Chapter 1

> This is a highlighted passage.

**Highlight** (<mark class="hltr-y">yellow</mark>) - Page 15 - Location 234

---

**Note**: This is my note about the highlight.

*Chapter 1 - Page 15 - Location 235*

---
```

## Development

Run tests (42 tests):

```bash
cargo test
```

Check code quality:

```bash
cargo clippy
```

Build:

```bash
cargo build --release
```

## License

MIT
