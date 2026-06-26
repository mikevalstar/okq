---
title: okq graph navigation — neighbors / backlinks / path / orphans / deadlinks
status: accepted # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, graph, neighbors, backlinks, path, orphans, deadlinks, edges, json]
milestone: M2
command: "okq neighbors | backlinks | path | orphans | deadlinks"
related: ["search.md", "find.md", "get.md", "../adrs/0002-library-stack.md", "../adrs/0004-exit-code-taxonomy.md", "../../PLAN.md"]
---

# okq graph navigation — neighbors / backlinks / path / orphans / deadlinks

## Summary

The M2 graph commands navigate the **cross-link structure** of a bundle:
`neighbors` (adjacent concepts), `backlinks` (inbound references), `path`
(shortest route between two concepts), `orphans` (no inbound links), and
`deadlinks` (links to missing concepts). Together they are okq's differentiator —
the *"start somewhere relevant, then move outward"* half of the loop that
`search`/`find` begin.

## Motivation

`search`/`find` answer *"where do I start?"*. Graph navigation answers *"what's
connected to here?"* — the question you can't answer by reading one doc (you can't
see a doc's *inbound* links from inside it). The expensive agent loop — find a
doc, then grep-and-read outward to map an area — collapses into
`search → neighbors → get`. No other OKF tool does graph-aware query; this is the
unique value (PLAN.md §3). `orphans`/`deadlinks` additionally turn "is our
knowledge base healthy?" into a command CI can run.

## Scope

### In scope

- Typed-edge navigation over the bundle's cross-links: `neighbors`, `backlinks`,
  `path`, `orphans`, `deadlinks`.
- Both edge sources: inline markdown links **and** frontmatter relations.
- Locations-only output (node = `id`/`type`/`title`/`path:line` + edge), `--json`.

### Out of scope

- **Bundle metrics** (`stats`: link density, hubs, distributions) — that's M3.
- **Ranking** — graph results are structural, ordered deterministically, not scored.
- **Content** — nodes are pointers; expand with `get`.
- **Mutation / link rewriting.**

## The graph model (shared foundation)

A bundle is a directed graph: nodes are concepts, edges are typed cross-links.
okq draws edges from **two sources**, answering PLAN.md §8's "reuse depth of
okf's graph":

1. **Inline links** — markdown links in a concept's body. **Reused from okf**
   (`Bundle::links_from`, `backlinks`, `broken_links`). Edge type: `link`.
2. **Frontmatter relations** — list/scalar frontmatter keys that name other
   concepts (`related`, `supersedes`, `superseded-by`, `depends-on`). **Built by
   okq** (okf does not treat frontmatter values as edges). Edge type = the key
   name. Values are resolved as a path relative to the source concept's directory
   (tolerating `.md` and `../`), then to a `ConceptId`; an unresolvable value is a
   typed dead link.

The **default recognized relation keys** are `related`, `supersedes`,
`superseded-by`, `depends-on` (plus inline `link`). The exact taxonomy — fixed
allowlist vs. deriving every path-valued frontmatter key — is an open question
(PLAN.md §8); start with the allowlist.

**Direction:** an edge is *out* of the concept that declares it (its body link or
its frontmatter relation) and *in* to the target. `backlinks` is the inbound view.

**Determinism:** adjacency is sorted by concept id; traversal (BFS) visits in
sorted order, so output is stable run-to-run (PLAN.md §4). Built on okf's parse +
`petgraph` for traversal/shortest-path (ADR-0002).

**Output — the node record** reuses the shared concept envelope (`get`/`find`)
plus edge metadata, and is token-frugal (no bodies):

```json
{ "id": "adrs/0001-...", "type": "adr", "title": "ADR-0001 — …",
  "path": "adrs/0001-....md", "line": 1,
  "edge": "supersedes", "direction": "out", "distance": 1 }
```

`edge`/`direction`/`distance` appear where meaningful (omitted for `orphans`).

## Behavior

### `okq neighbors <concept>`

Concepts adjacent to `<concept>` via edges.

