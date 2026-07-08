---
type: feature
title: okq stats — bundle overview & health metrics
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, stats, metrics, graph, health, json]
milestone: M3
command: "okq stats"
related: ["graph.md", "find.md", "../adrs/0004-exit-code-taxonomy.md", "../guides/design-overview.md"]
---

# okq stats — bundle overview & health metrics

## Summary

`okq stats` prints a single overview of a bundle: how many concepts, how they
break down by `type` and `tag`, how densely they're linked, which concepts are
hubs (most linked-to), the edge-type distribution, and health counts (orphans,
dead links, parse errors). It's the *"what is this knowledge base, at a glance?"*
command — orientation for a human, and a cheap first tool-call for an agent
sizing up an unfamiliar bundle.

## Motivation

Before querying a bundle you don't know, you want its shape: is it 20 docs or
2,000? Mostly ADRs or mostly runbooks? Well-connected or a pile of orphans? Today
that means several `find`/`neighbors`/`orphans`/`deadlinks` runs plus mental
arithmetic. `stats` computes it in one pass over the same parse + graph that
powers the other commands, and turns "is our knowledge base any good?" into a
number you can watch over time or assert in CI.

## Scope

### In scope

- Aggregate counts and distributions over concepts, frontmatter, and the graph.
- Hub detection (most-connected concepts) and link-density metrics.
- Health counts (orphans, dead links, parse errors), reusing M2's graph.
- Human summary and a stable `okq.stats/v1` JSON envelope.

### Out of scope

- **Per-concept detail** — `stats` is aggregate; drill in with `find`/`neighbors`.
- **Health *gating*** — listing/fixing orphans and dead links is `orphans`/
  `deadlinks --check` (M2); `stats` just reports their counts.
- **Time series / history** — `stats` is a point-in-time snapshot; trend tracking
  is the caller's job (run it in CI and diff).

## Behavior

### Metrics

| Metric | Definition |
|--------|-----------|
| `concepts` | number of parsed concepts |
| `edges` | total typed edges that resolve in-bundle (inline `link` + frontmatter relations) |
| `link_density` | `edges / concepts`, to 2 decimals (avg out-degree) |
| `orphans` | concepts with no inbound edges (count; list via `okq orphans`) |
| `dead_links` | **broken** links to missing concepts (count; list via `okq deadlinks`) |
| `phantom_links` | **phantom** links — bare `[[wikilinks]]` to not-yet-created notes (count; list via `okq deadlinks --phantoms-only`). See [phantom-links](phantom-links.md) |
| `parse_errors` | files okf could not parse (count) |
| `types` | map of frontmatter `type` → count (untyped concepts under `"(untyped)"`) |
| `tags` | map of tag → count (frontmatter `tags:` + inline `#tags`, per [inline-tags](inline-tags.md)) |
| `edge_types` | map of edge kind (`link`/`related`/…) → count |
| `hubs` | top-N concepts by inbound degree (the most-referenced docs) |

### Invocation & flags

```sh
okq stats                 # human summary for the bundle in cwd
okq stats --json          # the okq.stats/v1 envelope
okq stats --top 5         # cap the hubs and tags lists at 5 (default 10)
```

- No concept argument — `stats` operates on the whole bundle.
- `--top N` bounds the `hubs` list and the `tags` list (the long-tail ones);
  `types` and `edge_types` are small and always shown in full.

### Output

**Human:** a compact, readable summary, e.g.

```
Concepts: 25    Edges: 80    Density: 3.20 edges/concept
Orphans: 14     Dead links: 0    Phantom links: 3    Parse errors: 6

Types:  adr 6, feature 5, fixture 1, (untyped) 13
Edges:  link 60, related 18, supersedes 2
Tags:   cli 9, search 4, graph 3, … (top 10)

Hubs (most linked-to):
  6  adrs/0002-library-stack.md   ADR-0002 — Library stack
  4  features/search.md           okq search — ranked full-text retrieval
  …
```

