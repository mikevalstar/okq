//! The bundle's cross-link graph: typed edges between concepts, and the
//! traversals the M2 commands run over them. See `docs/features/graph.md`.
//!
//! Edges come from two sources (answering PLAN §8's "reuse depth of okf"):
//! **inline markdown links** (reused from okf, kind `link`) and **frontmatter
//! relations** (built here, kind = the frontmatter key). A simple sorted
//! adjacency + hand-rolled BFS covers neighbors/path; petgraph isn't needed for
//! these unweighted, typed, direction-filtered traversals.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use okf::{Bundle, ConceptId, Value};

/// Frontmatter keys treated as typed relation edges (graph.md). Inline links use
/// the synthetic kind [`LINK_KIND`].
pub const RELATION_KEYS: [&str; 4] = ["related", "supersedes", "superseded-by", "depends-on"];

/// Edge kind for an inline markdown link.
pub const LINK_KIND: &str = "link";

/// Edge kind for an Obsidian-style `[[wikilink]]` (or `![[embed]]`). These are
/// scanned by okq from the body — okf only understands CommonMark links. See
/// [`crate::wikilinks`] and issue #5.
pub const WIKILINK_KIND: &str = "wikilink";

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
    /// Whether this is a genuinely *broken* reference or a *phantom* (a bare
    /// `[[wikilink]]` to a not-yet-created note — normal in Obsidian).
    pub class: DeadClass,
}

/// How an unresolved link should be read (phantom-links.md).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeadClass {
    /// A concrete reference that should resolve but doesn't — a rename/move left
    /// it dangling, a bad path, an inline link to a missing file. Almost always
    /// a real mistake.
    Broken,
    /// A bare `[[Note]]` matching no concept and no alias — an Obsidian
    /// "unresolved" note you click to create later. Normal, not an error.
    Phantom,
}

