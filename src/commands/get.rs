//! `okq get` — expand one concept on demand.
//!
//! Resolves a concept by identity (path-minus-`.md`, or a `.md` path), then
//! emits the selected parts — frontmatter and/or body, or a single section or
//! frontmatter field — as human text or the `okq.get/v1` JSON envelope. See
//! `docs/features/get.md`.

use std::path::Path;

use okf::Value;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::GetArgs;
use crate::error::AppError;
use crate::sections::{self, Section};
use crate::view::Corpus;
use crate::yaml_json;

/// Schema tag stamped on every `get` JSON document; the contract agents depend on.
pub const SCHEMA: &str = "okq.get/v1";

/// The JSON envelope for `okq get`. The `id`/`type`/`title`/`path`/`line`
/// fields are the shared concept envelope reused by other commands' shortlists.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GetOutput {
    /// Schema tag (`okq.get/v1`).
    pub schema: &'static str,
    /// The concept id (path minus `.md`).
    pub id: String,
    /// The concept's path relative to the bundle root.
    pub path: String,
    /// 1-based line where the concept begins (always 1).
    pub line: usize,
    /// The frontmatter `type`, if present.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The concept's title: the frontmatter `title`, or the filename if none.
    pub title: String,
    /// Trust & lifecycle values, each omitted when at its default
    /// (`docs/features/trust.md`).
    #[serde(flatten)]
    pub trust: crate::trust::ConceptTrust,
    /// The `generated` and `verified` events the trust tier was derived from.
    /// Omitted when the concept carries neither.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<TrustEvents>,
    /// Frontmatter (well-known keys + producer extensions), narrowed to the one
    /// key named by `--field` if given. Omitted unless requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<serde_json::Value>,
    /// Full body markdown. Omitted unless requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The selected section(s). Present only with `--section`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<SectionOut>>,
}

/// The evidence behind a derived trust tier: who produced the content and who
/// has confirmed it. Reported so the derivation is auditable rather than magic
/// (ADR-0014).
#[derive(Debug, Serialize, JsonSchema)]
pub struct TrustEvents {
    /// `generated: {by, at}` — who wrote the current content, and when.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<TrustEvent>,
    /// `verified: [{by, at}]` — every confirmation, in frontmatter order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verified: Vec<TrustEvent>,
}

/// One `{by, at}` event, with both fields as written.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TrustEvent {
    /// The actor, in the §7 convention: `human:<id>`, `process:<id>`, or the
    /// agent form `<producer>/<version>` (e.g. `okq/0.7.0`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// The timestamp exactly as written in the frontmatter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// A section as it appears in the JSON envelope.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SectionOut {
    /// Heading text.
    pub heading: String,
    /// Slugified heading.
    pub slug: String,
    /// Heading depth, 1–6.
    pub level: u8,
    /// 1-based source line of the heading.
    pub line: usize,
    /// Section source markdown.
    pub body: String,
}

impl From<&Section> for SectionOut {
    fn from(s: &Section) -> Self {
        SectionOut {
            heading: s.heading.clone(),
            slug: s.slug.clone(),
            level: s.level,
            line: s.line,
            body: s.body.clone(),
        }
    }
}

/// The result of a `get`: the JSON-serializable envelope plus the original
/// frontmatter YAML (kept verbatim for faithful, order-preserving human output).
pub struct Got {
    /// The JSON envelope.
    pub output: GetOutput,
    /// Frontmatter rendered as YAML, present iff whole frontmatter was requested.
    pub frontmatter_yaml: Option<String>,
    /// The selected field's value, rendered bare. Present only with `--field`.
    pub field_yaml: Option<String>,
}

