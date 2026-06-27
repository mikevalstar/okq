//! Output types shared across commands.
//!
//! [`ConceptRecord`] is the locations-only concept envelope that every "list of
//! concepts" result reuses (`find` today; `search` and the graph commands
//! later), so a shortlist looks the same to an agent regardless of which
//! command produced it. It mirrors the per-concept envelope `get` ratified.

use okf::{Bundle, Concept, ConceptId};
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
/// never arbitrary substrings. A non-unique partial errors with the candidates.
pub fn resolve_concept(corpus: &Corpus, input: &str) -> Result<ConceptId, AppError> {
    let parsed = parse_concept_id(input)?;
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
        [] => Err(AppError::ConceptNotFound {
            input: input.to_string(),
        }),
        [one] => Ok(one.clone()),
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

/// One concept as it appears in a shortlist: identity, location, and the
/// frontmatter an agent needs to decide whether to expand it — never the body.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ConceptRecord {
    /// The concept id (path minus `.md`).
    pub id: String,
    /// The frontmatter `type`, if present.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The frontmatter `title`, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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
            title: c.document.frontmatter.title(),
            path: rel.to_string_lossy().replace('\\', "/"),
            line: 1,
            tags: c.document.frontmatter.tags(),
        }
    }
}
