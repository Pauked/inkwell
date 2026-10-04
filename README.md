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
  - Colour on the passage, the colour label, or both, as native Obsidian 1.14+ highlights (default) or [Obsidian Painter](https://github.com/KraXen72/obsidian-painter) classes
  - Blockquote layout (default) or a one-line layout matching Flint
  - Page numbers and locations
  - Section headings and subheadings
  - Notes and bookmarks
- YAML frontmatter with book metadata, highlight count and last-run timestamp
- Supports all Kindle citation formats: MLA, APA, Chicago Style, or None
- Configurable default export folder (supports multiple config locations)
- Automatic file naming based on author and title
- Fully unit tested (66 tests) with cargo clippy compliance

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

Both dialects are handled. A physical Kindle writes `Location 123-125` (the note records the first number) and also emits notes and bookmarks; CrossInk writes the chapter title instead, which becomes the section heading. The frontmatter `inkwell-source` property says which: `kindle-clippings` or `crossink-clippings`. Malformed or empty clippings are skipped with a warning on stderr.

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

# Highlight layout (optional): "quote" (default) or "line"
highlight_layout = "quote"

# Highlight colours (optional; these are the defaults)
[highlight_colours]
style = "obsidian"   # "obsidian" (==🟣text==) or "painter" (<mark class="hltr-p">text</mark>)
text = false         # colour the highlighted passage
label = true         # colour the colour-name label on the metadata line
```

With the defaults, a pink highlight renders as:

```markdown
> People like this tend to thrive.

**Highlight** (==🟣pink==) - Page 14 - Location 126
```

With `highlight_layout = "line"` the same highlight sits on one line, Flint-style:

```markdown
People like this tend to thrive. — ==🟣pink== | *Self-driving people* - Page 14 - Location 126
```

Notes and bookmarks look the same in both layouts.

| `style` | Markup | Needs |
|---|---|---|
| `obsidian` | `==🟣text==` | Obsidian 1.14+ |
| `painter` | `<mark class="hltr-p">text</mark>` | [Painter](https://github.com/KraXen72/obsidian-painter) plugin or a CSS snippet for `hltr-*` |

Set `text` and `label` to `false` for no colour markup at all. Obsidian colour mapping: orange 🟠, green 🟢, blue and aqua 🔵, pink 🟣, red 🔴; yellow gets no emoji, since a plain `==text==` is Obsidian's default yellow. Text that already contains `==` is left unwrapped.

The older `enable_painter_highlights = true` still works and gives its original output (Painter style, label only); `[highlight_colours]` wins if both are set. The Painter plugin can interfere with Obsidian's native highlight swatch, so turn it off when using `obsidian`.

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

- YAML frontmatter:
  - `title`, `author` and `citation` (Kindle's `Citation (Style):` label dropped; the Metadata section keeps it)
  - `inkwell-source`: `kindle-export`, `kindle-clippings` or `crossink-clippings`
  - `inkwell-highlights-count`
  - `inkwell-last-run-date`: when inkwell last wrote the note (`YYYY-MM-DDTHH:MM`, an Obsidian date-time)
  - `inkwell-version`
  - Properties inkwell owns carry the `inkwell-` prefix so they don't clash with others in your vault, such as Web Clipper's `source` URL or a note's `created` date
- Book metadata section
- Highlights organized by chapter/section
- All notes and bookmarks preserved with their context
- Page numbers and locations for easy reference (location omitted when the source has none)

Example output:

```markdown
---
title: "Book Title"
author: "Author Name"
citation: "Author, Name. Book Title. , 2025. Kindle file."
inkwell-source: kindle-export
inkwell-highlights-count: 1
inkwell-last-run-date: "2025-11-02T17:15"
inkwell-version: 0.2.0
---

# Book Title

## Metadata
* Author: Author Name
* Citation: Citation (MLA): Author, Name. Book Title. , 2025. Kindle file.

## Highlights

### Chapter 1

> This is a highlighted passage.

**Highlight** (==🔵blue==) - Page 15 - Location 234

---

**Note**: This is my note about the highlight.

*Chapter 1 - Page 15 - Location 235*

---
```

## Development

Run tests (66 tests):

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