/// Runs `get` against the bundle at `bundle_dir`.
pub fn run(bundle_dir: &Path, args: &GetArgs, no_ignore: bool) -> Result<Got, AppError> {
    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let id = crate::model::resolve_concept(&corpus, &args.concept)?;
    let concept = corpus
        .get(&id)
        .expect("resolve_concept returns an existing concept");

    let rel = concept
        .path
        .strip_prefix(corpus.bundle().root())
        .unwrap_or(&concept.path);
    let path = rel.to_string_lossy().replace('\\', "/");

    let frontmatter = &concept.document.frontmatter;
    let body = &concept.document.body;

    // Selectors are additive; with none, default to frontmatter + full body.
    let any_selector =
        args.frontmatter || args.body || args.section.is_some() || args.field.is_some();
    // `--field` narrows the frontmatter surface to the one key it names, so it
    // suppresses the whole-frontmatter rendering the way `--section` does the body.
    let want_frontmatter = args.field.is_none() && (args.frontmatter || !any_selector);
    let want_body = args.body || !any_selector;

    let field = match &args.field {
        Some(query) => Some(select_field(frontmatter.as_mapping(), query, &path)?),
        None => None,
    };

    let sections = match &args.section {
        Some(query) => {
            let raw = std::fs::read_to_string(&concept.path)?;
            let secs = sections::parse_sections(body, sections::body_start_line(&raw));
            let chosen = select_section(&secs, query, &path)?;
            Some(vec![SectionOut::from(chosen)])
        }
        None => None,
    };

    let frontmatter_json = match &field {
        Some((key, value)) => {
            let mut obj = serde_json::Map::new();
            obj.insert(key.clone(), yaml_json::yaml_to_json(value));
            Some(serde_json::Value::Object(obj))
        }
        None => want_frontmatter.then(|| yaml_json::mapping_to_json(frontmatter.as_mapping())),
    };
    let frontmatter_yaml = want_frontmatter.then(|| {
        Value::Mapping(frontmatter.as_mapping().clone())
            .to_yaml_string()
            .trim_end()
            .to_string()
    });
    // A string field prints verbatim (so `--field title` is pipe-safe); anything
    // else prints as YAML (a list as `- item` lines, a map as `key: value`).
    let field_yaml = field.as_ref().map(|(_, value)| match value.as_str() {
        Some(s) => s.to_string(),
        None => value.to_yaml_string().trim_end().to_string(),
    });

    Ok(Got {
        output: GetOutput {
            schema: SCHEMA,
            id: id.to_string(),
            path,
            line: 1,
            type_: frontmatter.type_().map(|t| t.into_owned()),
            title: crate::model::concept_title(concept),
            trust: crate::trust::ConceptTrust::from_frontmatter(
                frontmatter,
                okf::Date::today_utc(),
            ),
            provenance: trust_events(frontmatter),
            frontmatter: frontmatter_json,
            body: want_body.then(|| body.clone()),
            sections,
        },
        frontmatter_yaml,
        field_yaml,
    })
}

/// Collects the `generated`/`verified` events, or `None` when there are none —
/// so a concept with no trust frontmatter carries no extra keys at all.
fn trust_events(fm: &okf::Frontmatter) -> Option<TrustEvents> {
    let generated = fm.generated().map(|g| TrustEvent {
        by: g.by.map(|a| a.as_str().to_string()),
        at: g.at.map(|d| d.raw),
    });
    let verified: Vec<TrustEvent> = fm
        .verified()
        .into_iter()
        .map(|v| TrustEvent {
            by: v.by.map(|a| a.as_str().to_string()),
            at: v.at.map(|d| d.raw),
        })
        .collect();
    (generated.is_some() || !verified.is_empty()).then_some(TrustEvents {
        generated,
        verified,
    })
}

/// Selects the one frontmatter key matching `query` by exact name, or
/// case-insensitively with `-` and `_` treated as equivalent (`depends-on` ↔
/// `depends_on`); zero or multiple matches are errors (exit 5).
fn select_field<'a>(
    mapping: &'a okf::Mapping,
    query: &str,
    concept: &str,
) -> Result<(String, &'a Value), AppError> {
    let q_norm = normalize_key(query);
    let matches: Vec<(String, &Value)> = mapping
        .iter()
        .filter_map(|(k, v)| k.as_str().map(|k| (k.to_string(), v)))
        .filter(|(k, _)| k == query || normalize_key(k) == q_norm)
        .collect();

    match matches.len() {
        0 => Err(AppError::FieldNotFound {
            concept: concept.to_string(),
            query: query.to_string(),
        }),
        1 => Ok(matches
            .into_iter()
            .next()
            .expect("length checked to be exactly one")),
        _ => Err(AppError::FieldAmbiguous {
            concept: concept.to_string(),
            query: query.to_string(),
            candidates: matches.into_iter().map(|(k, _)| k).collect(),
        }),
    }
}

