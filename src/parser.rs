use anyhow::{Context, Result};
use scraper::{ElementRef, Html, Selector};

/// Where a book's highlights came from. Written to the `source` frontmatter
/// field so notes from different pipelines can be told apart in Obsidian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Kindle macOS app "Export Notebook" HTML.
    KindleExport,
    /// `My Clippings.txt` written by a physical Kindle (has Location fields).
    KindleClippings,
    /// `My Clippings.txt` written by CrossInk firmware (chapter titles, no Locations).
    CrossinkClippings,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::KindleExport => "kindle-export",
            Source::KindleClippings => "kindle-clippings",
            Source::CrossinkClippings => "crossink-clippings",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub citation: String,
    pub source: Source,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub heading: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
pub enum Entry {
    Highlight(Highlight),
    Note(Note),
    Bookmark(Bookmark),
}

#[derive(Debug, Clone)]
pub struct Highlight {
    pub color: String,
    pub page: Option<u32>,
    pub location: Option<u32>,
    pub subheading: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub page: Option<u32>,
    pub location: Option<u32>,
    pub subheading: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub page: Option<u32>,
    pub location: Option<u32>,
    pub subheading: Option<String>,
}

pub fn parse_html(html_content: &str) -> Result<Book> {
    let document = Html::parse_document(html_content);

    // Extract metadata
    let title = extract_text(&document, ".bookTitle").context("Book title not found")?;
    let author = extract_text(&document, ".authors").context("Author not found")?;
    let citation = extract_text(&document, ".citation").unwrap_or_default();

    // Parse sections
    let sections = parse_sections(&document)?;

    Ok(Book {
        title,
        author,
        citation,
        source: Source::KindleExport,
        sections,
    })
}

fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

fn parse_sections(document: &Html) -> Result<Vec<Section>> {
    let body_selector = Selector::parse(".bodyContainer").unwrap();
    let body = document
        .select(&body_selector)
        .next()
        .context("Body container not found")?;

    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;
    // A noteHeading whose noteText has not arrived yet. Bookmarks never get a
    // noteText, so anything still pending when the next heading (or the end of
    // the document) arrives is flushed as a text-less entry.
    let mut pending_heading: Option<String> = None;

    for element in body.children() {
        let Some(elem_ref) = ElementRef::wrap(element) else {
            continue;
        };

        let class = elem_ref.value().attr("class").unwrap_or("");

        match class {
            "sectionHeading" => {
                flush_pending(&mut pending_heading, current_section.as_mut());
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                let heading = elem_ref.text().collect::<String>().trim().to_string();
                current_section = Some(Section {
                    heading,
                    entries: Vec::new(),
                });
            }
            "noteHeading" => {
                flush_pending(&mut pending_heading, current_section.as_mut());
                pending_heading = Some(elem_ref.text().collect());
            }
            "noteText" => {
                if let (Some(heading), Some(section)) =
                    (pending_heading.take(), current_section.as_mut())
                {
                    let text = elem_ref.text().collect::<String>().trim().to_string();
                    if let Ok(entry) = parse_entry(&heading, Some(text)) {
                        section.entries.push(entry);
                    }
                }
            }
            _ => {}
        }
    }

    flush_pending(&mut pending_heading, current_section.as_mut());
    if let Some(section) = current_section {
        sections.push(section);
    }

    Ok(sections)
}

/// Push a heading that never received a noteText into the section. Only
/// bookmarks are legitimately text-less; a highlight or note without text is
/// malformed and is dropped.
fn flush_pending(pending_heading: &mut Option<String>, section: Option<&mut Section>) {
    if let (Some(heading), Some(section)) = (pending_heading.take(), section)
        && let Ok(entry @ Entry::Bookmark(_)) = parse_entry(&heading, None)
    {
        section.entries.push(entry);
    }
}

