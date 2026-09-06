use crate::parser::{Book, Bookmark, Entry, Highlight, Note};
use anyhow::Result;
use chrono::Local;

/// Render a book to Markdown, stamping the frontmatter with the current time.
pub fn generate_markdown(book: &Book, enable_painter: bool) -> Result<String> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    render_markdown(book, enable_painter, &timestamp)
}

/// Render a book to Markdown with an explicit `created` timestamp (testable).
pub fn render_markdown(book: &Book, enable_painter: bool, timestamp: &str) -> Result<String> {
    let mut output = String::new();

    // Generate frontmatter
    output.push_str("---\n");
    output.push_str(&format!("title: \"{}\"\n", escape_yaml(&book.title)));
    output.push_str(&format!("author: \"{}\"\n", escape_yaml(&book.author)));
    output.push_str(&format!("citation: \"{}\"\n", escape_yaml(&book.citation)));
    output.push_str(&format!("source: {}\n", book.source.as_str()));
    output.push_str(&format!("created: \"{}\"\n", timestamp));
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
                    format_highlight(&mut output, highlight, enable_painter);
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

fn format_highlight(output: &mut String, highlight: &Highlight, enable_painter: bool) {
    // Format the highlight text as a blockquote
    output.push_str(&format!("> {}\n\n", highlight.text));

    // Add metadata line
    output.push_str("**Highlight** (");

    if enable_painter {
        // Map Kindle colors to Obsidian Painter class suffixes
        let painter_class = match highlight.color.to_lowercase().as_str() {
            "yellow" => "y",
            "green" => "g",
            "pink" => "p",
            "blue" => "b",
            "red" => "r",
            "orange" => "o",
            "aqua" => "b", // Map aqua to blue as fallback
            _ => "y",      // Default to yellow
        };

        output.push_str(&format!(
            "<mark class=\"hltr-{}\">{}</mark>",
            painter_class, highlight.color
        ));
    } else {
        output.push_str(&highlight.color);
    }

    output.push(')');

    if let Some(ref subheading) = highlight.subheading {
        output.push_str(&format!(" - *{}*", subheading));
    }

    if let Some(page) = highlight.page {
        output.push_str(&format!(" - Page {}", page));
    }

    if let Some(location) = highlight.location {
        output.push_str(&format!(" - Location {}", location));
    }
    output.push_str("\n\n");

    output.push_str("---\n\n");
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
        format_highlight(&mut output, &highlight, true);

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
        format_highlight(&mut output, &highlight, false);

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
        format_highlight(&mut output, &highlight, true);

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
            format_highlight(&mut output, &highlight, true);

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
    fn test_generate_markdown_frontmatter() {
        let book = Book {
            title: "Test Book".to_string(),
            author: "Test Author".to_string(),
            citation: "Test Citation".to_string(),
            source: crate::parser::Source::KindleExport,
            sections: vec![],
        };

        let markdown = generate_markdown(&book, false).unwrap();

        assert!(markdown.contains("---"));
        assert!(markdown.contains("title: \"Test Book\""));
        assert!(markdown.contains("author: \"Test Author\""));
        assert!(markdown.contains("citation: \"Test Citation\""));
        assert!(markdown.contains("source: kindle-export"));
        assert!(markdown.contains("created: \""));
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

        let markdown = generate_markdown(&book, true).unwrap();

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

        let markdown = render_markdown(&book, false, "2026-01-01 00:00:00").unwrap();

        assert_eq!(markdown, golden);
    }

    #[test]
    fn test_render_markdown_crossink_clippings_matches_golden() {
        let content = include_str!("../tests/fixtures/my-clippings-crossink.txt");
        let golden = include_str!("../tests/fixtures/my-clippings-crossink.golden.md");
        let parsed = crate::clippings::parse_clippings(content);

        let markdown = render_markdown(&parsed.books[0], false, "2026-01-01 00:00:00").unwrap();

        assert_eq!(markdown, golden);
    }

    #[test]
    fn test_render_markdown_kindle_clippings_matches_golden() {
        let content = include_str!("../tests/fixtures/my-clippings-kindle.txt");
        let golden = include_str!("../tests/fixtures/my-clippings-kindle.golden.md");
        let parsed = crate::clippings::parse_clippings(content);

        let markdown = render_markdown(&parsed.books[0], false, "2026-01-01 00:00:00").unwrap();

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
