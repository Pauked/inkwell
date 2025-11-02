use anyhow::{Context, Result};
use scraper::{Html, Selector, ElementRef};

#[derive(Debug, Clone)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub citation: String,
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
    pub location: u32,
    pub subheading: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub page: Option<u32>,
    pub location: u32,
    pub subheading: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub page: Option<u32>,
    pub location: u32,
    pub subheading: Option<String>,
}

pub fn parse_html(html_content: &str) -> Result<Book> {
    let document = Html::parse_document(html_content);

    // Extract metadata
    let title = extract_text(&document, ".bookTitle")
        .context("Book title not found")?;
    let author = extract_text(&document, ".authors")
        .context("Author not found")?;
    let citation = extract_text(&document, ".citation")
        .unwrap_or_default();

    // Parse sections
    let sections = parse_sections(&document)?;

    Ok(Book {
        title,
        author,
        citation,
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
    let body = document.select(&body_selector).next()
        .context("Body container not found")?;

    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut current_heading: Option<String> = None;
    let mut pending_note_text: bool = false;

    // Iterate through all children of body container
    for element in body.children() {
        let Some(elem_ref) = ElementRef::wrap(element) else {
            continue;
        };

        let class = elem_ref.value().attr("class").unwrap_or("");

        match class {
            "sectionHeading" => {
                // Save previous section if exists
                if let Some(section) = current_section.take() {
                    sections.push(section);
                }

                // Start new section
                let heading = elem_ref.text().collect::<String>().trim().to_string();
                current_section = Some(Section {
                    heading,
                    entries: Vec::new(),
                });
            }
            "noteHeading" => {
                // Store heading text for the next noteText
                let heading_text: String = elem_ref.text().collect();
                current_heading = Some(heading_text);
                pending_note_text = true;
            }
            "noteText" => {
                if let (Some(heading), Some(ref mut section)) =
                    (current_heading.take(), current_section.as_mut())
                {
                    let text = elem_ref.text().collect::<String>().trim().to_string();

                    if let Ok(entry) = parse_entry(&heading, Some(text)) {
                        section.entries.push(entry);
                    }
                    pending_note_text = false;
                }
            }
            _ => {
                // If we had a noteHeading but no noteText followed, it might be a bookmark
                if pending_note_text && class != "noteText"
                    && let (Some(heading), Some(ref mut section)) =
                        (current_heading.take(), current_section.as_mut())
                    && let Ok(entry) = parse_entry(&heading, None)
                {
                    section.entries.push(entry);
                    pending_note_text = false;
                } else if pending_note_text && class != "noteText" {
                    pending_note_text = false;
                }
            }
        }
    }

    // Save last section
    if let Some(section) = current_section {
        sections.push(section);
    }

    Ok(sections)
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

fn extract_page_location(text: &str) -> Result<(Option<u32>, u32)> {
    let mut page: Option<u32> = None;
    let mut location: u32 = 0;

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
        location = after_location
            .split(|c: char| !c.is_numeric())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .context("Failed to parse location")?;
    }

    Ok((page, location))
}

fn extract_subheading(text: &str) -> Option<String> {
    // Extract subheading from patterns like:
    // "Note - Getting stuck in organisations > Page 16"
    // "Highlight(yellow) - Self-driving people > Page 14 · Location 126"

    text.find(" - ")
        .and_then(|dash_pos| {
            let after_dash = &text[dash_pos + 3..];

            // Look for " > " which indicates a subheading
            after_dash.find(" > ").map(|gt_pos| {
                after_dash[..gt_pos].trim().to_string()
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_highlight_color() {
        assert_eq!(extract_highlight_color("Highlight(yellow)"), Some("yellow".to_string()));
        assert_eq!(extract_highlight_color("Highlight(pink)"), Some("pink".to_string()));
        assert_eq!(extract_highlight_color("Contains yellow word"), Some("yellow".to_string()));
    }

    #[test]
    fn test_extract_page_location_with_page() {
        let result = extract_page_location("Page 11 · Location 108").unwrap();
        assert_eq!(result, (Some(11), 108));
    }

    #[test]
    fn test_extract_page_location_without_page() {
        let result = extract_page_location("Location 108").unwrap();
        assert_eq!(result, (None, 108));
    }

    #[test]
    fn test_extract_page_location_complex() {
        // This simulates what we get from HTML with span tags
        let result = extract_page_location("Highlight(yellow) - Page 11 · Location 108").unwrap();
        assert_eq!(result, (Some(11), 108));
    }

    #[test]
    fn test_extract_subheading() {
        let result = extract_subheading("Note - Getting stuck in organisations > Page 16 · Location 143");
        assert_eq!(result, Some("Getting stuck in organisations".to_string()));

        let result2 = extract_subheading("Highlight(yellow) - Self-driving people > Page 14 · Location 126");
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
            assert_eq!(h.location, 108);
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
            assert_eq!(n.location, 143);
            assert_eq!(n.subheading, Some("Getting stuck in organisations".to_string()));
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
        assert_eq!(location, 108);
    }
}
