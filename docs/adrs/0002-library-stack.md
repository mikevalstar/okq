---
title: ADR-0002 — Library stack (stand on the shoulders of giants)
status: accepted
created: 2026-06-26
updated: 2026-06-26
tags: [rust, dependencies, okf, ripgrep, search, graph, cli]
supersedes: null
superseded-by: null
related: ["0001-documentation-first-okf-shaped.md", "0003-search-index-in-xdg-cache.md", "../../PLAN.md"]
---

# ADR-0002: Library stack (stand on the shoulders of giants)

> **Amended by [ADR-0003](0003-search-index-in-xdg-cache.md): the search index lives in a per-bundle XDG cache directory, not in-bundle `.okq/index/`.** Every reference to `.okq/index/` below is superseded by that location; the rest of this ADR (Tantivy as the backend, the index-as-derived-cache rule, the whole stack) stands.

## Context

`okq` is Rust ([PLAN.md](../../PLAN.md) §4) and its design principle is "fast & dependency-light: lean on the upstream `okf` crate … `okq` adds the *query* surface on top rather than reimplementing the parser." We need to decide, across the whole [command surface](../../PLAN.md#5-command-surface-draft) (`search`, `find`, `neighbors`, `backlinks`, `path`, `orphans`, `deadlinks`, `stats`, `get`, `init`, `new`), which capabilities we **reuse from mature crates** versus build ourselves.

Two facts shape the decision:

1. **The upstream [`okf` crate](https://github.com/W4G1/okf) is pure-Rust and *zero-dependency*.** It ships its own YAML-subset frontmatter parser, markdown link scanner, directory walker, and arg parser, and exposes `Bundle::load()`, the link graph + backlinks, `validate_bundle()`, and `ConceptId`. It already covers the **data layer** — parse, model, frontmatter, graph, backlinks, validation, identity. (This answers PLAN.md §8's "reuse depth of the `okf` crate": reuse heavily for data; don't reimplement it.)
2. **`okf` deliberately stops at the data layer.** It does not rank, does not do graph *algorithms* (shortest path, depth-limited traversal, components), and gives no CLI/JSON/scaffolding surface. That is exactly `okq`'s job — and where reaching for giants pays off.

So "dependency-light" is reinterpreted, not abandoned: **dependency-*deliberate*.** Reuse `okf` for everything it covers; add a focused, battle-tested crate for each query-layer capability `okf` lacks; defer the heavy ones behind the same evidence gate the plan already applies to vectors.

## Options considered

The non-obvious sub-decisions (the rest of the stack is conventional and listed under Decision):

### Search: ranked retrieval vs. grep — and which backend

ripgrep is the obvious "look through the docs" giant, but **ripgrep ranks nothing** — it scans and filters. `okq search` is defined as *ranked* (PLAN.md §5). So ripgrep's crates power the **filter/scan** path (`find --match`, fast literal pre-scan), and a **separate scoring layer** powers `search`:

- **A — in-memory BM25** (small crate or hand-rolled over tokenized sections). Dependency-light, no persisted index, but rebuilt on every process start.
- **B — `tantivy`** (Lucene-class, persisted, BM25 built in). Heavier dependency and a real index lifecycle, but no per-call rebuild and richer queries/snippets out of the box.

**Chosen: B (`tantivy`) from day one.** Three reasons override the lighter option:

1. **No throwaway.** The project exists for the agent "context-assembly wall" past ~100 docs (PLAN.md §2) — i.e. we *know* we want a persisted, scalable index. Hand-rolling in-memory BM25 first means building it, then ripping it out and migrating. Starting on Tantivy skips the discard.
2. **No per-call rebuild in the hot path.** `okq` is called repeatedly in an agent loop (`search → neighbors → get`). In-memory BM25 rebuilds on every invocation; a persisted Tantivy index is built once and reopened cheaply — exactly where cold rebuild hurts most.
3. **Same scoring, more headroom.** Tantivy is BM25 by default (ranking semantics unchanged) and adds phrase/boolean/field queries, snippet highlighting (feeds the token-frugal `path:line` + snippet output, PLAN.md §3), and faceting — capabilities we'd otherwise hand-roll.

This reverses PLAN.md §6/§8's "defer Tantivy" posture; that text should be updated to name Tantivy as the backend.

### File discovery: `okf`'s walker vs. ripgrep's `ignore`

- **A — `okf`'s built-in walker.** Already there, strict-bundle semantics.
- **B — ripgrep's `ignore` crate.** Gitignore/`.ignore`-aware parallel walk.

**Chosen: B as the default, A available for strict mode.** `okq` is "format-tolerant" (PLAN.md §4) and pointed at real repos / `docs/` trees, where respecting `.gitignore` is the correct behavior; fall back to `okf`'s loader for strict OKF-bundle mode.

### Frontmatter access: reuse vs. a YAML dep

Prefer **`okf`'s frontmatter parser**. Only if `okf` doesn't expose generic field access needed by `find --where field=value` do we add a YAML reader — and then **`serde_yaml_ng`** or **`serde_norway`**, never `serde_yaml` (deprecated/archived) or `serde_yml` (flagged unsound, [RUSTSEC-2025-0068](https://rustsec.org/advisories/RUSTSEC-2025-0068.html)).

### Shelling out to `rg` vs. linking ripgrep's crates

**Link the crates** (`ignore`, `grep`, `regex`, `globset`), don't shell out to the `rg` binary. Keeps `okq` self-contained, deterministic, and free of an external-tool dependency — consistent with the agent-runnable / no-network contract.

## Decision

Adopt this stack. Each entry maps to a capability `okf` does not provide; heavy entries are deferred.

**Data foundation (reuse, don't rebuild)**
- **`okf`** — parse, model, frontmatter, link graph, backlinks, validation, concept identity.

**Scan & discovery (the "ripgrep" giants)**
- **`ignore`** — gitignore-aware file discovery (default walk).
- **`grep`** / **`grep-searcher`** / **`grep-regex`**, **`regex`**, **`globset`** — `find --match`, fast literal/regex scan.

**Ranked search**
- **`tantivy`** — persisted, section-granular BM25 index from day one. Each `pulldown-cmark` section becomes a Tantivy document (fields: `concept_id`, `path`, `line`, `heading`, `title`, `tags`/`type`, `body`), keeping search at section granularity (PLAN.md §5). The index is a **derived, git-ignored artifact** under `.okq/index/` — a cache rebuilt from the concept docs, never a source of truth (same "generated, not source" rule as OKF's `index.md`). `okq` auto-builds/refreshes it on demand; it tracks per-file mtime/hash + an index-format version to reindex incrementally and rebuild on version mismatch.

**Markdown / sections**
- **`pulldown-cmark`** — heading-delimited section chunking (search granularity + `get --section`) and inline-link extraction for OKF-*shaped* trees `okf` may not fully model. (`comrak` only if GFM rendering is ever needed — deferred.)

**Graph algorithms**
- **`petgraph`** — build a graph from `okf`'s edges, run shortest-path (`path`), depth-limited BFS (`neighbors --depth`), components/orphans (`orphans`), hubs (`stats`).

**CLI / output / errors**
- **`clap`** v4 (derive) — args & subcommands.
- **`serde`** + **`serde_json`** — `--json`.
- **`schemars`** — derive a stable, versioned JSON Schema for output types (the agent-facing contract, PLAN.md §8).
- **`anstream`** + **`anstyle`** — color honoring `--no-color` / `NO_COLOR` / non-TTY.
- **`anyhow`** (binary) + **`thiserror`** (library crate) — error model & exit codes.
- **`tracing`** + **`tracing-subscriber`** (or lighter **`env_logger`**) — logs to **stderr**, never stdout (which carries the `--json` document).

**Scaffolding**
- **`include_dir`** or **`rust-embed`** — embed `init`/`new` templates in the binary (no template-dir bootstrap).

**Dev / test**
- **`insta`** (snapshot the human + JSON output contracts), **`assert_cmd`** + **`predicates`** (CLI integration tests).

**Deferred / explicitly not now**
- **`comrak`** (GFM render) — added only if GFM tables/footnotes rendering is ever needed.
- Embeddings / vector search — out of scope for v1 by PLAN.md §4; its own future ADR.

## Consequences

- **`okf` becomes a hard dependency**, and the data/query split is now explicit: `okf` owns data, `okq` owns query. Risk: `okf` is young and may not expose everything (e.g. generic frontmatter field access, typed edges). Follow-up in **M0/M1**: verify `okf`'s actual public API against `find --where`, typed-edge, and section-chunking needs; the YAML-fallback and `pulldown-cmark` link-scan entries exist precisely to cover gaps.
- **The dependency count rises from `okf`'s zero to ~12–15 crates**, with `tantivy` the heaviest (it pulls in transitive deps and grows compile time + binary size). Accepted as the price of not reinventing a search engine and of avoiding a later BM25→Tantivy migration; binary size is revisited only if it becomes a distribution problem (M4). Embeddings stay out until evidence demands them. "Dependency-light" is preserved as "dependency-deliberate."
- **ripgrep powers scanning, not ranking** — keeping that boundary clear prevents conflating `find` (filter) with `search` (rank, via Tantivy), which PLAN.md §5 already separates.
- **`okq` now writes local state** — the Tantivy index under `.okq/index/`. It previously needed none; now there's an index directory to create, locate, git-ignore, and invalidate. This must be reconciled with the upcoming determinism / local-first ADR: the index is local and deterministic (same bundle state → same index → same ranking), a cache rather than independent truth, so the principle holds — but that ADR must say so explicitly.
- **Output stability is designed in** via `schemars` + `insta`, turning "JSON schema as a contract" (PLAN.md §8) into something enforced by tests from day one.
- **Follow-ups created:** confirm `okf`'s license is compatible with okq's Apache-2.0 ([LICENSE](../../LICENSE)); pin an MSRV; add `.okq/` to `.gitignore`; and settle the index lifecycle — staleness policy (mtime vs. content hash; partial vs. full reindex; index-format version pinning) and writer-lock behavior when two `okq` processes share an index. These become PLAN.md §8 open questions.

## Related

- [ADR-0001](0001-documentation-first-okf-shaped.md) — the docs/process decision this follows
- [PLAN.md](../../PLAN.md) — §4 principles, §5 command surface, §6 architecture, §8 open questions (reuse depth, search backend, schema versioning)