impl DeadClass {
    /// The string form used in output (`broken`/`phantom`).
    pub fn as_str(self) -> &'static str {
        match self {
            DeadClass::Broken => "broken",
            DeadClass::Phantom => "phantom",
        }
    }
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
    /// Builds the graph from a loaded bundle, excluding `.okqignore`-hidden
    /// concepts: hidden concepts contribute no edges, and a link *into* a hidden
    /// concept becomes a dead link (it now points at nothing in the bundle).
    pub fn build(bundle: &Bundle, hidden: &HashSet<ConceptId>) -> Self {
        let mut out: HashMap<ConceptId, Vec<HalfEdge>> = HashMap::new();
        let mut inn: HashMap<ConceptId, Vec<HalfEdge>> = HashMap::new();
        let mut dead: Vec<DeadLink> = Vec::new();

        // Index visible concepts by their bare name (case-insensitively) so a
        // wikilink like `[[Users]]` can resolve to `tables/users` the way an
        // Obsidian vault would, not just by full path. A second index maps
        // frontmatter `aliases:` → concept, consulted only when the name index
        // misses, so `[[Hooman]]` resolves to a note aliased "Hooman" but a real
        // filename always wins (ADR-0011).
        let names = name_index(bundle, hidden);
        let aliases = alias_index(bundle, hidden);

        for c in bundle.concepts() {
            if hidden.contains(&c.id) {
                continue;
            }
            // Inline markdown links (resolved by okf; broken ones are dead — but
            // only if they point *into* the bundle. A link that escapes the
            // bundle root (e.g. `../README.md`) or is external is not dead, just
            // out of scope; re-resolving `raw` ourselves filters those out,
            // matching how frontmatter relations are treated. A link to a hidden
            // concept is dead too — `.okqignore` removed it from the bundle.
            for link in bundle.links_from(&c.id) {
                if link.exists && !hidden.contains(&link.target) {
                    push_edge(&mut out, &mut inn, &c.id, &link.target, LINK_KIND);
                } else if resolve_relative(&c.id, &link.raw).is_some() {
                    dead.push(DeadLink {
                        source: c.id.clone(),
                        raw: link.raw.clone(),
                        kind: LINK_KIND.to_string(),
                        // An explicit CommonMark link to a missing file is broken,
                        // not a phantom (that concept is bare-name wikilinks only).
                        class: DeadClass::Broken,
                    });
                }
            }

            // Frontmatter relations (resolved here; okf doesn't graph these).
            for key in RELATION_KEYS {
                for value in relation_values(&c.document.frontmatter, key) {
                    match resolve_relative(&c.id, &value) {
                        Some(target) if bundle.contains(&target) && !hidden.contains(&target) => {
                            push_edge(&mut out, &mut inn, &c.id, &target, key);
                        }
                        Some(_) => dead.push(DeadLink {
                            source: c.id.clone(),
                            raw: value,
                            kind: key.to_string(),
                            // A frontmatter relation to a missing concept is broken.
                            class: DeadClass::Broken,
                        }),
                        // None: external URL or a path that escapes the bundle — not an edge.
                        None => {}
                    }
                }
            }

            // Obsidian-style wikilinks scanned from the body (okf only sees
            // CommonMark links). Deduped per source so `[[Foo]]` written twice
            // is one edge / one dead link.
            let mut seen: HashSet<String> = HashSet::new();
            for wl in crate::wikilinks::extract(&c.document.body) {
                if !seen.insert(wl.target.clone()) {
                    continue;
                }
                match resolve_wikilink(&c.id, &wl.target, bundle, hidden, &names, &aliases) {
                    WikiResolution::Resolved(target) => {
                        push_edge(&mut out, &mut inn, &c.id, &target, WIKILINK_KIND);
                    }
                    WikiResolution::Broken => dead.push(DeadLink {
                        source: c.id.clone(),
                        raw: wl.target,
                        kind: WIKILINK_KIND.to_string(),
                        class: DeadClass::Broken,
                    }),
                    WikiResolution::Phantom => dead.push(DeadLink {
                        source: c.id.clone(),
                        raw: wl.target,
                        kind: WIKILINK_KIND.to_string(),
                        class: DeadClass::Phantom,
                    }),
                    WikiResolution::Skip => {}
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

    /// Concepts with no inbound edges, in id order. Hidden (`.okqignore`)
    /// concepts are not candidates — they aren't in the bundle.
    pub fn orphans(&self, bundle: &Bundle, hidden: &HashSet<ConceptId>) -> Vec<ConceptId> {
        let mut ids: Vec<ConceptId> = bundle
            .concepts()
            .iter()
            .map(|c| c.id.clone())
            .filter(|id| !hidden.contains(id))
            .filter(|id| self.inn.get(id).map(Vec::is_empty).unwrap_or(true))
            .collect();
        ids.sort();
        ids
    }

    /// All dead links in the bundle (inline + frontmatter + wikilink), sorted.
    /// Includes both broken links and phantoms; callers filter by [`DeadLink::class`].
    pub fn dead_links(&self) -> &[DeadLink] {
        &self.dead
    }

    /// Count of genuinely *broken* dead links (excludes phantoms).
    pub fn broken_count(&self) -> usize {
        self.dead
            .iter()
            .filter(|d| d.class == DeadClass::Broken)
            .count()
    }

    /// Count of *phantom* links — bare `[[wikilinks]]` to not-yet-created notes.
    pub fn phantom_count(&self) -> usize {
        self.dead
            .iter()
            .filter(|d| d.class == DeadClass::Phantom)
            .count()
    }

    /// Total number of resolved edges (each directed edge counted once).
    pub fn total_edges(&self) -> usize {
        self.out.values().map(Vec::len).sum()
    }

    /// Count of edges by kind, key-sorted.
    pub fn edge_kind_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for edges in self.out.values() {
            for e in edges {
                *counts.entry(e.kind.clone()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Number of inbound edges to a concept.
    pub fn in_degree(&self, id: &ConceptId) -> usize {
        self.inn.get(id).map(Vec::len).unwrap_or(0)
    }

    /// Number of outbound edges from a concept.
    pub fn out_degree(&self, id: &ConceptId) -> usize {
        self.out.get(id).map(Vec::len).unwrap_or(0)
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

/// The outcome of resolving a wikilink target against the bundle.
enum WikiResolution {
    /// Points at an existing, visible concept.
    Resolved(ConceptId),
    /// A concrete path/`.md` in-bundle reference that resolves to nothing — a
    /// genuinely broken link (phantom-links.md).
    Broken,
    /// A bare name matching no concept and no alias — a phantom / not-yet-created
    /// note, normal in an Obsidian vault (phantom-links.md).
    Phantom,
    /// Not a bundle edge (external, or unresolvable and not worth flagging).
    Skip,
}

/// Bare concept name (lowercased) → the visible concepts with that name, sorted.
/// Lets `[[Users]]` find `tables/users` the way Obsidian resolves by note name.
fn name_index(bundle: &Bundle, hidden: &HashSet<ConceptId>) -> HashMap<String, Vec<ConceptId>> {
    let mut map: HashMap<String, Vec<ConceptId>> = HashMap::new();
    for c in bundle.concepts() {
        if hidden.contains(&c.id) {
            continue;
        }
        map.entry(c.id.name().to_lowercase())
            .or_default()
            .push(c.id.clone());
    }
    for ids in map.values_mut() {
        ids.sort();
    }
    map
}

/// Frontmatter alias (lowercased) → the visible concepts declaring it, sorted.
/// Lets `[[Hooman]]` resolve to a note whose frontmatter says `aliases: [Hooman]`
/// (ADR-0011). Consulted only after the name index misses, so a real filename
/// always wins over an alias.
fn alias_index(bundle: &Bundle, hidden: &HashSet<ConceptId>) -> HashMap<String, Vec<ConceptId>> {
    let mut map: HashMap<String, Vec<ConceptId>> = HashMap::new();
    for c in bundle.concepts() {
        if hidden.contains(&c.id) {
            continue;
        }
        for alias in crate::model::concept_aliases(c) {
            map.entry(alias.to_lowercase())
                .or_default()
                .push(c.id.clone());
        }
    }
    for ids in map.values_mut() {
        ids.sort();
        ids.dedup();
    }
    map
}

/// Resolves an Obsidian wikilink target (leniently, issue #5): a target with a
/// `/` is treated as a path (bundle-root-absolute first, then relative to the
/// source), and a bare name matches a concept's filename — or, failing that, a
/// frontmatter alias — anywhere in the bundle, case-insensitively. An in-bundle
/// path reference that matches nothing is *broken*; a bare name matching neither
/// a filename nor an alias is a *phantom*; anything that isn't a plausible bundle
/// target is skipped.
fn resolve_wikilink(
    source: &ConceptId,
    target: &str,
    bundle: &Bundle,
    hidden: &HashSet<ConceptId>,
    names: &HashMap<String, Vec<ConceptId>>,
    aliases: &HashMap<String, Vec<ConceptId>>,
) -> WikiResolution {
    let exists = |id: &ConceptId| bundle.contains(id) && !hidden.contains(id);

    if target.contains('/') {
        // A path: prefer the vault-relative (from bundle root) reading Obsidian
        // uses, then fall back to source-relative before declaring it dead.
        for candidate in [resolve_from_root(target), resolve_relative(source, target)]
            .into_iter()
            .flatten()
        {
            if exists(&candidate) {
                return WikiResolution::Resolved(candidate);
            }
        }
        // A path that at least forms a valid in-bundle id is broken; one that
        // escapes the bundle (`../..`) is simply out of scope.
        if resolve_from_root(target).is_some() {
            WikiResolution::Broken
        } else {
            WikiResolution::Skip
        }
    } else {
        let key = target.to_lowercase();
        // Filename first, then alias — a real file always wins (ADR-0011).
        match names.get(&key).or_else(|| aliases.get(&key)) {
            Some(ids) => WikiResolution::Resolved(ids[0].clone()),
            None => WikiResolution::Phantom,
        }
    }
}

/// Resolves a wikilink path read relative to the bundle root (how Obsidian reads
/// `[[folder/note]]`), tolerating `.md`, `.`/`..`, and a leading `/`.
fn resolve_from_root(value: &str) -> Option<ConceptId> {
    let value = value.strip_suffix(".md").unwrap_or(value);
    let mut segments: Vec<String> = Vec::new();
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

/// Resolves a relation/link target written relative to `source`'s directory into
/// a bundle concept id. Returns `None` for external URLs or paths that escape the
/// bundle root (mirrors okf's inline-link resolution). Each path component is
/// percent-decoded — as okf does — so an encoded target (`Quarterly%20Report`,
/// `%F0%9F%9A%80%20Launch`) is classified by the concept it denotes, not by its
/// literal escapes; that keeps a *broken* encoded link a dead link and a working
/// encoded relation a real edge, independent of what characters the id rule allows.
fn resolve_relative(source: &ConceptId, value: &str) -> Option<ConceptId> {
    let value = value.split('#').next().unwrap_or(value).trim();
    if value.is_empty() || value.contains("://") {
        return None;
    }
    let value = value.strip_suffix(".md").unwrap_or(value);

    let mut segments: Vec<String> = source.segments().to_vec();
    segments.pop(); // drop the source file → its directory
    for part in value.split('/') {
        // Structural `.`/`..` are matched before decoding, so an *encoded* dot
        // segment can't smuggle in path traversal; only real components decode.
        match part {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            p => segments.push(percent_decode(p)),
        }
    }
    ConceptId::new(segments).ok()
}

/// Percent-decodes a single path component (`Quarterly%20Report` →
/// `Quarterly Report`, `%F0%9F%9A%80` → `🚀`), accumulating raw bytes so multi-byte
/// UTF-8 escapes reassemble. Malformed or incomplete `%` escapes are left as
/// written. Mirrors okf's link resolver so okq classifies encoded targets the
/// same way the data layer does.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
