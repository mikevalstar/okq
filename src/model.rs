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

/// Resolves a caller-supplied identity into a [`ConceptId`]: a `.md` suffix and a
/// leading `./` are tolerated; the remainder must be a valid concept id. Shared
/// by the graph commands (and mirrors `get`'s resolution).
pub fn parse_concept_id(input: &str) -> Result<ConceptId, AppError> {
    let trimmed = input.trim_start_matches("./");
    let stripped = trimmed.strip_suffix(".md").unwrap_or(trimmed);
    ConceptId::parse(stripped).map_err(|e| AppError::InvalidConcept {
        input: input.to_string(),
        reason: e.to_string(),
    })
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
