//! Inline Obsidian `#tag` extraction — the second tag source layered on top of
//! okf's frontmatter `tags:` list (issue: Obsidian parity). See
//! `docs/features/inline-tags.md`.
//!
//! Obsidian unifies two tag mechanisms: frontmatter `tags:` and inline `#tag`
//! anywhere in the note body. okf only sees the frontmatter list, so a vault
//! that tags inline looks nearly untagged to okq. We parse the inline tags here,
//! *after* okf hands us the body, and merge them with the frontmatter tags in
//! [`crate::model::concept_tags`].
//!
//! Grammar (kept close to <https://help.obsidian.md/tags>, deliberately strict
//! to avoid false positives):
//!
//! - a tag starts at `#` only at **line start or after whitespace** (so `foo#bar`
//!   and a URL fragment `example.com/#section` are not tags);
//! - the run after `#` may contain Unicode letters/digits and `-`, `_`, `/`
//!   (nesting: `#area/work`), and must contain **at least one letter** (so `#123`
//!   and a Markdown heading `# Title` are not tags — the heading's `#` is followed
//!   by a space);
//! - a trailing `/` or `-` is trimmed (a tag can't end on a separator).
//!
//! Tags are lowercased (Obsidian matches case-insensitively) and returned in
//! document order with duplicates — deduplication happens when merging with
//! frontmatter tags. Like [`crate::wikilinks`], fenced code blocks and inline
//! code spans are skipped.

/// Extracts every inline `#tag` from a body, lowercased, in document order.
/// Fenced code blocks and inline code spans are skipped.
pub fn extract(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in crate::wikilinks::code_free_lines(body) {
        scan_line(&line, &mut out);
    }
    out
}

/// Scans one (code-free) line for `#tag` tokens.
fn scan_line(line: &str, out: &mut Vec<String>) {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // A `#` opens a tag only at line-start or right after whitespace.
        if chars[i] == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
            if let Some((tag, next)) = take_tag(&chars, i + 1) {
                out.push(tag);
                i = next;
                continue;
            }
        }
        i += 1;
    }
}

/// Reads a tag body starting just past the `#`. Returns the lowercased tag and
/// the index past the whole run, or `None` if the run is empty or contains no
/// letter (so it isn't a valid tag).
fn take_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut has_letter = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() {
            has_letter |= c.is_alphabetic();
            i += 1;
        } else if c == '-' || c == '_' || c == '/' {
            i += 1;
        } else {
            break;
        }
    }
    if i == start || !has_letter {
        return None;
    }
    // Trim trailing separators — a tag can't end on `/` or `-`.
    let mut end = i;
    while end > start && matches!(chars[end - 1], '/' | '-') {
        end -= 1;
    }
    if end == start {
        return None;
    }
    let tag: String = chars[start..end].iter().collect::<String>().to_lowercase();
    Some((tag, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_and_nested_tags() {
        assert_eq!(
            extract("A #simple tag and a #area/work nested one."),
            vec!["simple", "area/work"]
        );
    }

    #[test]
    fn line_start_tag() {
        assert_eq!(extract("#Obsidian\nsome text"), vec!["obsidian"]);
    }

    #[test]
    fn lowercased() {
        assert_eq!(extract("#KGPortal #Beef"), vec!["kgportal", "beef"]);
    }

    #[test]
    fn headings_are_not_tags() {
        // ATX headings: `#`/`##` followed by a space.
        assert!(extract("# Heading\n## Subheading\n### Deep").is_empty());
    }

    #[test]
    fn numeric_only_is_not_a_tag() {
        assert!(extract("issue #123 and #42").is_empty());
        assert_eq!(extract("#v2 and #2024/note"), vec!["v2", "2024/note"]);
    }

    #[test]
    fn mid_word_hash_is_not_a_tag() {
        assert!(extract("foo#bar and https://example.com/#section").is_empty());
    }

    #[test]
    fn punctuation_terminates() {
        assert_eq!(extract("(#tag) end. #other, next"), vec!["other"]);
        // `(#tag)` — `#` preceded by `(`, not whitespace → not a tag.
    }

    #[test]
    fn trailing_separators_trimmed() {
        assert_eq!(extract("#area/ and #done-"), vec!["area", "done"]);
    }

    #[test]
    fn code_is_ignored() {
        let body = "Inline `#notag` stays.\n\n```\n#alsonot\n```\n\nBut #real counts.\n";
        assert_eq!(extract(body), vec!["real"]);
    }

    #[test]
    fn duplicates_kept_in_order() {
        assert_eq!(extract("#a #b #a"), vec!["a", "b", "a"]);
    }

    #[test]
    fn empty_and_lone_hash() {
        assert!(extract("# \n#\na # b").is_empty());
    }
}
