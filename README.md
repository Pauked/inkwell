# inkwell

Convert Kindle HTML notebook exports to Markdown for use in Obsidian. Designed for sideloaded ePub files where Kindle's official API and sync tools don't provide export functionality.

## Features

- Converts Kindle macOS HTML notebook exports to clean Markdown
- Preserves all highlight metadata:
  - Highlight colors (yellow, pink, blue, orange, green, aqua, red)
  - Optional color-coded metadata labels using [Obsidian Painter](https://github.com/KraXen72/obsidian-painter) classes (off by default)
  - Page numbers and locations
  - Section headings and subheadings
  - Notes and bookmarks
- YAML frontmatter with book metadata and creation timestamp
- Supports MLA citation format from Kindle macOS exports
- Configurable default export folder (supports multiple config locations)
- Automatic file naming based on author and title
- Fully unit tested (18 tests) with cargo clippy compliance

## Installation

### Option 1: Build and Install Locally

```bash
cargo install --path .
```

### Option 2: Deploy to Dropbox (Recommended)

```bash
./deploy-local.sh
```

This will:
- Build a release binary
- Deploy to `$DROPBOX_PATH/Utils/inkwell` (or `~/Desktop/inkwell` if not set)
- Create a sample `config.toml` with your export folder path
- Create a README.txt with usage instructions

The binary in Dropbox can be run from anywhere and will use the config.toml in its directory.

**Note**: Set the `DROPBOX_PATH` environment variable to point to your Dropbox folder:
```bash
export DROPBOX_PATH="/path/to/your/Dropbox"
```

## Usage

### Basic Usage

Convert a Kindle HTML export to Markdown:

```bash
inkwell "/path/to/Book Title - Notebook.html"
```

This will create a Markdown file in your configured default export folder.

### Specify Output File

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
4. Choose MLA citation format when prompted
5. Save the HTML file
6. Run inkwell on the exported file

**Note:** This tool is specifically designed for sideloaded ePub files in Kindle where the official Kindle API and sync tools don't provide export functionality.

## Output Format

The generated Markdown includes:

- YAML frontmatter with title, author, and citation
- Book metadata section
- Highlights organized by chapter/section
- All notes and bookmarks preserved with their context
- Page numbers and locations for easy reference

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

Run tests (18 tests):

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

Deploy locally:

```bash
./deploy-local.sh
```

## License

MIT
