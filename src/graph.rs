//! The bundle's cross-link graph: typed edges between concepts, and the
//! traversals the M2 commands run over them. See `docs/features/graph.md`.
//!
//! Edges come from two sources (answering PLAN §8's "reuse depth of okf"):
//! **inline markdown links** (reused from okf, kind `link`) and **frontmatter
//! relations** (built here, kind = the frontmatter key). A simple sorted
//! adjacency + hand-rolled BFS covers neighbors/path; petgraph isn't needed for
//! these unweighted, typed, direction-filtered traversals.

use std::collections::{HashMap, HashSet, VecDeque};

use okf::{Bundle, ConceptId, Value};

/// Frontmatter keys treated as typed relation edges (graph.md). Inline links use
/// the synthetic kind [`LINK_KIND`].
pub const RELATION_KEYS: [&str; 4] = ["related", "supersedes", "superseded-by", "depends-on"];

/// Edge kind for an inline markdown link.
pub const LINK_KIND: &str = "link";

/// Traversal direction.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Follow inbound edges (who points at me).
    In,
    /// Follow outbound edges (who I point at).
    Out,
    /// Follow both.
    Both,
}

impl Direction {
    /// The string form used in output (`in`/`out`).
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::Both => "both",
        }
    }
}

/// A filter over edge kinds; empty = allow all.
pub struct EdgeFilter(Vec<String>);

impl EdgeFilter {
    /// Builds a filter from a list of kinds (empty allows everything).
    pub fn new(kinds: &[String]) -> Self {
        EdgeFilter(kinds.to_vec())
    }
    fn allows(&self, kind: &str) -> bool {
        self.0.is_empty() || self.0.iter().any(|k| k == kind)
    }
}

struct HalfEdge {
    other: ConceptId,
    kind: String,
}

/// A link pointing at a concept that does not exist in the bundle.
pub struct DeadLink {
    /// The concept that declares the link.
    pub source: ConceptId,
    /// The link target as written.
    pub raw: String,
    /// The edge kind (`link` or a relation key).
    pub kind: String,
}

/// A node reached during a neighbors traversal.
pub struct Reached {
    /// The reached concept.
    pub id: ConceptId,
    /// The first-hop edge kind on the path from the source.
    pub kind: String,
    /// The first-hop direction.
    pub direction: Direction,
    /// Hop distance from the source.
    pub distance: usize,
}

/// One step on a path: a node and the edge kind taken to reach it (`None` for
/// the starting node).
pub struct PathStep {
    /// The concept at this step.
    pub id: ConceptId,
    /// The edge kind taken into this node.
    pub edge: Option<String>,
}

/// A BFS frontier entry: (node, distance-from-source, first-hop edge+direction).
type Frontier = (ConceptId, usize, Option<(String, Direction)>);

/// The typed cross-link graph of a bundle.
pub struct Graph {
    out: HashMap<ConceptId, Vec<HalfEdge>>,
    inn: HashMap<ConceptId, Vec<HalfEdge>>,
    dead: Vec<DeadLink>,
}

impl Graph {
    /// Builds the graph from a loaded bundle.
    pub fn build(bundle: &Bundle) -> Self {
        let mut out: HashMap<ConceptId, Vec<HalfEdge>> = HashMap::new();
        let mut inn: HashMap<ConceptId, Vec<HalfEdge>> = HashMap::new();
        let mut dead: Vec<DeadLink> = Vec::new();

        for c in bundle.concepts() {
            // Inline markdown links (resolved by okf; broken ones are dead — but
            // only if they point *into* the bundle. A link that escapes the
            // bundle root (e.g. `../PLAN.md`) or is external is not dead, just
            // out of scope; re-resolving `raw` ourselves filters those out,
            // matching how frontmatter relations are treated.
            for link in bundle.links_from(&c.id) {
                if link.exists {
                    push_edge(&mut out, &mut inn, &c.id, &link.target, LINK_KIND);
                } else if resolve_relative(&c.id, &link.raw).is_some() {
                    dead.push(DeadLink {
                        source: c.id.clone(),
                        raw: link.raw.clone(),
                        kind: LINK_KIND.to_string(),
                    });
                }
            }

            // Frontmatter relations (resolved here; okf doesn't graph these).
            for key in RELATION_KEYS {
                for value in relation_values(&c.document.frontmatter, key) {
                    match resolve_relative(&c.id, &value) {
                        Some(target) if bundle.contains(&target) => {
                            push_edge(&mut out, &mut inn, &c.id, &target, key);
                        }
                        Some(_) => dead.push(DeadLink {
                            source: c.id.clone(),
                            raw: value,
                            kind: key.to_string(),
                        }),
                        // None: external URL or a path that escapes the bundle — not an edge.
                        None => {}
                    }
                }
            }
        }

        for edges in out.values_mut() {
            edges.sort_by(|a, b| a.other.cmp(&b.other).then(a.kind.cmp(&b.kind)));
        }
        for edges in inn.values_mut() {
            edges.sort_by(|a, b| a.other.cmp(&b.other).then(a.kind.cmp(&b.kind)));
        }
        dead.sort_by(|a, b| a.source.cmp(&b.source).then(a.raw.cmp(&b.raw)));

        Graph { out, inn, dead }
    }

