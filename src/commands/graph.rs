//! The M2 graph commands: `neighbors`, `backlinks`, `path`, `orphans`,
//! `deadlinks`. They share the typed-edge [`Graph`] and the concept envelope.
//! See `docs/features/graph.md`.

use std::io::Write;
use std::path::Path;

use okf::{Bundle, ConceptId};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cli::{
    BacklinksArgs, DeadlinksArgs, DirectionArg, NeighborsArgs, OrphansArgs, PathArgs,
};
use crate::error::AppError;
use crate::graph::{Direction, EdgeFilter, Graph, Reached};
use crate::model::{self, ConceptRecord};

/// Collection envelope for `neighbors`/`backlinks`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphListOutput {
    /// Schema tag (`okq.neighbors/v1` or `okq.backlinks/v1`).
    pub schema: &'static str,
    /// The concept the traversal started from.
    pub concept: String,
    /// Number of reached concepts.
    pub count: usize,
    /// The reached concepts, in (distance, id) order.
    pub results: Vec<GraphNode>,
}

/// A reached concept: the shared envelope plus the edge that reached it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphNode {
    #[serde(flatten)]
    concept: ConceptRecord,
    /// The first-hop edge kind on the path from the source.
    edge: String,
    /// The first-hop direction (`in`/`out`).
    direction: String,
    /// Hop distance from the source.
    distance: usize,
}

/// Envelope for `okq path`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PathOutput {
    /// Schema tag (`okq.path/v1`).
    pub schema: &'static str,
    /// Start concept id.
    pub from: String,
    /// End concept id.
    pub to: String,
    /// Whether a path was found.
    pub found: bool,
    /// Number of edges on the path (0 if not found).
    pub length: usize,
    /// The ordered nodes from `from` to `to`.
    pub path: Vec<PathNode>,
}

/// A node on a path, with the edge taken to reach it (`None` for the start).
#[derive(Debug, Serialize, JsonSchema)]
pub struct PathNode {
    #[serde(flatten)]
    concept: ConceptRecord,
    #[serde(skip_serializing_if = "Option::is_none")]
    edge: Option<String>,
}

/// Envelope for `okq orphans`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct OrphansOutput {
    /// Schema tag (`okq.orphans/v1`).
    pub schema: &'static str,
    /// Number of orphaned concepts.
    pub count: usize,
    /// Concepts with no inbound links, in id order.
    pub results: Vec<ConceptRecord>,
}

/// Envelope for `okq deadlinks`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DeadlinksOutput {
    /// Schema tag (`okq.deadlinks/v1`).
    pub schema: &'static str,
    /// Number of dead links.
    pub count: usize,
    /// The dead links, by source then raw target.
    pub results: Vec<DeadLinkRecord>,
}

/// One dead link: where it's declared and what it points at.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DeadLinkRecord {
    /// The concept that declares the link.
    pub source_id: String,
    /// That concept's path relative to the bundle root.
    pub source_path: String,
    /// 1-based line where the dead target appears.
    pub line: usize,
    /// The link target as written.
    pub raw: String,
    /// The edge kind (`link` or a relation key).
    pub edge: String,
}

// ---- run functions -------------------------------------------------------

/// `okq neighbors`.
pub fn neighbors(bundle_dir: &Path, args: &NeighborsArgs) -> Result<GraphListOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let id = require_concept(&bundle, &args.concept)?;
    let graph = Graph::build(&bundle);
    let reached = graph.neighbors(
        &id,
        args.depth,
        direction(args.direction),
        &EdgeFilter::new(&args.edge),
    );
    Ok(GraphListOutput {
        schema: "okq.neighbors/v1",
        concept: id.to_string(),
        count: reached.len(),
        results: reached.iter().map(|r| node(&bundle, r)).collect(),
    })
}

/// `okq backlinks` — inbound concepts (`neighbors --direction in --depth 1`).
pub fn backlinks(bundle_dir: &Path, args: &BacklinksArgs) -> Result<GraphListOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let id = require_concept(&bundle, &args.concept)?;
    let graph = Graph::build(&bundle);
    let reached = graph.neighbors(&id, 1, Direction::In, &EdgeFilter::new(&args.edge));
    Ok(GraphListOutput {
        schema: "okq.backlinks/v1",
        concept: id.to_string(),
        count: reached.len(),
        results: reached.iter().map(|r| node(&bundle, r)).collect(),
    })
}

/// `okq path`.
pub fn path(bundle_dir: &Path, args: &PathArgs) -> Result<PathOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let from = require_concept(&bundle, &args.from)?;
    let to = require_concept(&bundle, &args.to)?;
    let graph = Graph::build(&bundle);
    let found = graph.shortest_path(&from, &to, args.undirected, &EdgeFilter::new(&args.edge));

    let (found_flag, steps) = match found {
        Some(steps) => (true, steps),
        None => (false, Vec::new()),
    };
    let path: Vec<PathNode> = steps
        .iter()
        .map(|s| PathNode {
            concept: record(&bundle, &s.id),
            edge: s.edge.clone(),
        })
        .collect();
    Ok(PathOutput {
        schema: "okq.path/v1",
        from: from.to_string(),
        to: to.to_string(),
        found: found_flag,
        length: path.len().saturating_sub(1),
        path,
    })
}

/// `okq orphans`.
pub fn orphans(bundle_dir: &Path, _args: &OrphansArgs) -> Result<OrphansOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let graph = Graph::build(&bundle);
    let results: Vec<ConceptRecord> = graph
        .orphans(&bundle)
        .iter()
        .filter_map(|id| bundle.get(id))
        .map(|c| ConceptRecord::from_concept(&bundle, c))
        .collect();
    Ok(OrphansOutput {
        schema: "okq.orphans/v1",
        count: results.len(),
        results,
    })
}

