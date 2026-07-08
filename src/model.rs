//! Output types shared across commands.
//!
//! [`ConceptRecord`] is the locations-only concept envelope that every "list of
//! concepts" result reuses (`find` today; `search` and the graph commands
//! later), so a shortlist looks the same to an agent regardless of which
//! command produced it. It mirrors the per-concept envelope `get` ratified.

use std::collections::HashSet;

use okf::{Bundle, Concept, ConceptId, Frontmatter, Value};
use schemars::JsonSchema;
use serde::Serialize;

use crate::error::AppError;
use crate::view::Corpus;

/// Parses a caller-supplied identity into a [`ConceptId`] *syntactically*: a
/// `.md` suffix and a leading `./` are tolerated; the remainder must be a valid
/// concept id. This does not check existence — see [`resolve_concept`].
pub fn parse_concept_id(input: &str) -> Result<ConceptId, AppError> {
    let trimmed = input.trim_start_matches("./");
    let stripped = trimmed.strip_suffix(".md").unwrap_or(trimmed);
    ConceptId::parse(stripped).map_err(|e| AppError::InvalidConcept {
        input: input.to_string(),
        reason: e.to_string(),
    })
}

/// Resolves a caller-supplied identity to an **existing** concept. An exact
/// concept id (or `.md` path) always wins; otherwise a unique path-segment-
/// aligned *suffix* matches (so `0002-foo` finds `adrs/0002-foo`, and `foo`
/// finds a concept named `foo` in any directory). Matching is on `/` boundaries,
/// never arbitrary substrings. Failing all of that, a unique frontmatter
/// **alias** matches (case-insensitively) — the lowest-priority resolver, so a
/// real filename is never shadowed by another concept's alias (ADR-0011). A
/// non-unique partial or alias errors with the candidates.
pub fn resolve_concept(corpus: &Corpus, input: &str) -> Result<ConceptId, AppError> {
    // Filename resolution (exact id, then segment-suffix) always wins.
    if let Ok(parsed) = parse_concept_id(input) {
        if corpus.contains(&parsed) {
            return Ok(parsed);
        }
        let needle = parsed.segments();
        let mut matches: Vec<ConceptId> = corpus
            .concepts()
            .map(|c| c.id.clone())
            .filter(|id| ends_with_segments(id.segments(), needle))
            .collect();
        matches.sort();
        match matches.as_slice() {
            [] => {} // no filename match — fall through to aliases
            [one] => return Ok(one.clone()),
            many => {
                return Err(AppError::ConceptAmbiguous {
                    input: input.to_string(),
                    candidates: many.iter().map(ConceptId::to_string).collect(),
                });
            }
        }
    }

    // Alias fallback (ADR-0011): match the raw input against frontmatter aliases.
    let mut aliased = alias_matches(corpus, input);
    aliased.sort();
    aliased.dedup();
    match aliased.as_slice() {
        [one] => Ok(one.clone()),
        [] => {
            // Preserve the syntactic error if the input wasn't even a valid id;
            // otherwise it's a well-formed id that simply doesn't exist.
            parse_concept_id(input)?;
            Err(AppError::ConceptNotFound {
                input: input.to_string(),
            })
        }
        many => Err(AppError::ConceptAmbiguous {
            input: input.to_string(),
            candidates: many.iter().map(ConceptId::to_string).collect(),
        }),
    }
}

/// `true` if `segments` ends with `needle` (segment-aligned suffix).
fn ends_with_segments(segments: &[String], needle: &[String]) -> bool {
    segments.len() >= needle.len() && segments[segments.len() - needle.len()..] == *needle
}

/// The ids of visible concepts whose frontmatter `aliases:` include `input`
/// (case-insensitively). Empty input matches nothing.
fn alias_matches(corpus: &Corpus, input: &str) -> Vec<ConceptId> {
    let needle = input.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    corpus
        .concepts()
        .filter(|c| {
            concept_aliases(c)
                .iter()
                .any(|a| a.to_lowercase() == needle)
        })
        .map(|c| c.id.clone())
        .collect()
}

/// A concept's frontmatter `aliases:` — Obsidian's alternate note names. Accepts
/// both a YAML list (`aliases: [a, b]`) and a single scalar (`aliases: a`);
/// values are trimmed and empties dropped. These are resolution keys only, never
/// display titles (ADR-0011). See `docs/features/aliases.md`.
pub fn concept_aliases(c: &Concept) -> Vec<String> {
    alias_values(&c.document.frontmatter)
}

fn alias_values(fm: &Frontmatter) -> Vec<String> {
    let raw = match fm.get("aliases") {
        Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_display_string).collect(),
        Some(v) => v.as_display_string().into_iter().collect::<Vec<_>>(),
        None => Vec::new(),
    };
    raw.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// A concept's full tag set: frontmatter `tags:` (as written) unified with inline
/// body `#tags` (lowercased by the scanner). Obsidian treats the two as one tag
/// namespace; so does okq. Order is deterministic and author-faithful —
/// frontmatter tags in declaration order, then inline tags in document order —
/// deduplicated case-insensitively, so a frontmatter spelling wins over an inline
/// duplicate. See `docs/features/inline-tags.md`.
pub fn concept_tags(c: &Concept) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for t in c.document.frontmatter.tags() {
        if seen.insert(t.to_lowercase()) {
            out.push(t);
        }
    }
    for t in crate::tags::extract(&c.document.body) {
        if seen.insert(t.clone()) {
            out.push(t);
        }
    }
    out
}

/// The concept's display title: the frontmatter `title` if present and
/// non-empty, otherwise the concept's filename (its id's last segment),
/// **verbatim** — no humanizing. Frontmatter-less files are valid OKF concepts
/// (only `type` is required for conformance), and every concept has a non-empty
/// id segment, so a title is always available. This is a display value only;
/// the true frontmatter (see `get --frontmatter`) is never rewritten.
pub fn concept_title(c: &Concept) -> String {
    c.document
        .frontmatter
        .title()
        .unwrap_or_else(|| c.id.name().to_string())
}

/// One concept as it appears in a shortlist: identity, location, and the
/// frontmatter an agent needs to decide whether to expand it — never the body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ConceptRecord {
    /// The concept id (path minus `.md`).
    pub id: String,
    /// The frontmatter `type`, if present.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The concept's title: the frontmatter `title`, or the filename if none.
    pub title: String,
    /// The concept's path relative to the bundle root.
    pub path: String,
    /// 1-based line where the concept begins (always 1; match sites are `search`'s job).
    pub line: usize,
    /// The concept's tags (empty if none).
    pub tags: Vec<String>,
}

impl ConceptRecord {
    /// Builds a record from a loaded concept, with a bundle-relative path.
    pub fn from_concept(bundle: &Bundle, c: &Concept) -> Self {
        let rel = c.path.strip_prefix(bundle.root()).unwrap_or(&c.path);
        ConceptRecord {
            id: c.id.to_string(),
            type_: c.document.frontmatter.type_(),
            title: concept_title(c),
            path: rel.to_string_lossy().replace('\\', "/"),
            line: 1,
            tags: concept_tags(c),
        }
    }
}
