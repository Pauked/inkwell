//! Parser for Kindle-device style `My Clippings.txt` files, as written by
//! physical Kindles and by CrossInk firmware on the Xteink X3.
//!
//! Each clipping is a block terminated by a line of ten `=`:
//!
//! ```text
//! Title (Author)
//! - Your Highlight on Page 1 | 1: Introduction | Added on Sunday, September 6, 2026, 10:52 AM
//!
//! Highlighted text
//! ==========
//! ```
//!
//! Kindles put `Location 123-125` where CrossInk puts the chapter title, and
//! also emit `Your Note` and `Your Bookmark` blocks. The file is append-only,
//! so the same book appears many times and deleted clippings are never removed.

use crate::parser::{Book, Bookmark, Entry, Highlight, Note, Section, Source};
use std::collections::{HashMap, HashSet};

const SEPARATOR: &str = "==========";
const DEFAULT_SECTION: &str = "Highlights";
const DEFAULT_COLOR: &str = "yellow";
const UNKNOWN_AUTHOR: &str = "Unknown Author";

/// Result of parsing a clippings file: the books found plus any warnings about
/// entries that were skipped.
#[derive(Debug, Default)]
pub struct ParsedClippings {
    pub books: Vec<Book>,
    pub warnings: Vec<String>,
}

/// One successfully parsed clipping, before grouping into books.
#[derive(Debug)]
struct Clipping {
    title: String,
    author: String,
    chapter: Option<String>,
    entry: Entry,
}

/// Parse a whole `My Clippings.txt`, grouping clippings into books by
/// (title, author) in order of first appearance, one section per chapter (or a
/// single "Highlights" section when the dialect has no chapters), with exact
/// duplicates within a book dropped.
pub fn parse_clippings(content: &str) -> ParsedClippings {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);

    let mut parsed = ParsedClippings::default();
    let mut book_index: HashMap<(String, String), usize> = HashMap::new();
    let mut seen: Vec<HashSet<DedupeKey>> = Vec::new();

    for (block_number, block) in content.split(SEPARATOR).enumerate() {
        let clipping = match parse_block(block) {
            Ok(Some(clipping)) => clipping,
            Ok(None) => continue,
            Err(reason) => {
                parsed
                    .warnings
                    .push(format!("Skipped clipping {}: {}", block_number + 1, reason));
                continue;
            }
        };

        let key = (clipping.title.clone(), clipping.author.clone());
        let index = *book_index.entry(key).or_insert_with(|| {
            parsed.books.push(Book {
                title: clipping.title.clone(),
                author: clipping.author.clone(),
                citation: String::new(),
                source: Source::CrossinkClippings,
                sections: Vec::new(),
            });
            seen.push(HashSet::new());
            parsed.books.len() - 1
        });

        if !seen[index].insert(DedupeKey::from(&clipping.entry)) {
            continue;
        }
        push_entry(&mut parsed.books[index], clipping.chapter, clipping.entry);
    }

    for book in &mut parsed.books {
        book.source = detect_source(book);
    }

    parsed
}

/// Identity of an entry for duplicate detection: text for highlights and
/// notes, position for bookmarks (which have no text).
#[derive(Debug, PartialEq, Eq, Hash)]
enum DedupeKey {
    Highlight(String),
    Note(String),
    Bookmark(Option<u32>, Option<u32>),
}

impl From<&Entry> for DedupeKey {
    fn from(entry: &Entry) -> Self {
        match entry {
            Entry::Highlight(h) => DedupeKey::Highlight(h.text.clone()),
            Entry::Note(n) => DedupeKey::Note(n.text.clone()),
            Entry::Bookmark(b) => DedupeKey::Bookmark(b.page, b.location),
        }
    }
}

fn push_entry(book: &mut Book, chapter: Option<String>, entry: Entry) {
    let heading = chapter.unwrap_or_else(|| DEFAULT_SECTION.to_string());
    match book.sections.iter_mut().find(|s| s.heading == heading) {
        Some(section) => section.entries.push(entry),
        None => book.sections.push(Section {
            heading,
            entries: vec![entry],
        }),
    }
}

/// A book whose entries carry Kindle Location numbers came from a real Kindle.
fn detect_source(book: &Book) -> Source {
    let has_location = book
        .sections
        .iter()
        .flat_map(|s| &s.entries)
        .any(|entry| match entry {
            Entry::Highlight(h) => h.location.is_some(),
            Entry::Note(n) => n.location.is_some(),
            Entry::Bookmark(b) => b.location.is_some(),
        });
    if has_location {
        Source::KindleClippings
    } else {
        Source::CrossinkClippings
    }
}

