use crate::config::{ColourStyle, HighlightColours, HighlightFormat, HighlightLayout};
use crate::parser::{Book, Bookmark, Entry, Highlight, Note};
use anyhow::Result;
use chrono::Local;

/// Render a book to Markdown, stamping the frontmatter with the current time.
pub fn generate_markdown(book: &Book, format: HighlightFormat) -> Result<String> {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M").to_string();
    render_markdown(book, format, &timestamp, env!("CARGO_PKG_VERSION"))
}

/// Render a book to Markdown with an explicit `inkwell-last-run-date` and
/// `inkwell-version` (testable).
pub fn render_markdown(
    book: &Book,
    format: HighlightFormat,
    timestamp: &str,
    version: &str,
) -> Result<String> {
    let mut output = String::new();

    // Generate frontmatter
    output.push_str("---\n");
    output.push_str(&format!("title: \"{}\"\n", escape_yaml(&book.title)));
    output.push_str(&format!("author: \"{}\"\n", escape_yaml(&book.author)));
    output.push_str(&format!(
        "citation: \"{}\"\n",
        escape_yaml(frontmatter_citation(&book.citation))
    ));
    output.push_str(&format!("inkwell-source: {}\n", book.source.as_str()));
    output.push_str(&format!(
        "inkwell-highlights-count: {}\n",
        highlight_count(book)
    ));
    output.push_str(&format!("inkwell-last-run-date: \"{}\"\n", timestamp));
    output.push_str(&format!("inkwell-version: {}\n", version));
    output.push_str("---\n\n");

    // Generate title
    output.push_str(&format!("# {}\n\n", book.title));

    // Generate metadata section
    output.push_str("## Metadata\n");
    output.push_str(&format!("* Author: {}\n", book.author));
    if !book.citation.is_empty() {
        output.push_str(&format!("* Citation: {}\n", book.citation));
    }
    output.push('\n');

    // Generate highlights and notes by section
    output.push_str("## Highlights\n\n");

    for section in &book.sections {
        // Section heading
        output.push_str(&format!("### {}\n\n", section.heading));

        for entry in &section.entries {
            match entry {
                Entry::Highlight(highlight) => {
                    format_highlight(&mut output, highlight, format);
                }
                Entry::Note(note) => {
                    format_note(&mut output, note);
                }
                Entry::Bookmark(bookmark) => {
                    format_bookmark(&mut output, bookmark);
                }
            }
        }
    }

    Ok(output)
}

/// The citation without Kindle's `Citation (Style): ` label, which reads as
/// noise in a property. The Metadata section keeps the full text.
fn frontmatter_citation(citation: &str) -> &str {
    citation
        .strip_prefix("Citation (")
        .and_then(|rest| rest.split_once("): "))
        .map_or(citation, |(_, text)| text)
}

fn highlight_count(book: &Book) -> usize {
    book.sections
        .iter()
        .flat_map(|section| &section.entries)
        .filter(|entry| matches!(entry, Entry::Highlight(_)))
        .count()
}

fn format_highlight(output: &mut String, highlight: &Highlight, format: HighlightFormat) {
    let text = quote_text(highlight, format.colours);
    let label = label_text(highlight, format.colours);
    let position = position_parts(highlight);

    match format.layout {
        HighlightLayout::Quote => {
            output.push_str(&format!("> {}\n\n", text));
            output.push_str(&format!("**Highlight** ({})", label));
            position
                .iter()
                .for_each(|part| output.push_str(&format!(" - {}", part)));
        }
        HighlightLayout::Line => {
            output.push_str(&format!("{} — {}", text, label));
            if !position.is_empty() {
                output.push_str(&format!(" | {}", position.join(" - ")));
            }
        }
    }

    output.push_str("\n\n---\n\n");
}