/// `okq deadlinks`.
pub fn deadlinks(bundle_dir: &Path, _args: &DeadlinksArgs) -> Result<DeadlinksOutput, AppError> {
    let bundle = Bundle::load(bundle_dir)?;
    let graph = Graph::build(&bundle);
    let results: Vec<DeadLinkRecord> = graph
        .dead_links()
        .iter()
        .map(|d| {
            let (source_path, line) = source_location(&bundle, &d.source, &d.raw);
            DeadLinkRecord {
                source_id: d.source.to_string(),
                source_path,
                line,
                raw: d.raw.clone(),
                edge: d.kind.clone(),
            }
        })
        .collect();
    Ok(DeadlinksOutput {
        schema: "okq.deadlinks/v1",
        count: results.len(),
        results,
    })
}

// ---- helpers -------------------------------------------------------------

fn require_concept(bundle: &Bundle, input: &str) -> Result<ConceptId, AppError> {
    let id = model::parse_concept_id(input)?;
    if bundle.contains(&id) {
        Ok(id)
    } else {
        Err(AppError::ConceptNotFound {
            input: input.to_string(),
        })
    }
}

fn direction(arg: DirectionArg) -> Direction {
    match arg {
        DirectionArg::In => Direction::In,
        DirectionArg::Out => Direction::Out,
        DirectionArg::Both => Direction::Both,
    }
}

fn record(bundle: &Bundle, id: &ConceptId) -> ConceptRecord {
    bundle
        .get(id)
        .map(|c| ConceptRecord::from_concept(bundle, c))
        .unwrap_or_else(|| ConceptRecord {
            id: id.to_string(),
            type_: None,
            title: None,
            path: id.to_string(),
            line: 1,
            tags: Vec::new(),
        })
}

fn node(bundle: &Bundle, r: &Reached) -> GraphNode {
    GraphNode {
        concept: record(bundle, &r.id),
        edge: r.kind.clone(),
        direction: r.direction.as_str().to_string(),
        distance: r.distance,
    }
}

fn rel_path(bundle: &Bundle, path: &Path) -> String {
    path.strip_prefix(bundle.root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The source's relative path and the 1-based line where `raw` appears (the
/// whole file is scanned so frontmatter relations and body links both resolve).
fn source_location(bundle: &Bundle, source: &ConceptId, raw: &str) -> (String, usize) {
    let Some(concept) = bundle.get(source) else {
        return (source.to_string(), 1);
    };
    let path = rel_path(bundle, &concept.path);
    let line = std::fs::read_to_string(&concept.path)
        .ok()
        .and_then(|text| text.lines().position(|l| l.contains(raw)).map(|i| i + 1))
        .unwrap_or(1);
    (path, line)
}

// ---- output --------------------------------------------------------------

/// Serializes any of the graph envelopes as one pretty JSON document.
pub fn to_json<T: Serialize>(out: &T) -> String {
    serde_json::to_string_pretty(out).expect("graph output is always serializable")
}

fn loc_style(no_color: bool) -> anstyle::Style {
    if no_color {
        anstyle::Style::new()
    } else {
        anstyle::Style::new().bold()
    }
}

/// Human listing for `neighbors`/`backlinks`.
pub fn render_nodes(
    w: &mut impl Write,
    out: &GraphListOutput,
    no_color: bool,
) -> std::io::Result<()> {
    let loc = loc_style(no_color);
    for r in &out.results {
        let title = r.concept.title.as_deref().unwrap_or("");
        writeln!(
            w,
            "{loc}{}:{}{loc:#}  {} ({})  {title}",
            r.concept.path, r.concept.line, r.edge, r.direction
        )?;
    }
    Ok(())
}

/// Human listing for `orphans`.
pub fn render_orphans(
    w: &mut impl Write,
    out: &OrphansOutput,
    no_color: bool,
) -> std::io::Result<()> {
    let loc = loc_style(no_color);
    for r in &out.results {
        let title = r.title.as_deref().unwrap_or("");
        writeln!(w, "{loc}{}:{}{loc:#}  {title}", r.path, r.line)?;
    }
    Ok(())
}

/// Human listing for `deadlinks`.
pub fn render_deadlinks(
    w: &mut impl Write,
    out: &DeadlinksOutput,
    no_color: bool,
) -> std::io::Result<()> {
    let loc = loc_style(no_color);
    for d in &out.results {
        writeln!(
            w,
            "{loc}{}:{}{loc:#}  -[{}]->  {}",
            d.source_path, d.line, d.edge, d.raw
        )?;
    }
    Ok(())
}

/// Human rendering for `path`.
pub fn render_path(w: &mut impl Write, out: &PathOutput, no_color: bool) -> std::io::Result<()> {
    let loc = loc_style(no_color);
    for (i, node) in out.path.iter().enumerate() {
        let title = node.concept.title.as_deref().unwrap_or("");
        if i == 0 {
            writeln!(
                w,
                "{loc}{}:{}{loc:#}  {title}",
                node.concept.path, node.concept.line
            )?;
        } else {
            let edge = node.edge.as_deref().unwrap_or("link");
            writeln!(
                w,
                "  -[{edge}]->  {loc}{}:{}{loc:#}  {title}",
                node.concept.path, node.concept.line
            )?;
        }
    }
    Ok(())
}