    fn adjacent(
        &self,
        node: &ConceptId,
        direction: Direction,
        edges: &EdgeFilter,
    ) -> Vec<(ConceptId, String, Direction)> {
        let mut adj = Vec::new();
        if matches!(direction, Direction::Out | Direction::Both) {
            for e in self.out.get(node).map(Vec::as_slice).unwrap_or(&[]) {
                if edges.allows(&e.kind) {
                    adj.push((e.other.clone(), e.kind.clone(), Direction::Out));
                }
            }
        }
        if matches!(direction, Direction::In | Direction::Both) {
            for e in self.inn.get(node).map(Vec::as_slice).unwrap_or(&[]) {
                if edges.allows(&e.kind) {
                    adj.push((e.other.clone(), e.kind.clone(), Direction::In));
                }
            }
        }
        adj
    }

    /// Concepts within `depth` hops of `source`, with the first-hop edge and the
    /// total distance, in (distance, id) order. The source is excluded.
    pub fn neighbors(
        &self,
        source: &ConceptId,
        depth: usize,
        direction: Direction,
        edges: &EdgeFilter,
    ) -> Vec<Reached> {
        let mut visited: HashSet<ConceptId> = HashSet::from([source.clone()]);
        let mut queue: VecDeque<Frontier> = VecDeque::new();
        queue.push_back((source.clone(), 0, None));
        let mut result = Vec::new();

        while let Some((node, dist, first)) = queue.pop_front() {
            if dist >= depth {
                continue;
            }
            for (other, kind, hop_dir) in self.adjacent(&node, direction, edges) {
                if visited.insert(other.clone()) {
                    // Propagate the first hop's edge/direction (graph.md choice).
                    let first_hop = first.clone().unwrap_or((kind, hop_dir));
                    result.push(Reached {
                        id: other.clone(),
                        kind: first_hop.0.clone(),
                        direction: first_hop.1,
                        distance: dist + 1,
                    });
                    queue.push_back((other, dist + 1, Some(first_hop)));
                }
            }
        }

        result.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.id.cmp(&b.id)));
        result
    }

    /// The shortest path from `a` to `b` (unweighted BFS), or `None` if there is
    /// no route. `undirected` ignores edge direction.
    pub fn shortest_path(
        &self,
        a: &ConceptId,
        b: &ConceptId,
        undirected: bool,
        edges: &EdgeFilter,
    ) -> Option<Vec<PathStep>> {
        if a == b {
            return Some(vec![PathStep {
                id: a.clone(),
                edge: None,
            }]);
        }
        let direction = if undirected {
            Direction::Both
        } else {
            Direction::Out
        };

        let mut prev: HashMap<ConceptId, (ConceptId, String)> = HashMap::new();
        let mut visited: HashSet<ConceptId> = HashSet::from([a.clone()]);
        let mut queue: VecDeque<ConceptId> = VecDeque::from([a.clone()]);

        while let Some(node) = queue.pop_front() {
            for (other, kind, _) in self.adjacent(&node, direction, edges) {
                if visited.insert(other.clone()) {
                    prev.insert(other.clone(), (node.clone(), kind));
                    if &other == b {
                        return Some(reconstruct(a, b, &prev));
                    }
                    queue.push_back(other);
                }
            }
        }
        None
    }

    /// Concepts with no inbound edges, in id order.
    pub fn orphans(&self, bundle: &Bundle) -> Vec<ConceptId> {
        let mut ids: Vec<ConceptId> = bundle
            .concepts()
            .iter()
            .map(|c| c.id.clone())
            .filter(|id| self.inn.get(id).map(Vec::is_empty).unwrap_or(true))
            .collect();
        ids.sort();
        ids
    }

    /// All dead links in the bundle (inline + frontmatter), sorted.
    pub fn dead_links(&self) -> &[DeadLink] {
        &self.dead
    }
}

fn push_edge(
    out: &mut HashMap<ConceptId, Vec<HalfEdge>>,
    inn: &mut HashMap<ConceptId, Vec<HalfEdge>>,
    from: &ConceptId,
    to: &ConceptId,
    kind: &str,
) {
    out.entry(from.clone()).or_default().push(HalfEdge {
        other: to.clone(),
        kind: kind.to_string(),
    });
    inn.entry(to.clone()).or_default().push(HalfEdge {
        other: from.clone(),
        kind: kind.to_string(),
    });
}

fn reconstruct(
    a: &ConceptId,
    b: &ConceptId,
    prev: &HashMap<ConceptId, (ConceptId, String)>,
) -> Vec<PathStep> {
    let mut chain = vec![b.clone()];
    let mut cur = b.clone();
    while &cur != a {
        let (parent, _) = prev[&cur].clone();
        chain.push(parent.clone());
        cur = parent;
    }
    chain.reverse();
    chain
        .into_iter()
        .map(|id| {
            let edge = prev.get(&id).map(|(_, k)| k.clone());
            PathStep { id, edge }
        })
        .collect()
}

/// The string values of a frontmatter relation key (scalar or sequence).
fn relation_values(fm: &okf::Frontmatter, key: &str) -> Vec<String> {
    match fm.get(key) {
        Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_display_string).collect(),
        Some(v) => v.as_display_string().into_iter().collect(),
        None => Vec::new(),
    }
}

/// Resolves a relation/link target written relative to `source`'s directory into
/// a bundle concept id. Returns `None` for external URLs or paths that escape the
/// bundle root (mirrors okf's inline-link resolution).
fn resolve_relative(source: &ConceptId, value: &str) -> Option<ConceptId> {
    let value = value.split('#').next().unwrap_or(value).trim();
    if value.is_empty() || value.contains("://") {
        return None;
    }
    let value = value.strip_suffix(".md").unwrap_or(value);

    let mut segments: Vec<String> = source.segments().to_vec();
    segments.pop(); // drop the source file → its directory
    for part in value.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            p => segments.push(p.to_string()),
        }
    }
    ConceptId::new(segments).ok()
}
