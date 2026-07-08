//! Obsidian-style wikilink extraction — the extra edge source layered on top of
//! okf's CommonMark links (issue #5).
//!
//! okf only understands `[text](dest)` links. Many knowledge bases — Obsidian
//! vaults in particular — also cross-link with `[[wikilinks]]`. We parse those
//! here, *after* okf hands us the concept body, and feed the results into the
//! [`crate::graph`] as an extra edge source. Resolution is deliberately lenient
//! (issue #5): we accept the whole spread of shapes Obsidian permits and match
//! bare note names case-insensitively.
//!
//! Shapes covered (see <https://help.obsidian.md/links>):
//!
//! | written | note target we extract |
//! |---------|------------------------|
//! | `[[Note]]` | `Note` |
//! | `[[Note\|Alias]]` | `Note` (display alias dropped) |
//! | `[[folder/Note]]` | `folder/Note` (vault-relative path) |
//! | `[[Note.md]]` | `Note` (`.md` tolerated) |
//! | `[[Note#Heading]]` | `Note` (heading dropped) |
//! | `[[Note#^block-id]]` | `Note` (block ref dropped) |
//! | `[[Note#Heading\|Alias]]` | `Note` |
//! | `[[#Heading]]`, `[[#^id]]` | *(same-note ref — no edge)* |
//! | `![[Note]]`, `![[Note#…]]` | `Note` (embed / transclusion) |
//!
//! Embeds (`![[…]]`) need no special handling: the `[[…]]` they contain is
//! scanned like any other wikilink, so a transclusion becomes a reference edge.
//!
//! Like okf's own link scanner this is dependency-free and skips fenced code
//! blocks and inline code spans — a `[[x]]` shown as code is content, not a link.

/// A wikilink found in a concept body, reduced to the note it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wikilink {
    /// The note target: alias and `#heading`/`#^block` anchors stripped, a
    /// trailing `.md` removed, trimmed. Empty targets (`[[#heading]]`,
    /// same-note references) are never emitted.
    pub target: String,
}

/// Extracts every `[[wikilink]]` (and `![[embed]]`) from a body, skipping fenced
/// code blocks and inline code spans, in document order.
pub fn extract(body: &str) -> Vec<Wikilink> {
    let mut out = Vec::new();
    for line in code_free_lines(body) {
        scan_line(&line, &mut out);
    }
    out
}

/// Returns the body's lines with fenced code blocks removed and inline code
/// spans blanked out (mirrors okf's `links::code_free_lines`). Shared with
/// [`crate::tags`], which scans the same code-free view of the body for `#tags`.
pub(crate) fn code_free_lines(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fence: Option<char> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(f) = fence {
            if trimmed.starts_with(&f.to_string().repeat(3)) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            fence = Some('`');
            continue;
        }
        if trimmed.starts_with("~~~") {
            fence = Some('~');
            continue;
        }
        out.push(blank_inline_code(line));
    }
    out
}

/// Replaces inline code spans (backtick-delimited) with spaces so wikilinks
/// inside them are not extracted.
fn blank_inline_code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
            out.push(' ');
        } else if in_code {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Scans a single (code-free) line for `[[…]]` spans.
fn scan_line(line: &str, out: &mut Vec<Wikilink>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '[' && chars[i + 1] == '[' {
            if let Some((inner, next)) = take_until_close(&chars, i + 2) {
                if let Some(target) = note_target(&inner) {
                    out.push(Wikilink { target });
                }
                i = next;
                continue;
            }
        }
        i += 1;
    }
}

/// Reads the wikilink body starting just past the opening `[[`, returning the
/// inner text and the index just past the closing `]]`. `None` if unterminated.
fn take_until_close(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == ']' && chars[i + 1] == ']' {
            let inner: String = chars[start..i].iter().collect();
            return Some((inner, i + 2));
        }
        // A wikilink never spans a `[[` — bail so `[[a]] [[b]]` reads as two.
        if chars[i] == '[' && chars[i + 1] == '[' {
            return None;
        }
        i += 1;
    }
    None
}

/// Reduces a wikilink's inner text to the note it targets: drop the `|alias`,
/// drop the `#heading`/`#^block` anchor, tolerate a `.md` suffix, trim. Returns
/// `None` for a same-note reference (`[[#heading]]`) or an empty/external target.
fn note_target(inner: &str) -> Option<String> {
    let note = inner.split('|').next().unwrap_or(inner);
    let note = note.split('#').next().unwrap_or(note).trim();
    if note.is_empty() || note.contains("://") {
        return None;
    }
    let note = note.strip_suffix(".md").unwrap_or(note).trim_end();
    if note.is_empty() {
        return None;
    }
    Some(note.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(body: &str) -> Vec<String> {
        extract(body).into_iter().map(|w| w.target).collect()
    }

    #[test]
    fn bare_and_aliased_and_pathed() {
        assert_eq!(
            targets("See [[Note]], [[Other|shown]], and [[folder/Deep]]."),
            vec!["Note", "Other", "folder/Deep"]
        );
    }

    #[test]
    fn anchors_and_blocks_are_stripped() {
        assert_eq!(
            targets("[[Note#Heading]] [[Note#^block-id]] [[Note#Heading|Alias]]"),
            vec!["Note", "Note", "Note"]
        );
    }

    #[test]
    fn embeds_are_links_too() {
        assert_eq!(
            targets("![[Note]] and ![[img/Diagram#Overview]]"),
            vec!["Note", "img/Diagram"]
        );
    }

    #[test]
    fn md_suffix_and_spaces_tolerated() {
        assert_eq!(
            targets("[[My Note.md]] [[  Spaced  |x]]"),
            vec!["My Note", "Spaced"]
        );
    }

    #[test]
    fn same_note_and_empty_refs_skipped() {
        assert!(targets("[[#Heading]] [[#^block]] [[]] [[ ]]").is_empty());
    }

    #[test]
    fn code_is_ignored() {
        let body =
            "Inline `[[NotALink]]` stays.\n\n```\n[[AlsoNot]]\n```\n\nBut [[Real]] counts.\n";
        assert_eq!(targets(body), vec!["Real"]);
    }

    #[test]
    fn adjacent_links_read_separately() {
        assert_eq!(targets("[[a]] [[b]][[c]]"), vec!["a", "b", "c"]);
    }

    #[test]
    fn unterminated_is_not_a_link() {
        assert!(targets("a [[ dangling with no close").is_empty());
    }

    #[test]
    fn external_targets_skipped() {
        assert!(targets("[[https://example.com|site]]").is_empty());
    }
}