/// Parse one block. `Ok(None)` is a blank block (nothing to warn about);
/// `Err` is a block that looked like a clipping but could not be understood.
fn parse_block(block: &str) -> Result<Option<Clipping>, String> {
    let mut lines = block
        .lines()
        .map(str::trim_end)
        .skip_while(|l| l.trim().is_empty());

    let Some(title_line) = lines.next() else {
        return Ok(None);
    };
    let Some(meta_line) = lines.next() else {
        return Err(format!("\"{}\" has no metadata line", title_line.trim()));
    };

    let (title, author) = split_title_author(title_line.trim());
    let meta = parse_metadata(meta_line.trim())?;
    let text = collect_text(lines);

    let entry = match meta.kind {
        Kind::Highlight => {
            if text.is_empty() {
                return Err(format!("empty highlight in \"{}\"", title));
            }
            Entry::Highlight(Highlight {
                color: DEFAULT_COLOR.to_string(),
                page: meta.page,
                location: meta.location,
                subheading: None,
                text,
            })
        }
        Kind::Note => {
            if text.is_empty() {
                return Err(format!("empty note in \"{}\"", title));
            }
            Entry::Note(Note {
                page: meta.page,
                location: meta.location,
                subheading: None,
                text,
            })
        }
        Kind::Bookmark => Entry::Bookmark(Bookmark {
            page: meta.page,
            location: meta.location,
            subheading: None,
        }),
    };

    Ok(Some(Clipping {
        title,
        author,
        chapter: meta.chapter,
        entry,
    }))
}

/// Body text: the lines after the metadata line, with surrounding blank lines
/// removed and internal line breaks kept.
fn collect_text<'a>(lines: impl Iterator<Item = &'a str>) -> String {
    let body: Vec<&str> = lines.collect();
    let start = body.iter().position(|l| !l.trim().is_empty());
    let end = body.iter().rposition(|l| !l.trim().is_empty());
    match (start, end) {
        (Some(start), Some(end)) => body[start..=end].join("\n"),
        _ => String::new(),
    }
}

