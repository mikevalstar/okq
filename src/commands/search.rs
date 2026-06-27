//! `okq search` — ranked full-text retrieval over section text.
//!
//! Builds/opens the per-bundle Tantivy index, runs a BM25 query (OR + boosts),
//! and returns the top-N section hits with `path:line`, heading, score, and a
//! snippet — deterministically ordered. See `docs/features/search.md`.

use std::cmp::Ordering;
use std::io::Write;
use std::path::Path;

use okf::ConceptId;
use schemars::JsonSchema;
use serde::Serialize;
use tantivy::TantivyDocument;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Field;
use tantivy::schema::document::Value as _;
use tantivy::snippet::SnippetGenerator;

use crate::cli::SearchArgs;
use crate::error::AppError;
use crate::index::{self, Fields};
use crate::view::Corpus;

/// Schema tag stamped on every `search` JSON document.
pub const SCHEMA: &str = "okq.search/v1";

/// The `okq.search/v1` collection envelope.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchOutput {
    /// Schema tag (`okq.search/v1`).
    pub schema: &'static str,
    /// The query as parsed.
    pub query: String,
    /// Number of returned hits (≤ `--limit`).
    pub count: usize,
    /// The ranked section hits.
    pub results: Vec<SearchHit>,
}

/// One ranked section hit: the concept envelope + section locator + score + snippet.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SearchHit {
    /// The concept id.
    pub id: String,
    /// The frontmatter `type`, if present.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// The frontmatter `title`, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The concept's path relative to the bundle root.
    pub path: String,
    /// 1-based line of the matching section.
    pub line: u64,
    /// The section heading (the concept title for a pre-heading match).
    pub heading: String,
    /// Slugified heading.
    pub slug: String,
    /// Heading depth (0 for a pre-heading/preamble hit).
    pub level: u8,
    /// BM25 score, rounded to 2 decimals.
    pub score: f64,
    /// The concept's tags.
    pub tags: Vec<String>,
    /// A short snippet of the matching text (plain, single line).
    pub snippet: String,
}

/// Runs `search` against the bundle at `bundle_dir`.
pub fn run(
    bundle_dir: &Path,
    args: &SearchArgs,
    no_ignore: bool,
) -> Result<SearchOutput, AppError> {
    let query_str = args.query.trim();
    if query_str.is_empty() {
        return Err(AppError::Usage("search query must not be empty".into()));
    }

    let corpus = Corpus::load(bundle_dir, no_ignore)?;
    let si = index::open_or_build(&corpus, args.reindex, args.ephemeral)?;
    let f = &si.fields;

    let mut parser = QueryParser::for_index(&si.index, vec![f.body, f.title, f.heading]);
    parser.set_field_boost(f.title, 3.0);
    parser.set_field_boost(f.heading, 2.0);
    let query = parser
        .parse_query(query_str)
        .map_err(|e| AppError::Usage(format!("invalid query: {e}")))?;

    let reader = si
        .index
        .reader()
        .map_err(|e| AppError::Index(e.to_string()))?;
    let searcher = reader.searcher();
    let top = searcher
        .search(&query, &TopDocs::with_limit(args.limit.max(1)))
        .map_err(|e| AppError::Index(e.to_string()))?;

    let mut snippet_gen = SnippetGenerator::create(&searcher, &query, f.body)
        .map_err(|e| AppError::Index(e.to_string()))?;
    snippet_gen.set_max_num_chars(160);

    let mut hits = Vec::with_capacity(top.len());
    for (score, address) in top {
        let doc: TantivyDocument = searcher
            .doc(address)
            .map_err(|e| AppError::Index(e.to_string()))?;
        hits.push(build_hit(score, &doc, f, &snippet_gen, &corpus));
    }

    // Deterministic order: score desc, then path, then line.
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line.cmp(&b.line))
    });

    Ok(SearchOutput {
        schema: SCHEMA,
        query: query_str.to_string(),
        count: hits.len(),
        results: hits,
    })
}

fn build_hit(
    score: f32,
    doc: &TantivyDocument,
    f: &Fields,
    snippet_gen: &SnippetGenerator,
    corpus: &Corpus,
) -> SearchHit {
    let id = first_str(doc, f.concept_id);
    let heading = first_str(doc, f.heading);

    // Concept-level fields come from the bundle, keyed by id (visible only).
    let concept = ConceptId::parse(&id).ok().and_then(|cid| corpus.get(&cid));
    let (type_, title, path, tags) = match concept {
        Some(c) => {
            let rel = c
                .path
                .strip_prefix(corpus.bundle().root())
                .unwrap_or(&c.path)
                .to_string_lossy()
                .replace('\\', "/");
            (
                c.document.frontmatter.type_(),
                c.document.frontmatter.title(),
                rel,
                c.document.frontmatter.tags(),
            )
        }
        None => (None, None, id.clone(), Vec::new()),
    };

    let snippet = snippet_gen.snippet_from_doc(doc);
    let fragment = snippet.fragment().trim();
    let snippet = if fragment.is_empty() {
        first_str(doc, f.body)
            .split_whitespace()
            .take(24)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        fragment.split_whitespace().collect::<Vec<_>>().join(" ")
    };

    SearchHit {
        id,
        type_,
        title,
        path,
        line: first_u64(doc, f.line),
        heading,
        slug: first_str(doc, f.slug),
        level: first_u64(doc, f.level) as u8,
        score: round2(score),
        tags,
        snippet,
    }
}

fn first_str(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_default()
}

fn first_u64(doc: &TantivyDocument, field: Field) -> u64 {
    doc.get_first(field).and_then(|v| v.as_u64()).unwrap_or(0)
}

fn round2(score: f32) -> f64 {
    ((score as f64) * 100.0).round() / 100.0
}

/// Serializes the collection envelope as one pretty JSON document.
pub fn to_json(out: &SearchOutput) -> String {
    serde_json::to_string_pretty(out).expect("SearchOutput is always serializable")
}

/// Writes the human-readable ranked listing.
pub fn render_human(w: &mut impl Write, out: &SearchOutput, no_color: bool) -> std::io::Result<()> {
    let loc = if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().bold()
    };
    let dim = if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().dimmed()
    };
    for r in &out.results {
        let heading = if r.level > 0 {
            format!("{} {}", "#".repeat(r.level as usize), r.heading)
        } else {
            r.heading.clone()
        };
        writeln!(
            w,
            "{loc}{}:{}{loc:#}   {:.2}   {heading}",
            r.path, r.line, r.score
        )?;
        if !r.snippet.is_empty() {
            writeln!(w, "    {dim}{}{dim:#}", r.snippet)?;
        }
    }
    Ok(())
}
