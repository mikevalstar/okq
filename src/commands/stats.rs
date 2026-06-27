//! `okq stats` — a one-pass overview of a bundle: counts, distributions, graph
//! metrics, and health counts. See `docs/features/stats.md`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use okf::Bundle;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::StatsArgs;
use crate::error::AppError;
use crate::graph::Graph;

/// Schema tag stamped on every `stats` JSON document.
pub const SCHEMA: &str = "okq.stats/v1";

/// The `okq.stats/v1` envelope. Maps are key-sorted (`BTreeMap`) for determinism.
#[derive(Debug, Serialize, JsonSchema)]
pub struct StatsOutput {
    /// Schema tag (`okq.stats/v1`).
    pub schema: &'static str,
    /// Number of parsed concepts.
    pub concepts: usize,
    /// Total resolved edges.
    pub edges: usize,
    /// Edges per concept (avg out-degree), to 2 decimals.
    pub link_density: f64,
    /// Concepts with no inbound edges.
    pub orphans: usize,
    /// Links pointing at missing concepts.
    pub dead_links: usize,
    /// Files okf could not parse.
    pub parse_errors: usize,
    /// Count of concepts by frontmatter `type` (untyped under `(untyped)`).
    pub types: BTreeMap<String, usize>,
    /// Count of concepts by tag.
    pub tags: BTreeMap<String, usize>,
    /// Count of edges by kind.
    pub edge_types: BTreeMap<String, usize>,
    /// The most linked-to concepts (in-degree desc, id tie-break), capped by `--top`.
    pub hubs: Vec<Hub>,
}

/// A hub concept: identity, location, and its degrees.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Hub {
    /// The concept id.
    pub id: String,
    /// The frontmatter title, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Path relative to the bundle root.
    pub path: String,
    /// Number of inbound edges.
    pub in_degree: usize,
    /// Number of outbound edges.
    pub out_degree: usize,
}

/// Runs `stats` against the bundle at `bundle_dir`.
pub fn run(bundle_dir: &Path, args: &StatsArgs) -> Result<StatsOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let graph = Graph::build(&bundle);

    let concepts = bundle.concepts().len();
    let edges = graph.total_edges();
    let link_density = if concepts > 0 {
        ((edges as f64 / concepts as f64) * 100.0).round() / 100.0
    } else {
        0.0
    };

    let mut types: BTreeMap<String, usize> = BTreeMap::new();
    let mut tags: BTreeMap<String, usize> = BTreeMap::new();
    for c in bundle.concepts() {
        let type_ = c
            .document
            .frontmatter
            .type_()
            .unwrap_or_else(|| "(untyped)".to_string());
        *types.entry(type_).or_insert(0) += 1;
        for tag in c.document.frontmatter.tags() {
            *tags.entry(tag).or_insert(0) += 1;
        }
    }

    let mut ranked: Vec<Hub> = bundle
        .concepts()
        .iter()
        .map(|c| {
            let path = c
                .path
                .strip_prefix(bundle.root())
                .unwrap_or(&c.path)
                .to_string_lossy()
                .replace('\\', "/");
            Hub {
                id: c.id.to_string(),
                title: c.document.frontmatter.title(),
                path,
                in_degree: graph.in_degree(&c.id),
                out_degree: graph.out_degree(&c.id),
            }
        })
        .filter(|h| h.in_degree > 0)
        .collect();
    ranked.sort_by(|a, b| b.in_degree.cmp(&a.in_degree).then(a.id.cmp(&b.id)));
    ranked.truncate(args.top);

    Ok(StatsOutput {
        schema: SCHEMA,
        concepts,
        edges,
        link_density,
        orphans: graph.orphans(&bundle).len(),
        dead_links: graph.dead_links().len(),
        parse_errors: bundle.parse_errors().len(),
        types,
        tags,
        edge_types: graph.edge_kind_counts(),
        hubs: ranked,
    })
}

/// Serializes the envelope as one pretty JSON document.
pub fn to_json(out: &StatsOutput) -> String {
    serde_json::to_string_pretty(out).expect("StatsOutput is always serializable")
}

/// Formats a map as `k v, k v` ordered by count descending (id tie-break),
/// capped at `top` entries (0 = no cap).
fn distribution(map: &BTreeMap<String, usize>, top: usize) -> String {
    let mut pairs: Vec<(&String, &usize)> = map.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let total = pairs.len();
    let shown: Vec<String> = pairs
        .iter()
        .take(if top == 0 { total } else { top })
        .map(|(k, v)| format!("{k} {v}"))
        .collect();
    let mut out = shown.join(", ");
    if top != 0 && total > top {
        out.push_str(&format!(", … ({total} total)"));
    }
    out
}

/// Writes the human-readable summary. `top` caps the displayed tags list.
pub fn render_human(
    w: &mut impl Write,
    out: &StatsOutput,
    top: usize,
    no_color: bool,
) -> std::io::Result<()> {
    let bold = if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().bold()
    };

    writeln!(
        w,
        "Concepts: {}    Edges: {}    Density: {:.2} edges/concept",
        out.concepts, out.edges, out.link_density
    )?;
    writeln!(
        w,
        "Orphans: {}     Dead links: {}    Parse errors: {}",
        out.orphans, out.dead_links, out.parse_errors
    )?;
    writeln!(w)?;
    writeln!(w, "Types:  {}", distribution(&out.types, 0))?;
    writeln!(w, "Edges:  {}", distribution(&out.edge_types, 0))?;
    writeln!(w, "Tags:   {}", distribution(&out.tags, top))?;

    if !out.hubs.is_empty() {
        writeln!(w, "\nHubs (most linked-to):")?;
        for h in &out.hubs {
            let title = h.title.as_deref().unwrap_or("");
            writeln!(w, "  {:>3}  {bold}{}{bold:#}  {title}", h.in_degree, h.path)?;
        }
    }
    Ok(())
}