/// `Title (Author)` -> (`Title`, `Author`). The author is the trailing
/// parenthesised group, so titles with their own parentheses survive.
fn split_title_author(line: &str) -> (String, String) {
    let parsed = line.strip_suffix(')').and_then(|without_close| {
        let open = without_close.rfind('(')?;
        let title = without_close[..open].trim();
        let author = without_close[open + 1..].trim();
        (!title.is_empty() && !author.is_empty()).then(|| (title.to_string(), author.to_string()))
    });
    parsed.unwrap_or_else(|| (line.to_string(), UNKNOWN_AUTHOR.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Highlight,
    Note,
    Bookmark,
}

#[derive(Debug)]
struct Metadata {
    kind: Kind,
    page: Option<u32>,
    location: Option<u32>,
    chapter: Option<String>,
}

/// Parse `- Your Highlight on Page 1 | 1: Introduction | Added on ...`.
///
/// The segments between the kind and the trailing `Added on` timestamp are
/// each either a page, a Kindle location range, or (CrossInk) a chapter title.
fn parse_metadata(line: &str) -> Result<Metadata, String> {
    let rest = line
        .strip_prefix("- ")
        .and_then(|l| l.strip_prefix("Your "))
        .ok_or_else(|| format!("\"{}\" is not a metadata line", line))?;

    let (kind_word, after_kind) = rest
        .split_once(' ')
        .ok_or_else(|| format!("\"{}\" has no position", line))?;

    let kind = match kind_word {
        "Highlight" => Kind::Highlight,
        "Note" => Kind::Note,
        "Bookmark" => Kind::Bookmark,
        other => return Err(format!("unknown clipping type \"{}\"", other)),
    };

    // "on Page 1 | ..." (current firmware) or "at location 55 | ..." (older Kindles)
    let position = after_kind
        .strip_prefix("on ")
        .or_else(|| after_kind.strip_prefix("at "))
        .ok_or_else(|| format!("\"{}\" has no position", line))?;

    let segments = position
        .split(" | ")
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with("Added on"));

    let mut metadata = Metadata {
        kind,
        page: None,
        location: None,
        chapter: None,
    };
    for segment in segments {
        let lower = segment.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("page ") {
            metadata.page = leading_number(value);
        } else if let Some(value) = lower.strip_prefix("location ") {
            metadata.location = leading_number(value);
        } else {
            metadata.chapter = Some(segment.to_string());
        }
    }
    Ok(metadata)
}

/// First run of digits in `value` (`123-125` -> 123).
fn leading_number(value: &str) -> Option<u32> {
    value
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())
        .and_then(|s| s.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Source;

    const CROSSINK: &str = include_str!("../tests/fixtures/my-clippings-crossink.txt");
    const KINDLE: &str = include_str!("../tests/fixtures/my-clippings-kindle.txt");

    fn highlight(entry: &Entry) -> &crate::parser::Highlight {
        match entry {
            Entry::Highlight(h) => h,
            other => panic!("expected highlight, got {other:?}"),
        }
    }

    #[test]
    fn parses_crossink_fixture_into_one_book_with_chapter_section() {
        let parsed = parse_clippings(CROSSINK);

        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.books.len(), 1);
        let book = &parsed.books[0];
        assert_eq!(book.title, "A Philosophy of Software Design, 2nd Edition");
        assert_eq!(book.author, "John K. Ousterhout");
        assert_eq!(book.citation, "");
        assert_eq!(book.source, Source::CrossinkClippings);
        assert_eq!(book.sections.len(), 1);
        assert_eq!(book.sections[0].heading, "1: Introduction");
        assert_eq!(book.sections[0].entries.len(), 1);

        let h = highlight(&book.sections[0].entries[0]);
        assert_eq!(h.page, Some(1));
        assert_eq!(h.location, None);
        assert_eq!(h.subheading, None);
        assert_eq!(h.color, "yellow");
        assert_eq!(
            h.text,
            "Writing computer software is one of the purest creative activities in the history of the human race."
        );
    }

    #[test]
    fn parses_kindle_fixture_with_bom_crlf_notes_and_bookmarks() {
        let parsed = parse_clippings(KINDLE);

        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.books.len(), 2);

        let pragmatic = &parsed.books[0];
        assert_eq!(pragmatic.title, "The Pragmatic Programmer");
        assert_eq!(pragmatic.author, "David Thomas; Andrew Hunt");
        assert_eq!(pragmatic.source, Source::KindleClippings);
        assert_eq!(pragmatic.sections.len(), 1);
        assert_eq!(pragmatic.sections[0].heading, "Highlights");
        let entries = &pragmatic.sections[0].entries;
        assert_eq!(entries.len(), 4);

        let h = highlight(&entries[0]);
        assert_eq!(h.page, Some(12));
        assert_eq!(h.location, Some(123));
        assert!(h.text.starts_with("Care about your craft."));

        match &entries[1] {
            Entry::Note(n) => {
                assert_eq!(n.page, Some(12));
                assert_eq!(n.location, Some(125));
                assert_eq!(n.text, "This is the whole book in one line.");
            }
            other => panic!("expected note, got {other:?}"),
        }
        match &entries[2] {
            Entry::Bookmark(b) => {
                assert_eq!(b.page, Some(20));
                assert_eq!(b.location, Some(200));
            }
            other => panic!("expected bookmark, got {other:?}"),
        }
        let h = highlight(&entries[3]);
        assert_eq!(h.page, None);
        assert_eq!(h.location, Some(301));
        assert_eq!(h.text, "Don't live with broken windows.");

        let rust = &parsed.books[1];
        assert_eq!(rust.title, "Rust in Action (2nd ed.)");
        assert_eq!(rust.author, "Tim McNamara");
        let h = highlight(&rust.sections[0].entries[0]);
        assert_eq!(h.page, None);
        assert_eq!(h.location, Some(55));
    }

    #[test]
    fn groups_interleaved_books_and_keeps_file_order() {
        let content = "\
Book A (Author A)
- Your Highlight on Page 1 | Ch 1 | Added on Sunday, September 6, 2026, 10:52 AM

first a
==========
Book B (Author B)
- Your Highlight on Page 5 | Ch 9 | Added on Sunday, September 6, 2026, 10:53 AM

first b
==========
Book A (Author A)
- Your Highlight on Page 2 | Ch 1 | Added on Sunday, September 6, 2026, 10:54 AM

second a
==========
";
        let parsed = parse_clippings(content);

        assert_eq!(parsed.books.len(), 2);
        assert_eq!(parsed.books[0].title, "Book A");
        assert_eq!(parsed.books[1].title, "Book B");
        let a_texts: Vec<&str> = parsed.books[0].sections[0]
            .entries
            .iter()
            .map(|e| highlight(e).text.as_str())
            .collect();
        assert_eq!(a_texts, vec!["first a", "second a"]);
    }

    #[test]
    fn chapters_become_sections_in_order_of_first_appearance() {
        let content = "\
Book (Author)
- Your Highlight on Page 30 | 2: Second | Added on Sunday, September 6, 2026, 10:52 AM

two-a
==========
Book (Author)
- Your Highlight on Page 3 | 1: First | Added on Sunday, September 6, 2026, 10:53 AM

one-a
==========
Book (Author)
- Your Highlight on Page 31 | 2: Second | Added on Sunday, September 6, 2026, 10:54 AM

two-b
==========
Book (Author)
- Your Highlight on Page 40 | Added on Sunday, September 6, 2026, 10:55 AM

no chapter
==========
";
        let parsed = parse_clippings(content);

        let headings: Vec<&str> = parsed.books[0]
            .sections
            .iter()
            .map(|s| s.heading.as_str())
            .collect();
        assert_eq!(headings, vec!["2: Second", "1: First", "Highlights"]);
        assert_eq!(parsed.books[0].sections[0].entries.len(), 2);
        assert_eq!(parsed.books[0].sections[1].entries.len(), 1);
        assert_eq!(parsed.books[0].sections[2].entries.len(), 1);
    }

    #[test]
    fn dedupes_exact_duplicate_text_within_a_book_only() {
        let content = "\
Book (Author)
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:52 AM

same text
==========
Book (Author)
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:53 AM

same text
==========
Other (Author)
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:54 AM

same text
==========
Book (Author)
- Your Bookmark on page 9 | Location 90 | Added on Sunday, September 6, 2026, 10:55 AM


==========
Book (Author)
- Your Bookmark on page 9 | Location 90 | Added on Sunday, September 6, 2026, 10:56 AM


==========
";
        let parsed = parse_clippings(content);

        assert_eq!(parsed.books.len(), 2);
        let book_entries: usize = parsed.books[0]
            .sections
            .iter()
            .map(|s| s.entries.len())
            .sum();
        assert_eq!(book_entries, 2, "one highlight plus one bookmark");
        let other_entries: usize = parsed.books[1]
            .sections
            .iter()
            .map(|s| s.entries.len())
            .sum();
        assert_eq!(other_entries, 1);
    }

    #[test]
    fn skips_malformed_and_empty_entries_with_warnings() {
        let content = "\
==========
Book (Author)
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:52 AM

good one
==========
Book (Author)
this is not a metadata line

text
==========
Book (Author)
- Your Highlight on Page 2 | Ch | Added on Sunday, September 6, 2026, 10:53 AM


==========
Only a title line
==========

==========
Book (Author)
- Your Doodle on Page 2 | Ch | Added on Sunday, September 6, 2026, 10:53 AM

what
==========
";
        let parsed = parse_clippings(content);

        assert_eq!(parsed.books.len(), 1);
        assert_eq!(parsed.books[0].sections[0].entries.len(), 1);
        assert_eq!(parsed.warnings.len(), 4, "{:?}", parsed.warnings);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| w.contains("not a metadata line"))
        );
        assert!(parsed.warnings.iter().any(|w| w.contains("empty")));
        assert!(parsed.warnings.iter().any(|w| w.contains("Doodle")));
    }

    #[test]
    fn title_without_author_gets_placeholder_author() {
        let content = "\
Untitled Manuscript
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:52 AM

text
==========
";
        let parsed = parse_clippings(content);

        assert_eq!(parsed.books[0].title, "Untitled Manuscript");
        assert_eq!(parsed.books[0].author, "Unknown Author");
    }

    #[test]
    fn multi_line_highlight_text_is_preserved() {
        let content = "\
Book (Author)
- Your Highlight on Page 1 | Ch | Added on Sunday, September 6, 2026, 10:52 AM

line one
line two
==========
";
        let parsed = parse_clippings(content);

        let h = highlight(&parsed.books[0].sections[0].entries[0]);
        assert_eq!(h.text, "line one\nline two");
    }
}