/// Subheading, page and location, whichever the highlight has.
fn position_parts(highlight: &Highlight) -> Vec<String> {
    [
        highlight.subheading.as_ref().map(|s| format!("*{}*", s)),
        highlight.page.map(|p| format!("Page {}", p)),
        highlight.location.map(|l| format!("Location {}", l)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The blockquote body, coloured when `text` is on.
fn quote_text(highlight: &Highlight, colours: HighlightColours) -> String {
    if colours.text {
        paint(&highlight.text, &highlight.color, colours.style)
    } else {
        highlight.text.clone()
    }
}

/// The colour name on the metadata line, coloured when `label` is on.
fn label_text(highlight: &Highlight, colours: HighlightColours) -> String {
    if colours.label {
        paint(&highlight.color, &highlight.color, colours.style)
    } else {
        highlight.color.clone()
    }
}

/// Wrap `content` in `color`. Obsidian style leaves text that already holds
/// `==` alone, since that would end the highlight early.
fn paint(content: &str, color: &str, style: ColourStyle) -> String {
    match style {
        ColourStyle::Obsidian if content.contains("==") => content.to_string(),
        ColourStyle::Obsidian => format!("=={}{}==", obsidian_color_emoji(color), content),
        ColourStyle::Painter => format!(
            "<mark class=\"hltr-{}\">{}</mark>",
            painter_class(color),
            content
        ),
    }
}

/// Obsidian Painter class suffix for a Kindle colour.
fn painter_class(color: &str) -> &'static str {
    match color.to_lowercase().as_str() {
        "green" => "g",
        "pink" => "p",
        "blue" | "aqua" => "b",
        "red" => "r",
        "orange" => "o",
        _ => "y",
    }
}

/// Obsidian's colour prefix for a Kindle colour. Yellow (and anything
/// unrecognised) gets none: a bare `==text==` is Obsidian's default yellow.
fn obsidian_color_emoji(color: &str) -> &'static str {
    match color.to_lowercase().as_str() {
        "red" => "🔴",
        "orange" => "🟠",
        "green" => "🟢",
        "blue" | "aqua" => "🔵",
        "pink" => "🟣",
        _ => "",
    }
}

fn format_note(output: &mut String, note: &Note) {
    // Format the note text
    output.push_str(&format!("**Note**: {}\n", note.text));
    output.push('\n');

    // Add metadata line (italic), skipped entirely when there is nothing to say
    let parts: Vec<String> = [
        note.subheading.clone(),
        note.page.map(|p| format!("Page {}", p)),
        note.location.map(|l| format!("Location {}", l)),
    ]
    .into_iter()
    .flatten()
    .collect();

    if !parts.is_empty() {
        output.push_str(&format!("*{}*\n\n", parts.join(" - ")));
    }

    output.push_str("---\n\n");
}

fn format_bookmark(output: &mut String, bookmark: &Bookmark) {
    output.push_str("**Bookmark**");

    if let Some(ref subheading) = bookmark.subheading {
        output.push_str(&format!(" - *{}*", subheading));
    }

    if let Some(page) = bookmark.page {
        output.push_str(&format!(" - Page {}", page));
    }

    if let Some(location) = bookmark.location {
        output.push_str(&format!(" - Location {}", location));
    }
    output.push_str("\n\n");

    output.push_str("---\n\n");
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Section;

    const PLAIN: HighlightColours = HighlightColours {
        style: ColourStyle::Obsidian,
        text: false,
        label: false,
    };

    const PAINTER_LABEL: HighlightColours = HighlightColours {
        style: ColourStyle::Painter,
        text: false,
        label: true,
    };

    const OBSIDIAN_BOTH: HighlightColours = HighlightColours {
        style: ColourStyle::Obsidian,
        text: true,
        label: true,
    };

    fn quote(colours: HighlightColours) -> HighlightFormat {
        HighlightFormat {
            layout: HighlightLayout::Quote,
            colours,
        }
    }

    fn line(colours: HighlightColours) -> HighlightFormat {
        HighlightFormat {
            layout: HighlightLayout::Line,
            colours,
        }
    }

    fn render_line(highlight: &Highlight) -> String {
        let mut output = String::new();
        format_highlight(&mut output, highlight, line(OBSIDIAN_BOTH));
        output
    }

    #[test]
    fn line_layout_puts_text_label_and_position_on_one_line() {
        let highlight = Highlight {
            color: "pink".to_string(),
            page: Some(14),
            location: Some(126),
            subheading: Some("Self-driving people".to_string()),
            text: "People like this tend to thrive.".to_string(),
        };
        assert_eq!(
            render_line(&highlight),
            "==🟣People like this tend to thrive.== — ==🟣pink== | *Self-driving people* - Page 14 - Location 126\n\n---\n\n"
        );
    }

    #[test]
    fn line_layout_omits_missing_position_parts() {
        let highlight = Highlight {
            location: Some(200),
            page: None,
            ..highlight_in("blue", "No page here.")
        };
        assert_eq!(
            render_line(&highlight),
            "==🔵No page here.== — ==🔵blue== | Location 200\n\n---\n\n"
        );
    }

    #[test]
    fn line_layout_without_position_ends_at_label() {
        let highlight = Highlight {
            page: None,
            ..highlight_in("yellow", "Nowhere.")
        };
        assert_eq!(
            render_line(&highlight),
            "==Nowhere.== — ==yellow==\n\n---\n\n"
        );
    }

    #[test]
    fn line_layout_with_colours_off_is_plain() {
        let mut output = String::new();
        format_highlight(&mut output, &highlight_in("pink", "Plain."), line(PLAIN));
        assert_eq!(output, "Plain. — pink | Page 3\n\n---\n\n");
    }

    #[test]
    fn line_layout_leaves_notes_and_bookmarks_unchanged() -> Result<()> {
        let book = Book {
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            citation: String::new(),
            source: crate::parser::Source::KindleExport,
            sections: vec![Section {
                heading: "Chapter".to_string(),
                entries: vec![
                    Entry::Note(Note {
                        page: Some(16),
                        location: Some(143),
                        subheading: None,
                        text: "My note.".to_string(),
                    }),
                    Entry::Bookmark(Bookmark {
                        page: Some(17),
                        location: Some(150),
                        subheading: None,
                    }),
                ],
            }],
        };
        let quoted = render_markdown(&book, quote(OBSIDIAN_BOTH), "t", "0.0.0")?;
        let lined = render_markdown(&book, line(OBSIDIAN_BOTH), "t", "0.0.0")?;
        assert_eq!(quoted, lined);
        Ok(())
    }

    #[test]
    fn test_format_highlight_yellow_with_painter() {
        let highlight = Highlight {
            color: "yellow".to_string(),
            page: Some(11),
            location: Some(108),
            subheading: None,
            text: "This is a test highlight.".to_string(),
        };

        let mut output = String::new();
        format_highlight(&mut output, &highlight, quote(PAINTER_LABEL));

        assert!(output.contains("> This is a test highlight."));
        assert!(output.contains("<mark class=\"hltr-y\">yellow</mark>"));
        assert!(output.contains("Page 11"));
        assert!(output.contains("Location 108"));
        assert!(output.contains("---"));
    }

    #[test]
    fn test_format_highlight_yellow_without_painter() {
        let highlight = Highlight {
            color: "yellow".to_string(),
            page: Some(11),
            location: Some(108),
            subheading: None,
            text: "This is a test highlight.".to_string(),
        };

        let mut output = String::new();
        format_highlight(&mut output, &highlight, quote(PLAIN));

        assert!(output.contains("> This is a test highlight."));
        assert!(output.contains("**Highlight** (yellow)"));
        assert!(!output.contains("<mark"));
        assert!(output.contains("Page 11"));
        assert!(output.contains("Location 108"));
    }

    #[test]
    fn test_format_highlight_pink() {
        let highlight = Highlight {
            color: "pink".to_string(),
            page: Some(15),
            location: Some(128),
            subheading: Some("Leaders don't make great followers".to_string()),
            text: "People like this tend to thrive.".to_string(),
        };

        let mut output = String::new();
        format_highlight(&mut output, &highlight, quote(PAINTER_LABEL));

        assert!(output.contains("> People like this tend to thrive."));
        assert!(output.contains("<mark class=\"hltr-p\">pink</mark>"));
        assert!(output.contains("*Leaders don't make great followers*"));
        assert!(output.contains("Page 15"));
        assert!(output.contains("Location 128"));
    }

    #[test]
    fn test_format_highlight_all_colors() {
        let test_colors = vec![
            ("yellow", "y"),
            ("green", "g"),
            ("pink", "p"),
            ("blue", "b"),
            ("red", "r"),
            ("orange", "o"),
            ("aqua", "b"), // Maps to blue
        ];

        for (color_name, expected_class) in test_colors {
            let highlight = Highlight {
                color: color_name.to_string(),
                page: None,
                location: Some(100),
                subheading: None,
                text: "Test text".to_string(),
            };

            let mut output = String::new();
            format_highlight(&mut output, &highlight, quote(PAINTER_LABEL));

            let expected_markup = format!(
                "<mark class=\"hltr-{}\">{}</mark>",
                expected_class, color_name
            );
            assert!(
                output.contains(&expected_markup),
                "Expected '{}' for color '{}', got: {}",
                expected_markup,
                color_name,
                output
            );
        }
    }

    fn highlight_in(color: &str, text: &str) -> Highlight {
        Highlight {
            color: color.to_string(),
            page: Some(3),
            location: None,
            subheading: None,
            text: text.to_string(),
        }
    }

    fn render(colours: HighlightColours, color: &str, text: &str) -> String {
        let mut output = String::new();
        format_highlight(&mut output, &highlight_in(color, text), quote(colours));
        output
    }

    #[test]
    fn obsidian_text_and_label_use_color_emoji() {
        let cases = [
            ("orange", "🟠"),
            ("green", "🟢"),
            ("blue", "🔵"),
            ("aqua", "🔵"),
            ("pink", "🟣"),
            ("red", "🔴"),
            ("Blue", "🔵"),
        ];

        for (color, emoji) in cases {
            let output = render(OBSIDIAN_BOTH, color, "Test text");
            let quote = format!("> =={}Test text==\n", emoji);
            let label = format!("**Highlight** (=={}{}==) - Page 3", emoji, color);
            assert!(output.starts_with(&quote), "{}: {}", color, output);
            assert!(output.contains(&label), "{}: {}", color, output);
        }
    }

    #[test]
    fn obsidian_yellow_and_unknown_colors_get_no_emoji() {
        for color in ["yellow", "chartreuse"] {
            let output = render(OBSIDIAN_BOTH, color, "Test text");
            assert!(output.starts_with("> ==Test text==\n"), "{}", output);
            let label = format!("**Highlight** (=={}==)", color);
            assert!(output.contains(&label), "{}", output);
        }
    }

    #[test]
    fn obsidian_label_only_leaves_quote_plain() {
        let colours = HighlightColours {
            text: false,
            ..OBSIDIAN_BOTH
        };
        let output = render(colours, "pink", "Test text");
        assert!(output.starts_with("> Test text\n"), "{}", output);
        assert!(output.contains("**Highlight** (==🟣pink==)"), "{}", output);
    }

    #[test]
    fn obsidian_text_only_leaves_label_plain() {
        let colours = HighlightColours {
            label: false,
            ..OBSIDIAN_BOTH
        };
        let output = render(colours, "pink", "Test text");
        assert!(output.starts_with("> ==🟣Test text==\n"), "{}", output);
        assert!(
            output.contains("**Highlight** (pink) - Page 3"),
            "{}",
            output
        );
    }

    #[test]
    fn painter_text_wraps_quote_in_mark() {
        let colours = HighlightColours {
            style: ColourStyle::Painter,
            text: true,
            label: true,
        };
        let output = render(colours, "pink", "Test text");
        assert!(
            output.starts_with("> <mark class=\"hltr-p\">Test text</mark>\n"),
            "{}",
            output
        );
        assert!(
            output.contains("(<mark class=\"hltr-p\">pink</mark>)"),
            "{}",
            output
        );
    }

    #[test]
    fn plain_colours_leave_quote_and_label_unmarked() {
        let output = render(PLAIN, "pink", "Test text");
        assert!(output.starts_with("> Test text\n"), "{}", output);
        assert!(output.contains("**Highlight** (pink)"), "{}", output);
        assert!(
            !output.contains("==") && !output.contains("<mark"),
            "{}",
            output
        );
    }

    #[test]
    fn obsidian_skips_wrapping_text_containing_highlight_marker() {
        let output = render(OBSIDIAN_BOTH, "blue", "if a == b then");
        assert!(output.starts_with("> if a == b then\n"), "{}", output);
        assert!(output.contains("(==🔵blue==)"), "{}", output);
    }

    #[test]
    fn test_format_note_with_page() {
        let note = Note {
            page: Some(16),
            location: Some(143),
            subheading: Some("Getting stuck in organisations".to_string()),
            text: "Very true".to_string(),
        };

        let mut output = String::new();
        format_note(&mut output, &note);

        assert!(output.contains("**Note**: Very true"));
        assert!(output.contains("*Getting stuck in organisations - Page 16 - Location 143*"));
        assert!(output.contains("---"));
    }

    #[test]
    fn test_format_note_without_page() {
        let note = Note {
            page: None,
            location: Some(200),
            subheading: None,
            text: "My thoughts".to_string(),
        };

        let mut output = String::new();
        format_note(&mut output, &note);

        assert!(output.contains("**Note**: My thoughts"));
        assert!(output.contains("*Location 200*"));
        assert!(!output.contains("Page"));
    }

    #[test]
    fn test_format_bookmark() {
        let bookmark = Bookmark {
            page: Some(31),
            location: Some(307),
            subheading: Some("Understanding your risk exposure".to_string()),
        };

        let mut output = String::new();
        format_bookmark(&mut output, &bookmark);

        assert!(output.contains("**Bookmark**"));
        assert!(output.contains("*Understanding your risk exposure*"));
        assert!(output.contains("Page 31"));
        assert!(output.contains("Location 307"));
    }

    #[test]
    fn test_escape_yaml() {
        assert_eq!(escape_yaml("simple"), "simple");
        assert_eq!(escape_yaml("has \"quotes\""), "has \\\"quotes\\\"");
        assert_eq!(escape_yaml("has\\backslash"), "has\\\\backslash");
        assert_eq!(escape_yaml("has\nnewline"), "has newline");
        assert_eq!(
            escape_yaml("complex: \"test\\value\"\nwith newline"),
            "complex: \\\"test\\\\value\\\" with newline"
        );
    }

    #[test]
    fn frontmatter_citation_drops_style_label() {
        assert_eq!(
            frontmatter_citation("Citation (Chicago Style): Arundel, John. Master."),
            "Arundel, John. Master."
        );
        assert_eq!(
            frontmatter_citation("Citation (APA): Author, T. (2025)."),
            "Author, T. (2025)."
        );
        assert_eq!(frontmatter_citation("Test Citation"), "Test Citation");
        assert_eq!(frontmatter_citation(""), "");
    }

    #[test]
    fn test_generate_markdown_frontmatter() {
        let book = Book {
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            citation: "Test Citation".to_string(),
            source: crate::parser::Source::KindleExport,
            sections: vec![],
        };

        let markdown = generate_markdown(&book, quote(PLAIN)).unwrap();

        assert!(markdown.contains("---"));
        assert!(markdown.contains("title: \"Test Book\""));
        assert!(markdown.contains("author: \"Test Author\""));
        assert!(markdown.contains("citation: \"Test Citation\""));
        assert!(markdown.contains("\ninkwell-source: kindle-export\n"));
        assert!(markdown.contains("\ninkwell-highlights-count: 0\n"));
        assert!(!markdown.contains("\nsource:") && !markdown.contains("\ncreated:"));
        let run_date = markdown
            .lines()
            .find_map(|line| line.strip_prefix("inkwell-last-run-date: "))
            .unwrap_or_default();
        assert!(
            chrono::NaiveDateTime::parse_from_str(run_date, "\"%Y-%m-%dT%H:%M\"").is_ok(),
            "{run_date}"
        );
        let version_line = format!("\ninkwell-version: {}\n", env!("CARGO_PKG_VERSION"));
        assert!(markdown.contains(&version_line), "{markdown}");
    }

    #[test]
    fn test_generate_markdown_with_highlights() {
        let book = Book {
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            citation: "".to_string(),
            source: crate::parser::Source::KindleExport,
            sections: vec![Section {
                heading: "Chapter 1".to_string(),
                entries: vec![
                    Entry::Highlight(Highlight {
                        color: "yellow".to_string(),
                        page: Some(10),
                        location: Some(100),
                        subheading: None,
                        text: "First highlight".to_string(),
                    }),
                    Entry::Note(Note {
                        page: Some(10),
                        location: Some(101),
                        subheading: None,
                        text: "My note".to_string(),
                    }),
                ],
            }],
        };

        let markdown = generate_markdown(&book, quote(PAINTER_LABEL)).unwrap();

        assert!(markdown.contains("### Chapter 1"));
        assert!(markdown.contains("> First highlight"));
        assert!(markdown.contains("<mark class=\"hltr-y\">yellow</mark>"));
        assert!(markdown.contains("**Note**: My note"));
    }
    #[test]
    fn test_render_markdown_html_export_matches_golden() {
        let html = include_str!("../tests/fixtures/kindle-export.html");
        let golden = include_str!("../tests/fixtures/kindle-export.golden.md");
        let book = crate::parser::parse_html(html).unwrap();

        let markdown = render_markdown(&book, quote(PLAIN), "2026-01-01T00:00", "0.0.0").unwrap();

        assert_eq!(markdown, golden);
    }

    #[test]
    fn test_render_markdown_crossink_clippings_matches_golden() {
        let content = include_str!("../tests/fixtures/my-clippings-crossink.txt");
        let golden = include_str!("../tests/fixtures/my-clippings-crossink.golden.md");
        let parsed = crate::clippings::parse_clippings(content);

        let markdown =
            render_markdown(&parsed.books[0], quote(PLAIN), "2026-01-01T00:00", "0.0.0").unwrap();

        assert_eq!(markdown, golden);
    }

    #[test]
    fn test_render_markdown_kindle_clippings_matches_golden() {
        let content = include_str!("../tests/fixtures/my-clippings-kindle.txt");
        let golden = include_str!("../tests/fixtures/my-clippings-kindle.golden.md");
        let parsed = crate::clippings::parse_clippings(content);

        let markdown =
            render_markdown(&parsed.books[0], quote(PLAIN), "2026-01-01T00:00", "0.0.0").unwrap();

        assert_eq!(markdown, golden);
    }

    #[test]
    fn test_format_note_without_location_prints_only_page() {
        let note = Note {
            page: Some(16),
            location: None,
            subheading: None,
            text: "Just a page".to_string(),
        };

        let mut output = String::new();
        format_note(&mut output, &note);

        assert!(output.contains("*Page 16*"), "{output}");
        assert!(!output.contains("Location"));
    }

    #[test]
    fn test_format_bookmark_without_location_prints_only_page() {
        let bookmark = Bookmark {
            page: Some(31),
            location: None,
            subheading: None,
        };

        let mut output = String::new();
        format_bookmark(&mut output, &bookmark);

        assert!(output.contains("**Bookmark** - Page 31\n"), "{output}");
        assert!(!output.contains("Location"));
    }
}