fn parse_entry(heading_text: &str, text: Option<String>) -> Result<Entry> {
    let heading = heading_text.trim();

    if heading.starts_with("Highlight") {
        let color = extract_highlight_color(heading).unwrap_or_else(|| "yellow".to_string());
        let (page, location) = extract_page_location(heading)?;
        let subheading = extract_subheading(heading);

        Ok(Entry::Highlight(Highlight {
            color,
            page,
            location,
            subheading,
            text: text.unwrap_or_default(),
        }))
    } else if heading.starts_with("Note") {
        let (page, location) = extract_page_location(heading)?;
        let subheading = extract_subheading(heading);

        Ok(Entry::Note(Note {
            page,
            location,
            subheading,
            text: text.unwrap_or_default(),
        }))
    } else if heading.starts_with("Bookmark") {
        let (page, location) = extract_page_location(heading)?;
        let subheading = extract_subheading(heading);

        Ok(Entry::Bookmark(Bookmark {
            page,
            location,
            subheading,
        }))
    } else {
        anyhow::bail!("Unknown entry type: {}", heading)
    }
}

fn extract_highlight_color(text: &str) -> Option<String> {
    // Extract color from "Highlight(yellow)" or just from text content
    // Handle both "Highlight(yellow)" and text with color in it
    if let Some(start) = text.find('(')
        && let Some(end) = text.find(')')
    {
        return Some(text[start + 1..end].to_string());
    }

    // Fallback: look for color names in the text
    for color in &["yellow", "pink", "blue", "orange", "green", "aqua"] {
        if text.contains(color) {
            return Some(color.to_string());
        }
    }

    None
}

fn extract_page_location(text: &str) -> Result<(Option<u32>, Option<u32>)> {
    let mut page: Option<u32> = None;
    let mut location: Option<u32> = None;

    // Look for "Page XX" pattern anywhere in the text
    if let Some(page_pos) = text.find("Page ") {
        let after_page = &text[page_pos + 5..]; // Skip "Page "
        page = after_page
            .split(|c: char| !c.is_numeric())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse().ok());
    }

    // Look for "Location XX" pattern anywhere in the text
    if let Some(loc_pos) = text.find("Location ") {
        let after_location = &text[loc_pos + 9..]; // Skip "Location "
        location = Some(
            after_location
                .split(|c: char| !c.is_numeric())
                .find(|s| !s.is_empty())
                .and_then(|s| s.parse().ok())
                .context("Failed to parse location")?,
        );
    }

    Ok((page, location))
}

