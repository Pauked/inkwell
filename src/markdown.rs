use crate::parser::{Book, Entry, Highlight, Note, Bookmark};
use anyhow::Result;
use chrono::Local;

pub fn generate_markdown(book: &Book) -> Result<String> {
    let mut output = String::new();

    // Get current timestamp
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

    // Generate frontmatter
    output.push_str("---\n");
    output.push_str(&format!("title: \"{}\"\n", escape_yaml(&book.title)));
    output.push_str(&format!("author: \"{}\"\n", escape_yaml(&book.author)));
    output.push_str(&format!("citation: \"{}\"\n", escape_yaml(&book.citation)));
    output.push_str("source: kindle-export\n");
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
                    format_highlight(&mut output, highlight);
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

fn format_highlight(output: &mut String, highlight: &Highlight) {
    // Map Kindle colors to Obsidian Painter class suffixes
    let painter_class = match highlight.color.to_lowercase().as_str() {
        "yellow" => "y",
        "green" => "g",
        "pink" => "p",
        "blue" => "b",
        "red" => "r",
        "orange" => "o",
        "aqua" => "b", // Map aqua to blue as fallback
        _ => "y", // Default to yellow
    };

    // Format the highlight text as a blockquote
    output.push_str(&format!("> {}\n\n", highlight.text));

    // Add metadata line with colored highlight for the color name
    output.push_str("**Highlight** (");
    output.push_str(&format!(
        "<mark class=\"hltr-{}\">{}</mark>",
        painter_class,
        highlight.color
    ));
    output.push(')');

    if let Some(ref subheading) = highlight.subheading {
        output.push_str(&format!(" - *{}*", subheading));
    }

    if let Some(page) = highlight.page {
        output.push_str(&format!(" - Page {}", page));
    }

    output.push_str(&format!(" - Location {}", highlight.location));
    output.push_str("\n\n");

    output.push_str("---\n\n");
}

fn format_note(output: &mut String, note: &Note) {
    // Format the note text
    output.push_str(&format!("**Note**: {}\n", note.text));
    output.push('\n');

    // Add metadata line
    output.push('*');

    if let Some(ref subheading) = note.subheading {
        output.push_str(&format!("{} - ", subheading));
    }

    if let Some(page) = note.page {
        output.push_str(&format!("Page {} - ", page));
    }

    output.push_str(&format!("Location {}*", note.location));
    output.push_str("\n\n");

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

    output.push_str(&format!(" - Location {}", bookmark.location));
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
    fn test_format_highlight_yellow() {
        let highlight = Highlight {
            color: "yellow".to_string(),
            page: Some(11),
            location: 108,
            subheading: None,
            text: "This is a test highlight.".to_string(),
        };

        let mut output = String::new();
        format_highlight(&mut output, &highlight);

        assert!(output.contains("> This is a test highlight."));
        assert!(output.contains("<mark class=\"hltr-y\">yellow</mark>"));
        assert!(output.contains("Page 11"));
        assert!(output.contains("Location 108"));
        assert!(output.contains("---"));
    }

    #[test]
    fn test_format_highlight_pink() {
        let highlight = Highlight {
            color: "pink".to_string(),
            page: Some(15),
            location: 128,
            subheading: Some("Leaders don't make great followers".to_string()),
            text: "People like this tend to thrive.".to_string(),
        };

        let mut output = String::new();
        format_highlight(&mut output, &highlight);

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
                location: 100,
                subheading: None,
                text: "Test text".to_string(),
            };

            let mut output = String::new();
            format_highlight(&mut output, &highlight);

            let expected_markup = format!("<mark class=\"hltr-{}\">{}</mark>", expected_class, color_name);
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
            location: 143,
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
            location: 200,
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
            location: 307,
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
            sections: vec![],
        };

        let markdown = generate_markdown(&book).unwrap();

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
            sections: vec![Section {
                heading: "Chapter 1".to_string(),
                entries: vec![
                    Entry::Highlight(Highlight {
                        color: "yellow".to_string(),
                        page: Some(10),
                        location: 100,
                        subheading: None,
                        text: "First highlight".to_string(),
                    }),
                    Entry::Note(Note {
                        page: Some(10),
                        location: 101,
                        subheading: None,
                        text: "My note".to_string(),
                    }),
                ],
            }],
        };

        let markdown = generate_markdown(&book).unwrap();

        assert!(markdown.contains("### Chapter 1"));
        assert!(markdown.contains("> First highlight"));
        assert!(markdown.contains("<mark class=\"hltr-y\">yellow</mark>"));
        assert!(markdown.contains("**Note**: My note"));
    }
}
