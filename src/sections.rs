//! Heading-delimited section chunking over a concept's markdown body.
//!
//! okf parses a document into frontmatter + body but does not chunk by heading;
//! that is okq's job (ADR-0002). A *section* runs from a heading to the next
//! heading of the same or higher level, and carries the absolute `path:line`
//! at which it starts. Headings inside fenced code blocks are not headings —
//! pulldown-cmark already handles that for us.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// One heading-delimited section of a document body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The heading text (without leading `#`s).
    pub heading: String,
    /// A url-style slug derived from the heading (`"Open Questions"` → `open-questions`).
    pub slug: String,
    /// Heading depth, 1–6.
    pub level: u8,
    /// 1-based line in the source file where the heading sits.
    pub line: usize,
    /// The section's source markdown, from its heading to the next sibling-or-shallower heading.
    pub body: String,
}

fn level_to_u8(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Lowercases and hyphenates a heading into a slug; runs of non-alphanumerics
/// collapse to a single `-`, and leading/trailing `-` are trimmed.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.extend(c.to_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

/// The 1-based source line on which the document body begins, accounting for a
/// leading YAML frontmatter block (and the single blank line okf strips after
/// the closing `---`).
pub fn body_start_line(raw: &str) -> usize {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.first().map(|l| l.trim()) != Some("---") {
        return 1;
    }
    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim() == "---")
        .map(|(i, _)| i);
    match end {
        Some(end) => {
            let mut start = end + 1; // 0-based index of first body line
            if lines.get(start).map(|l| l.is_empty()).unwrap_or(false) {
                start += 1; // okf drops one blank line after the closing delimiter
            }
            start + 1 // back to 1-based
        }
        None => 1,
    }
}

struct Marker {
    offset: usize,
    level: u8,
    text: String,
}

/// Parses `body` into sections. `body_start_line` is the 1-based file line at
/// which `body` begins (see [`body_start_line`]), used to report absolute
/// `path:line` for each heading.
pub fn parse_sections(body: &str, body_start_line: usize) -> Vec<Section> {
    let mut markers: Vec<Marker> = Vec::new();
    let mut current: Option<Marker> = None;

    for (event, range) in Parser::new_ext(body, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Marker {
                    offset: range.start,
                    level: level_to_u8(level),
                    text: String::new(),
                });
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(m) = current.as_mut() {
                    m.text.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(m) = current.take() {
                    markers.push(m);
                }
            }
            _ => {}
        }
    }

    let mut sections = Vec::with_capacity(markers.len());
    for i in 0..markers.len() {
        let start = markers[i].offset;
        let level = markers[i].level;
        let end = markers[i + 1..]
            .iter()
            .find(|m| m.level <= level)
            .map(|m| m.offset)
            .unwrap_or(body.len());
        let line = body_start_line + body[..start].bytes().filter(|&b| b == b'\n').count();
        let heading = markers[i].text.clone();
        sections.push(Section {
            slug: slugify(&heading),
            heading,
            level,
            line,
            body: body[start..end].trim_end().to_string(),
        });
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Open Questions"), "open-questions");
        assert_eq!(slugify("  Schema!! "), "schema");
        assert_eq!(slugify("ADR-0002 — Stack"), "adr-0002-stack");
    }

    #[test]
    fn body_start_line_with_frontmatter() {
        let raw = "---\ntype: x\ntitle: y\n---\n\n# Heading\nbody\n";
        // lines: 0:--- 1:type 2:title 3:--- 4:(blank) 5:# Heading ...
        // body starts at file line 6 (1-based)
        assert_eq!(body_start_line(raw), 6);
    }

    #[test]
    fn body_start_line_no_frontmatter() {
        assert_eq!(body_start_line("# Heading\nbody\n"), 1);
    }

    #[test]
    fn sections_split_and_lines() {
        let body = "# Title\n\nintro\n\n## Schema\n\ncols\n\n## Notes\n\nmore\n";
        let secs = parse_sections(body, 6);
        assert_eq!(secs.len(), 3);
        assert_eq!(secs[0].heading, "Title");
        assert_eq!(secs[0].level, 1);
        assert_eq!(secs[0].line, 6);
        // "## Schema" is on body line index 4 → file line 6 + 4 = 10
        assert_eq!(secs[1].heading, "Schema");
        assert_eq!(secs[1].slug, "schema");
        assert_eq!(secs[1].level, 2);
        assert_eq!(secs[1].line, 10);
        assert!(secs[1].body.contains("cols"));
        // a level-2 section stops at the next level-2 heading
        assert!(!secs[1].body.contains("Notes"));
    }

    #[test]
    fn fenced_code_headings_are_not_sections() {
        let body = "# Real\n\n```\n# not a heading\n```\n";
        let secs = parse_sections(body, 1);
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].heading, "Real");
    }
}