fn extract_subheading(text: &str) -> Option<String> {
    // Extract subheading from patterns like:
    // "Note - Getting stuck in organisations > Page 16"
    // "Highlight(yellow) - Self-driving people > Page 14 · Location 126"

    text.find(" - ").and_then(|dash_pos| {
        let after_dash = &text[dash_pos + 3..];

        // Look for " > " which indicates a subheading
        after_dash
            .find(" > ")
            .map(|gt_pos| after_dash[..gt_pos].trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_highlight_color() {
        assert_eq!(
            extract_highlight_color("Highlight(yellow)"),
            Some("yellow".to_string())
        );
        assert_eq!(
            extract_highlight_color("Highlight(pink)"),
            Some("pink".to_string())
        );
        assert_eq!(
            extract_highlight_color("Contains yellow word"),
            Some("yellow".to_string())
        );
    }

    #[test]
    fn test_extract_page_location_with_page() {
        let result = extract_page_location("Page 11 · Location 108").unwrap();
        assert_eq!(result, (Some(11), Some(108)));
    }

    #[test]
    fn test_extract_page_location_without_page() {
        let result = extract_page_location("Location 108").unwrap();
        assert_eq!(result, (None, Some(108)));
    }

    #[test]
    fn test_extract_page_location_complex() {
        // This simulates what we get from HTML with span tags
        let result = extract_page_location("Highlight(yellow) - Page 11 · Location 108").unwrap();
        assert_eq!(result, (Some(11), Some(108)));
    }

    #[test]
    fn test_extract_subheading() {
        let result =
            extract_subheading("Note - Getting stuck in organisations > Page 16 · Location 143");
        assert_eq!(result, Some("Getting stuck in organisations".to_string()));

        let result2 =
            extract_subheading("Highlight(yellow) - Self-driving people > Page 14 · Location 126");
        assert_eq!(result2, Some("Self-driving people".to_string()));

        let result3 = extract_subheading("Highlight(yellow) - Page 11 · Location 108");
        assert_eq!(result3, None);
    }

    #[test]
    fn test_parse_entry_highlight_with_page() {
        let heading = "Highlight(yellow) - Page 11 · Location 108";
        let text = Some("Some highlighted text".to_string());
        let entry = parse_entry(heading, text).unwrap();

        if let Entry::Highlight(h) = entry {
            assert_eq!(h.color, "yellow");
            assert_eq!(h.page, Some(11));
            assert_eq!(h.location, Some(108));
            assert_eq!(h.text, "Some highlighted text");
        } else {
            panic!("Expected Highlight entry");
        }
    }

    #[test]
    fn test_parse_entry_note_with_subheading() {
        let heading = "Note - Getting stuck in organisations > Page 16 · Location 143";
        let text = Some("Very true".to_string());
        let entry = parse_entry(heading, text).unwrap();

        if let Entry::Note(n) = entry {
            assert_eq!(n.page, Some(16));
            assert_eq!(n.location, Some(143));
            assert_eq!(
                n.subheading,
                Some("Getting stuck in organisations".to_string())
            );
            assert_eq!(n.text, "Very true");
        } else {
            panic!("Expected Note entry");
        }
    }

    #[test]
    fn test_html_text_extraction() {
        // Test that HTML with span tags extracts correctly
        let html = r#"<div class="noteHeading">
    Highlight(<span class="highlight_yellow">yellow</span>) - Page 11 · Location 108
</div>"#;

        let document = Html::parse_fragment(html);
        let selector = Selector::parse("div").unwrap();
        let text: String = document.select(&selector).next().unwrap().text().collect();

        println!("Extracted text: '{}'", text.trim());

        // Test parsing this text
        let (page, location) = extract_page_location(&text).unwrap();
        assert_eq!(page, Some(11));
        assert_eq!(location, Some(108));
    }

    #[test]
    fn test_parse_html_with_mla_citation() {
        let html = r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body>
    <div class="bodyContainer">
        <div class="bookTitle">Test Book</div>
        <div class="authors">Test Author</div>
        <div class="citation">Citation (MLA): Author, Test. <i>Test Book</i>. , 2025. Kindle file.</div>
        <div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteText">Test highlight text</div>
    </div>
</body>
</html>"#;

        let book = parse_html(html).unwrap();
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.author, "Test Author");
        assert_eq!(
            book.citation,
            "Citation (MLA): Author, Test. Test Book. , 2025. Kindle file."
        );
        assert_eq!(book.sections.len(), 1);
    }

    #[test]
    fn test_parse_html_with_apa_citation() {
        let html = r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body>
    <div class="bodyContainer">
        <div class="bookTitle">Test Book</div>
        <div class="authors">Test Author</div>
        <div class="citation">Citation (APA): Author, T. (2025). <i>Test Book</i> [Kindle iOS version]. Retrieved from Amazon.com</div>
        <div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteText">Test highlight text</div>
    </div>
</body>
</html>"#;

        let book = parse_html(html).unwrap();
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.author, "Test Author");
        assert_eq!(
            book.citation,
            "Citation (APA): Author, T. (2025). Test Book [Kindle iOS version]. Retrieved from Amazon.com"
        );
        assert_eq!(book.sections.len(), 1);
    }

    #[test]
    fn test_parse_html_with_chicago_citation() {
        let html = r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body>
    <div class="bodyContainer">
        <div class="bookTitle">Test Book</div>
        <div class="authors">Test Author</div>
        <div class="citation">Citation (Chicago Style): Author, Test. <i>Test Book</i>. , 2025. Kindle edition.</div>
        <div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteText">Test highlight text</div>
    </div>
</body>
</html>"#;

        let book = parse_html(html).unwrap();
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.author, "Test Author");
        assert_eq!(
            book.citation,
            "Citation (Chicago Style): Author, Test. Test Book. , 2025. Kindle edition."
        );
        assert_eq!(book.sections.len(), 1);
    }

    #[test]
    fn test_parse_html_with_no_citation() {
        let html = r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body>
    <div class="bodyContainer">
        <div class="bookTitle">Test Book</div>
        <div class="authors">Test Author</div>
        <div class="citation"></div>
        <div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteText">Test highlight text</div>
    </div>
</body>
</html>"#;

        let book = parse_html(html).unwrap();
        assert_eq!(book.title, "Test Book");
        assert_eq!(book.author, "Test Author");
        assert_eq!(book.citation, "");
        assert_eq!(book.sections.len(), 1);
    }
    fn html_with_entries(entries: &str) -> String {
        format!(
            r#"<html><body><div class="bodyContainer">
        <div class="bookTitle">Test Book</div>
        <div class="authors">Test Author</div>
        <div class="citation"></div>
        {entries}
    </div></body></html>"#
        )
    }

    fn entry_kinds(book: &Book) -> Vec<String> {
        book.sections
            .iter()
            .flat_map(|s| &s.entries)
            .map(|e| match e {
                Entry::Highlight(_) => "highlight".to_string(),
                Entry::Note(_) => "note".to_string(),
                Entry::Bookmark(b) => format!("bookmark@{}", b.location.unwrap_or(0)),
            })
            .collect()
    }

    #[test]
    fn test_bookmark_followed_by_another_heading_is_kept() {
        let html = html_with_entries(
            r#"<div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Bookmark - Page 31 · Location 307</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 400</div>
        <div class="noteText">After the bookmark</div>"#,
        );

        let book = parse_html(&html).unwrap();

        assert_eq!(entry_kinds(&book), vec!["bookmark@307", "highlight"]);
    }

    #[test]
    fn test_bookmark_before_section_heading_is_kept_in_its_own_section() {
        let html = html_with_entries(
            r#"<div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Bookmark - Page 31 · Location 307</div>
        <div class="sectionHeading">Chapter 2</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 400</div>
        <div class="noteText">In chapter two</div>"#,
        );

        let book = parse_html(&html).unwrap();

        assert_eq!(book.sections.len(), 2);
        assert_eq!(entry_kinds(&book), vec!["bookmark@307", "highlight"]);
        assert_eq!(
            book.sections[0].entries.len(),
            1,
            "bookmark stays in Chapter 1"
        );
    }

    #[test]
    fn test_bookmark_at_end_of_file_is_kept() {
        let html = html_with_entries(
            r#"<div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteText">Some text</div>
        <div class="noteHeading">Bookmark - Page 31 · Location 307</div>"#,
        );

        let book = parse_html(&html).unwrap();

        assert_eq!(entry_kinds(&book), vec!["highlight", "bookmark@307"]);
    }

    #[test]
    fn test_bookmark_with_empty_note_text_is_kept_once() {
        let html = html_with_entries(
            r#"<div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Bookmark - Page 31 · Location 307</div>
        <div class="noteText"></div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 400</div>
        <div class="noteText">After</div>"#,
        );

        let book = parse_html(&html).unwrap();

        assert_eq!(entry_kinds(&book), vec!["bookmark@307", "highlight"]);
    }
    #[test]
    fn test_highlight_heading_without_text_is_dropped_not_emitted_empty() {
        let html = html_with_entries(
            r#"<div class="sectionHeading">Chapter 1</div>
        <div class="noteHeading">Highlight(<span class="highlight_yellow">yellow</span>) - Location 100</div>
        <div class="noteHeading">Bookmark - Page 31 · Location 307</div>"#,
        );

        let book = parse_html(&html).unwrap();

        assert_eq!(entry_kinds(&book), vec!["bookmark@307"]);
    }
}