- `--depth N` (default **1**) — N-hop neighborhood.
- `--direction in|out|both` (default **both**).
- `--edge <type>` (repeatable) — restrict to edge types (`link`, `related`, …).
- Output: node records (the source excluded), ordered by `distance` then `id`.
  Each carries the `edge` type and `direction` of the hop that reached it (for
  depth > 1, the first hop's edge and the total `distance`).
- `<concept>` not found → exit 4. No neighbors → exit 0, empty.
- Schema: `okq.neighbors/v1` collection envelope.

### `okq backlinks <concept>`

Concepts that link/relate **to** `<concept>` (inbound, depth 1) — the view you
can't get by reading the doc itself.

- `--edge <type>` (repeatable) filter.
- Equivalent to `neighbors --direction in --depth 1`, kept as a first-class command
  for the common case.
- Output: node records with the inbound `edge` type. Not found → 4; none → 0.
- Schema: `okq.backlinks/v1`.

### `okq path <a> <b>`

Shortest path between two concepts over the link graph (unweighted BFS).

- `--undirected` — ignore edge direction (treat links as bidirectional); default
  follows edge **direction** (out-edges). *(Default directedness is an open
  question — see below.)*
- `--edge <type>` (repeatable) filter.
- Output: `okq.path/v1` — `{ from, to, found, length, path: [node, …] }`, the
  ordered nodes from `a` to `b`, each (after the first) carrying the `edge` taken.
- `a` or `b` not found → exit 4. **No path → exit 0** with `found: false`, empty
  `path` (an empty answer is success, ADR-0004).

### `okq orphans`

Concepts with **no inbound edges** (no backlinks of any type) — stale-doc
candidates.

- `--check` — exit **3** if any orphans are found (CI gate, ADR-0004); default
  lists them at exit 0.
- Output: `okq.orphans/v1` node records (no edge metadata). Deterministic id order.
- Note: a bundle's intentional roots (e.g. a top `index`) may be legitimately
  orphaned; this command surfaces candidates, it doesn't judge.

### `okq deadlinks`

Edges pointing to missing/renamed concepts — from inline links (okf
`broken_links`) **and** unresolvable frontmatter relations.

- `--check` — exit **3** if any dead links are found; default exit 0.
- Output: `okq.deadlinks/v1` — `{ count, results: [ { source_id, source_path,
  line, raw, edge } ] }`, where `raw` is the link target as written and `edge` is
  its type. Ordered by `source_id` then `raw`.

## Cross-cutting contracts this feature ratifies

- **The typed-edge model** — inline `link` (okf) + frontmatter relations (okq),
  with `--edge` filtering — reused by every graph command and by M3 `stats`.
- **The node record** — concept envelope + `edge`/`direction`/`distance` — the
  graph counterpart to `search`'s hit record.
- **Reuse depth of okf's graph is decided** (PLAN.md §8): reuse okf for inline
  links/backlinks/broken-links; okq builds frontmatter-relation edges and all
  traversal algorithms.
- **ADR-0004 in practice:** not-found → 4, empty/no-path → 0, health `--check` → 3.

## Acceptance criteria

- [ ] `neighbors` honors `--depth`, `--direction`, `--edge`; excludes the source;
  orders by distance then id; reports edge + direction + distance.
- [ ] `backlinks` returns inbound concepts with edge type; matches
  `neighbors --direction in --depth 1`.
- [ ] `path` finds a shortest route, respects `--undirected`/`--edge`, and reports
  the ordered nodes + edges; no path → exit 0 `found:false`.
- [ ] `orphans` lists concepts with zero inbound edges; `--check` → exit 3 when any.
- [ ] `deadlinks` reports inline **and** frontmatter dead links with source
  `path:line`, raw target, and edge type; `--check` → exit 3 when any.
- [ ] Edges combine inline links + the default relation keys; `--edge` filters.
- [ ] Missing concept → exit 4; all commands have `--json` (collection/path
  envelopes); output is locations-only (no bodies); deterministic ordering.
- [ ] Works on this repo's own `docs/` tree (which cross-links via both inline
  links and `related:` frontmatter); robust to malformed/edge fixtures.
- [ ] Output snapshot-tested (`insta`).

## Open questions

- **Edge-type taxonomy** — fixed allowlist (`related`/`supersedes`/`superseded-by`/
  `depends-on` + `link`) vs. deriving every path-valued frontmatter key; how to
  configure per-bundle. (PLAN.md §8.)
- **`path` directedness default** — follow edge direction (chosen default) vs.
  undirected reachability. Knowledge graphs are often navigated undirected; revisit
  with real usage.
- **Relation resolution** — values as bundle-relative paths vs. bare concept ids vs.
  frontmatter `id`; how strict (warn vs. silently drop unresolvable). Ties to the
  concept-identity open question.
- **`neighbors` deep-hop edge reporting** — report only the first-hop edge + total
  distance (chosen) or the full path per neighbor?
- **`orphans` roots** — should known roots (`index`) be excluded by default or via a
  flag, to cut false positives?
- **`backlinks` vs `neighbors`** — keep both, or make `backlinks` a thin alias? Kept
  separate for discoverability and the locked `okq.backlinks/v1` schema.

## Related

- [search](search.md) / [find](find.md) — the "where do I start" half; `search → neighbors → get` is the core loop
- [get](get.md) — expands a node chosen from a graph result
- [ADR-0002](../adrs/0002-library-stack.md) — `okf` link graph + `petgraph` for traversal
- [ADR-0004](../adrs/0004-exit-code-taxonomy.md) — not-found/empty/`--check` exit codes these commands adopt
- [PLAN.md](../../PLAN.md) — §3 graph differentiator, §5 command surface, §8 edge-taxonomy & graph-reuse open questions
