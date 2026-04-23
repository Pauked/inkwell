# Changelog

All notable changes to this project will be documented in this file.

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