/// Normalizes a frontmatter key for lenient matching: lowercased, with `_`
/// folded to `-`.
fn normalize_key(key: &str) -> String {
    key.to_lowercase().replace('_', "-")
}

/// Selects the one section matching `query` by case-insensitive heading text or
/// slug; zero or multiple matches are errors (exit 5). `concept` names the
/// document searched, for the error message (also reused by `okq spec`).
pub(crate) fn select_section<'a>(
    sections: &'a [Section],
    query: &str,
    concept: &str,
) -> Result<&'a Section, AppError> {
    let q_lower = query.to_lowercase();
    let q_slug = sections::slugify(query);
    let matches: Vec<&Section> = sections
        .iter()
        .filter(|s| s.heading.to_lowercase() == q_lower || s.slug == q_slug || s.slug == query)
        .collect();

    match matches.as_slice() {
        [] => Err(AppError::SectionNotFound {
            concept: concept.to_string(),
            query: query.to_string(),
        }),
        [one] => Ok(one),
        many => Err(AppError::SectionAmbiguous {
            concept: concept.to_string(),
            query: query.to_string(),
            candidates: many
                .iter()
                .map(|s| format!("{} (line {})", s.heading, s.line))
                .collect(),
        }),
    }
}

/// Serializes the envelope as one pretty JSON document.
pub fn to_json(got: &Got) -> String {
    serde_json::to_string_pretty(&got.output).expect("GetOutput is always serializable")
}

/// Writes the human-readable rendering to `w`. Color is applied unless `no_color`.
///
/// Every selector prints a leading `path:line` header so the location is always
/// visible — except `--field`, whose whole point is to yield one value. Emitting
/// the header there would put a second line on stdout and break
/// `$(okq get x --field title)`; the location stays available via `--json`.
pub fn render_human(w: &mut impl std::io::Write, got: &Got, no_color: bool) -> std::io::Result<()> {
    let header = if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().bold()
    };
    // `--field` prints the value alone (pipe-safe), so neither the header nor
    // the trust block appears with it.
    if got.field_yaml.is_none() {
        let labels = got.output.trust.labels();
        let dim = if no_color {
            anstyle::Style::new()
        } else {
            anstyle::Style::new().dimmed()
        };
        write!(
            w,
            "{header}{}:{}{header:#}",
            got.output.path, got.output.line
        )?;
        if !labels.is_empty() {
            write!(w, "  {dim}[{}]{dim:#}", labels.join(", "))?;
        }
        writeln!(w)?;
        render_trust_events(w, got.output.provenance.as_ref(), dim)?;
    }

    if let Some(fm) = &got.frontmatter_yaml {
        writeln!(w, "---\n{fm}\n---")?;
    }
    if let Some(field) = &got.field_yaml {
        writeln!(w, "{field}")?;
    }
    if let Some(body) = &got.output.body {
        writeln!(w, "\n{}", body.trim_end())?;
    }
    if let Some(sections) = &got.output.sections {
        for s in sections {
            writeln!(w, "\n{}", s.body.trim_end())?;
        }
    }
    Ok(())
}

/// The `generated`/`verified` evidence lines, aligned. Prints nothing when the
/// concept carries no trust events.
fn render_trust_events(
    w: &mut impl std::io::Write,
    events: Option<&TrustEvents>,
    dim: anstyle::Style,
) -> std::io::Result<()> {
    let Some(events) = events else {
        return Ok(());
    };
    let mut rows: Vec<(&str, &TrustEvent)> = Vec::new();
    if let Some(g) = &events.generated {
        rows.push(("generated", g));
    }
    rows.extend(events.verified.iter().map(|v| ("verified", v)));

    // An absent *or empty* `by` renders as `-`; a blank column reads as a bug
    // rather than as the missing value it is.
    let actor = |e: &TrustEvent| match e.by.as_deref() {
        Some(by) if !by.trim().is_empty() => by.to_string(),
        _ => "-".to_string(),
    };
    let widest = rows.iter().map(|(_, e)| actor(e).len()).max().unwrap_or(0);
    for (label, e) in rows {
        let by = actor(e);
        match &e.at {
            Some(at) => writeln!(w, "{dim}{label:<9}  {by:<widest$}  {at}{dim:#}")?,
            None => writeln!(w, "{dim}{label:<9}  {by}{dim:#}")?,
        }
    }
    Ok(())
}