**`--json`:** one `okq.stats/v1` document. Maps (`types`/`tags`/`edge_types`) are
key-sorted for determinism; `hubs` is an array of `{ id, title, path, in_degree,
out_degree }` ordered by `in_degree` desc then `id`.

```json
{
  "schema": "okq.stats/v1",
  "concepts": 25, "edges": 80, "link_density": 3.20,
  "orphans": 14, "dead_links": 0, "phantom_links": 3, "parse_errors": 6,
  "types": { "adr": 6, "feature": 5, "(untyped)": 13 },
  "tags": { "cli": 9, "search": 4 },
  "edge_types": { "link": 60, "related": 18, "supersedes": 2 },
  "hubs": [
    { "id": "adrs/0002-library-stack", "title": "ADR-0002 — Library stack",
      "path": "adrs/0002-library-stack.md", "in_degree": 6, "out_degree": 3 }
  ]
}
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success — `stats` is a query; it reports health counts but never gates on them |
| 1 | Bundle could not be loaded |

(Per ADR-0004. CI gating on findings is `orphans`/`deadlinks --check`, not `stats`.)

## Documented stable JSON schemas (the other half of M3)

PLAN.md §7's M3 also calls for **"stable JSON schemas documented"** — the agent
contract. Every command's output type already derives `schemars::JsonSchema`; M3
should surface those:

- Add **`okq schema [<command>]`** — prints the JSON Schema for a command's
  `--json` envelope (all commands, or one), generated from the `schemars` derives.
  Agents can fetch and validate against it; we can snapshot it to detect
  accidental contract drift.
- The `okq.<command>/vN` tags are the versioned contract; a breaking output change
  bumps the version, never mutates `v1` in place.

This is specced here as part of M3 but is a **separable deliverable** — `stats`
can ship first. (Open question below on exact form.)

## Acceptance criteria

- [ ] `okq stats` reports concepts, edges, link_density, orphans, dead_links,
  parse_errors, and the `types`/`tags`/`edge_types` distributions.
- [ ] `hubs` lists the most-linked-to concepts (in-degree desc, id tie-break),
  bounded by `--top` (default 10); `tags` list also bounded by `--top`.
- [ ] Counts agree with the standalone commands (`orphans`/`deadlinks` counts,
  `find` type/tag counts) on the same bundle.
- [ ] `--json` emits `okq.stats/v1` with key-sorted maps and deterministic `hubs`.
- [ ] Exit 0 always on a loadable bundle (no gating); exit 1 on a bad bundle.
- [ ] Works on this repo's own `docs/` tree; output snapshot-tested (`insta`),
  with counts that don't depend on traversal nondeterminism.
- [ ] *(If included)* `okq schema` emits valid JSON Schema for each envelope.

## Open questions

- **Untyped bucket** — label untyped concepts `"(untyped)"` (chosen) or omit them
  from `types`? Reserved/parse-error files are already excluded.
- **Density definition** — `edges / concepts` (avg out-degree, chosen) vs. graph
  density `edges / (n·(n-1))`; the former is more legible for sparse doc graphs.
- **Hubs metric** — rank by inbound degree (chosen, "most referenced") vs. total
  degree vs. a centrality measure; keep it cheap and explainable for v1.
- **`tags` cap** — cap by `--top` (chosen) vs. always-full; large bundles can have
  long tag tails.
- **`okq schema` form** — a subcommand printing JSON Schema (proposed) vs.
  generating a committed `docs/reference/schemas/` tree vs. both. Do it in this
  round or as an M3 follow-up?

## Related

- [graph](graph.md) — the typed-edge model `stats` aggregates (edges, hubs, orphans, dead links)
- [find](find.md) — the per-concept counterpart; `stats` is the bundle-level rollup
- [ADR-0004](../adrs/0004-exit-code-taxonomy.md) — why `stats` is exit-0 (a query, not a gate)
- [PLAN.md](../guides/design-overview.md) — §5 `stats`, §7 M3 (stats + documented JSON schemas), §8 schema versioning
