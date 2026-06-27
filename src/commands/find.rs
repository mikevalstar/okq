//! `okq find` — filter concepts by frontmatter/content predicates.
//!
//! Set membership, not ranking: a concept is returned iff it satisfies every
//! supplied predicate. Results are locations-only (the shared concept envelope)
//! in deterministic concept-id order. See `docs/features/find.md`.

use std::io::Write;
use std::path::Path;

use okf::{Concept, Frontmatter, Value};
use regex::Regex;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::FindArgs;
use crate::error::AppError;
use crate::model::ConceptRecord;
use crate::view::Corpus;

/// Schema tag stamped on every `find` JSON document.
pub const SCHEMA: &str = "okq.find/v1";

/// The `okq.find/v1` collection envelope — the list shape reused by `search`
/// and the graph list-commands.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FindOutput {
    /// Schema tag (`okq.find/v1`).
    pub schema: &'static str,
    /// Number of matching concepts.
    pub count: usize,
    /// The matching concepts, in concept-id order.
    pub results: Vec<ConceptRecord>,
}

/// Runs `find` against the bundle at `bundle_dir`.
pub fn run(bundle_dir: &Path, args: &FindArgs, no_ignore: bool) -> Result<FindOutput, AppError> {
    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let predicate = Predicate::build(args)?;

    let results: Vec<ConceptRecord> = corpus
        .concepts()
        .filter(|c| predicate.matches(c))
        .map(|c| ConceptRecord::from_concept(corpus.bundle(), c))
        .collect();

    Ok(FindOutput {
        schema: SCHEMA,
        count: results.len(),
        results,
    })
}

/// A compiled set of predicates: all must hold for a concept to match.
struct Predicate<'a> {
    /// Tags that must all be present (AND).
    tags: &'a [String],
    /// Types, any of which matches (OR); empty means "no type constraint".
    types: &'a [String],
    /// Frontmatter `field=value` constraints (AND).
    wheres: Vec<WherePred>,
    /// Optional title/body matcher.
    matcher: Option<Matcher>,
}

struct WherePred {
    field: String,
    value: String,
}

enum Matcher {
    /// Lowercased needle for case-insensitive substring matching.
    Substring(String),
    Regex(Regex),
}

impl<'a> Predicate<'a> {
    fn build(args: &'a FindArgs) -> Result<Self, AppError> {
        let wheres = args
            .where_
            .iter()
            .map(|spec| {
                let (field, value) = spec.split_once('=').ok_or_else(|| {
                    AppError::Usage(format!("--where must be FIELD=VALUE, got {spec:?}"))
                })?;
                if field.is_empty() {
                    return Err(AppError::Usage(format!(
                        "--where field is empty in {spec:?}"
                    )));
                }
                Ok(WherePred {
                    field: field.to_string(),
                    value: value.to_string(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        let matcher = match &args.match_ {
            None => None,
            Some(pattern) if args.regex => {
                Some(Matcher::Regex(Regex::new(pattern).map_err(|e| {
                    AppError::Usage(format!("invalid --match regex: {e}"))
                })?))
            }
            Some(pattern) => Some(Matcher::Substring(pattern.to_lowercase())),
        };

        Ok(Predicate {
            tags: &args.tag,
            types: &args.type_,
            wheres,
            matcher,
        })
    }

    fn matches(&self, c: &Concept) -> bool {
        let fm = &c.document.frontmatter;

        if !self.tags.is_empty() {
            let tags = fm.tags();
            if !self.tags.iter().all(|t| tags.iter().any(|x| x == t)) {
                return false;
            }
        }

        if !self.types.is_empty() {
            match fm.type_() {
                Some(ty) if self.types.iter().any(|t| t == &ty) => {}
                _ => return false,
            }
        }

        for w in &self.wheres {
            if !where_matches(fm, w) {
                return false;
            }
        }

        if let Some(matcher) = &self.matcher {
            let title = fm.title();
            let mut haystacks: Vec<&str> = Vec::new();
            if let Some(t) = &title {
                haystacks.push(t);
            }
            haystacks.push(&c.document.body);
            if !matcher.is_match(&haystacks) {
                return false;
            }
        }

        true
    }
}

impl Matcher {
    fn is_match(&self, haystacks: &[&str]) -> bool {
        match self {
            Matcher::Substring(needle) => {
                haystacks.iter().any(|h| h.to_lowercase().contains(needle))
            }
            Matcher::Regex(re) => haystacks.iter().any(|h| re.is_match(h)),
        }
    }
}

fn where_matches(fm: &Frontmatter, w: &WherePred) -> bool {
    match fm.get(&w.field) {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(Value::as_display_string)
            .any(|s| s == w.value),
        Some(v) => v.as_display_string().map(|s| s == w.value).unwrap_or(false),
        None => false,
    }
}

/// Serializes the collection envelope as one pretty JSON document.
pub fn to_json(out: &FindOutput) -> String {
    serde_json::to_string_pretty(out).expect("FindOutput is always serializable")
}

/// Writes the human-readable listing: one concept per line, `path:line` first.
pub fn render_human(w: &mut impl Write, out: &FindOutput, no_color: bool) -> std::io::Result<()> {
    let loc = if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().bold()
    };
    for r in &out.results {
        let ty = r.type_.as_deref().unwrap_or("-");
        let title = r.title.as_deref().unwrap_or("");
        writeln!(w, "{loc}{}:{}{loc:#}\t{ty}\t{title}", r.path, r.line)?;
    }
    Ok(())
}
