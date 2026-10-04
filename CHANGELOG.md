# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-10-04

**Breaking:** frontmatter properties renamed; update any Bases or Dataview queries over inkwell notes. Notes are rebuilt on every run, so re-running inkwell updates them.

### Added
- `inkwell-version` frontmatter property recording the version that wrote the note
- `inkwell-highlights-count` frontmatter property
- `highlight_layout` config option: `quote` (default, the existing blockquote) or `line` (`text — colour | position` on one line, matching Flint)
- `[highlight_colours]` config table: `style` (`obsidian` or `painter`), `text` and `label` switches
  - `obsidian` writes Obsidian 1.14+ native colour highlights (`==🟣text==`); `painter` writes `<mark class="hltr-*">`
  - `text` colours the quoted passage, `label` the colour name on the metadata line
  - Orange 🟠, green 🟢, blue and aqua 🔵, pink 🟣, red 🔴; yellow and unknown colours get a plain `==text==`
  - Text already containing `==` is left unwrapped in `obsidian` style

### Changed
- `source` renamed to `inkwell-source`, so it no longer clashes with Web Clipper's `source` URL
- `created` replaced by `inkwell-last-run-date` in `YYYY-MM-DDTHH:MM` (an Obsidian date-time); it was rewritten every run, so it never meant creation
- `citation` property drops Kindle's `Citation (Style): ` label; the Metadata section keeps the full text
- Default output colours the label in `obsidian` style and leaves the quoted passage plain; previously colours were plain text unless Painter was enabled
- `enable_painter_highlights` is deprecated: `true` still gives the old Painter label, and `[highlight_colours]` takes precedence
- Auto-created configs write `[highlight_colours]` instead of `enable_painter_highlights`

## [0.1.2] - 2026-09-06

### Added
- `My Clippings.txt` import (`.txt` input) for Kindle devices and CrossInk e-readers
  - Groups clippings by book, one Markdown note per book, rebuilt from the whole file each run
  - CrossInk chapter titles become section headings; Kindle files get a single "Highlights" section
  - Handles highlights, notes and bookmarks; exact duplicates within a book are dropped
  - Malformed or empty clippings are skipped with a warning instead of aborting
  - Frontmatter `source` is `kindle-clippings` or `crossink-clippings`
- Golden-file tests for HTML export and both clippings dialects

### Changed
- Location is now optional; entries without one print only the page number
- `--output` is rejected when the input contains more than one book
- Source ran through `cargo fmt`

### Fixed
- HTML export bookmarks were dropped unless Kindle emitted an empty text block after them; a bookmark followed by another entry, a section heading, or the end of the file is now kept

## [0.1.1] - 2025-11-02

### Added
- Comprehensive unit tests for all Kindle citation formats (MLA, APA, Chicago Style, None)
- Detailed error reporting with OS error codes and error kinds for file operations

### Changed
- Improved error messages to show technical details without verbose troubleshooting steps
- Updated documentation to clarify all citation formats are supported

### Fixed
- Error handling now provides clear diagnostic information for file access issues

## [0.1.0] - 2025-11-02

### Added
- Initial release of inkwell - Kindle to Obsidian Markdown converter
- HTML parser for Kindle macOS notebook exports
- Markdown generation with YAML frontmatter
- Support for all Kindle highlight colors (yellow, pink, blue, orange, green, aqua, red)
- Preservation of highlight metadata (page numbers, locations, subheadings)
- Support for notes and bookmarks
- Optional Obsidian Painter plugin integration for color-coded highlights (off by default)
- Configurable default export folder with multiple config file locations:
  - `./config.toml` (current directory)
  - `config.toml` in binary directory
  - `~/.config/inkwell/config.toml` (user config directory)
- CLI arguments for custom output paths and export directory overrides
- Automatic file naming based on author and title
- MLA citation format support from Kindle macOS exports
- TOML-based configuration system
- Comprehensive unit test suite (18 tests)
- Full cargo clippy compliance

### Features
- Clean Markdown output optimized for Obsidian
- Section-based organization matching book structure
- Blockquote formatting for highlighted passages
- Timestamp generation for export tracking
- Filename sanitization for cross-platform compatibility
- Graceful fallback to default configuration

### Documentation
- Complete README with installation and usage instructions
- Configuration examples and best practices
- Example output format
- Development guidelines
