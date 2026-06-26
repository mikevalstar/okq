---
title: okq search — ranked full-text retrieval
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, search, tantivy, bm25, ranking, index, json]
milestone: M1
command: "okq search"
related: ["find.md", "get.md", "../adrs/0002-library-stack.md", "../adrs/0003-search-index-in-xdg-cache.md", "../../PLAN.md"]
---

# okq search — ranked full-text retrieval

## Summary

`okq search <query>` returns a **relevance-ranked** shortlist of **section-level** hits across the bundle — each with `path:line`, the heading, a score, and a short snippet — backed by a persisted Tantivy BM25 index. It is the *"find the doc(s) about X"* backbone, and the ranked counterpart to `find` (exact predicate, unranked).

## Motivation

`find` answers "which concepts match this *exact* predicate?". It can't answer "what's the most relevant material about *retrieval latency*?" — that needs **ranking**, and ranking across a bundle too large to read linearly is exactly the agent "context-assembly wall" okq exists for (PLAN.md §2). `search` returns *just the best few sections + their locations*, so an agent spends context on the right pages and a human gets an answer instead of a `rg` dump. It's the first half of the core loop: **`search → get --section`** (locate, then expand), and later `search → neighbors` (locate, then traverse).

## Scope

### In scope

- Ranked retrieval over section text, titles, and headings via a persisted Tantivy BM25 index.
- Term and quoted-phrase queries; per-field boosting (title/heading over body).
- Top-N ranked hits with `path:line`, heading, score, and a snippet — locations-only, never full bodies.
- Automatic index build/refresh in a per-bundle XDG cache directory, plus an ephemeral (in-memory) mode.
- Human and `--json` (the `okq.search/v1` collection envelope).

### Out of scope

- **Semantic / vector retrieval** — deferred and evidence-gated (PLAN.md §4); a future ADR.
- **Predicate filtering** — that's `find` (though combining the two — "search within `--type adr`" — is an open question).
- **Graph traversal** — `neighbors`/`path` (M2).
- **Content expansion** — a hit is a *pointer*; `get --section` expands it.

## Behavior

### What gets indexed (the unit is a section)

Each **section** (heading-delimited, via the same `pulldown-cmark` chunking `get` ratified) becomes one Tantivy document with fields:

| Field | Indexed | Stored | Purpose |
|-------|---------|--------|---------|
| `body` | yes (tokenized) | yes (for snippet) | the section text — primary ranked field |
| `heading` | yes (boosted) | yes | the section heading |
| `title` | yes (boosted) | yes | the concept's frontmatter title (on every section) |
| `type`, `tags` | yes | yes | for display and future search-within-filter |
| `concept_id`, `path`, `line`, `slug`, `level` | no | yes | locators returned in results |

- Body text **before the first heading** is indexed as a leading section anchored at the concept's first body line, so no prose is unsearchable.
- **Tokenizer: lowercased + English stemming (v1).** Chosen for recall on prose — `retrieval` matches `retrieve`/`retrieving`. The trade-off is that exact-token queries (IDs, API names, error codes) can over-match; the escape hatch is a **quoted phrase** (`"okq.get"`), and a future per-field raw/exact tokenizer is an open question.

### Query

```sh
okq search "tantivy index lifecycle"      # rank sections by relevance
okq search '"search backend"'             # quoted phrase
okq search retrieval --limit 5            # top 5 (default 10)
okq search "okf" --json                   # ranked envelope with scores
okq search "decision" --reindex           # force a full rebuild first
okq search "decision" --ephemeral         # in-memory index, no disk writes
```

- Multiple terms default to **OR with BM25 scoring** (a section matching more/rarer terms ranks higher) — the standard "aboutness" model. `--all` (require every term) is an open question.
- Quoted `"…"` is a phrase query. Field boosts: `title` > `heading` > `body`.
- An empty/whitespace query is a usage error (exit 2); an unparseable query (bad phrase syntax) is also exit 2.

### Output

Ranked, token-frugal — **never full bodies**.

**Human:** one hit per entry, location first:

```
adrs/0002-library-stack.md:78   3.41   ## Decision
    …Tantivy is the search backend from day one; each section becomes a…
```

(`path:line` is the **section's** line — the win over `find`'s `:1`. Score to 2 decimals. Snippet is one elided line with matched terms emphasized unless `--no-color`.)

**`--json`:** the `okq.search/v1` collection envelope (same top-level shape as `find`, richer per item):

```json
{
  "schema": "okq.search/v1",
  "query": "tantivy",
  "count": 2,
  "results": [
    { "id": "adrs/0002-library-stack", "type": "adr",
      "title": "ADR-0002 — Library stack",
      "path": "adrs/0002-library-stack.md", "line": 78,
      "heading": "Decision", "slug": "decision", "level": 2,
      "score": 3.41, "tags": ["rust", "search"],
      "snippet": "Tantivy is the search backend from day one…" }
  ]
}
```

Each result is the shared concept envelope (`id`/`type`/`title`/`path`) extended with the **section locator** (`line`/`heading`/`slug`/`level`), `score`, and a plain-text `snippet` (no ANSI — agent-friendly). `count` is the number of returned hits (≤ `--limit`).

### Ranking & determinism

Results are sorted by **score descending, tie-broken by `(path, line)` ascending**, so identical bundle state always yields identical ordering (the determinism principle, PLAN.md §4) — BM25 ties never reorder run-to-run or machine-to-machine.

### The index lifecycle

- **Location: a per-bundle XDG cache directory** — `${XDG_CACHE_HOME:-~/.cache}/okq/<bundle-key>/`, where `<bundle-key>` is derived from the bundle's canonical absolute path. The index **never writes into the bundle/repo** (so read-only and shared bundles just work, and nothing needs git-ignoring). It is a derived, rebuildable cache, never source of truth (the "generated, not source" rule). See [ADR-0003](../adrs/0003-search-index-in-xdg-cache.md).
- **Auto-build on demand:** if the index is missing or stale, `search` builds/refreshes it transparently — no separate "index first" step.
- **Staleness:** a manifest stored beside the index records each concept file's `(path, mtime, size)` plus an okq **index-schema version** and the okf/tantivy versions. On search, changed/added files are re-indexed and deleted files dropped; a schema/version mismatch triggers a full rebuild. `--reindex` forces a full rebuild.
- **Ephemeral mode:** `--ephemeral` builds a transient in-memory index for one run and writes nothing — for CI or when the cache dir can't be created. okq also falls back to ephemeral (with a stderr note) if the cache directory isn't writable.
- **Concurrency:** Tantivy holds a writer lock during a build; a second concurrent writer fails fast with a clear message rather than corrupting (exact behavior — wait vs. error vs. ephemeral-fallback — is an open question).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Ran successfully, **including zero hits** (`count: 0`) |
| 2 | Usage: empty query, unparseable query syntax, bad flags |
| 1 | Index or bundle error (load failure, writer-lock contention, I/O) |

## Cross-cutting contracts this feature ratifies

- **The search-hit record** = concept envelope + section locator (`line`/`heading`/`slug`/`level`) + `score` + `snippet` — the ranked-result shape.
- **The index lifecycle conventions** — per-bundle XDG cache location, manifest-based staleness, schema-version pinning, ephemeral fallback — which any future indexing feature (incremental `--watch`, the optional MCP server) inherits.
- Reuses the section model (`get`) and the collection-envelope top shape (`find`).

## Acceptance criteria

- [ ] `okq search <query>` returns sections ranked by BM25, with section-level `path:line`, heading, score, and snippet.
- [ ] Term queries default to OR-with-scoring; quoted phrases work; title/heading are boosted over body.
- [ ] Body before the first heading is searchable; the v1 tokenizer is lowercase + English stemming.
- [ ] `--limit` bounds results (default 10); ordering is score-desc, tie-broken by `(path, line)` — deterministic across runs.
- [ ] `--json` emits the `okq.search/v1` envelope (query + count + hit records); snippets are plain text.
- [ ] Index auto-builds in the per-bundle XDG cache, refreshes on changed/added/removed files, and rebuilds on schema-version mismatch; `--reindex` forces a rebuild.
- [ ] `--ephemeral` writes nothing; okq falls back to ephemeral when the cache dir is unwritable.
- [ ] Zero hits → exit 0; empty/invalid query → exit 2; index/bundle failure → exit 1.
- [ ] No full bodies emitted (token-frugal); fully non-interactive; output snapshot-tested (`insta`), with index nondeterminism controlled (fixed corpus, stable tie-break).
- [ ] Works on this repo's own `docs/` tree; nothing is written into the bundle.

## Open questions

- **Stemming escape hatch** — stemming is the v1 default (recall); do we add a per-field raw/exact tokenizer (or a `--exact` flag) so ID/code queries can opt out of stemming, beyond quoting?
- **Multi-term default** — OR-with-scoring (chosen); do we add `--all`/`--any`, or lean on query syntax (`+term`)?
- **search × find** — let `search` take `find`'s predicates (`--type`, `--tag`, `--where`) to rank *within* a filtered set? Tantivy faceting makes this natural; worth doing if the combined query is common.
- **Bundle-key derivation & cleanup** — how to key the XDG cache dir (path hash) so it's stable but collision-free, and when to garbage-collect stale per-bundle caches (an `okq cache clear`?).
- **Staleness granularity** — `(mtime, size)` manifest (chosen for v1) vs. content hashing (robust to mtime quirks, e.g. CI checkouts); incremental vs. always-full rebuild.
- **Concurrency** — writer-lock contention: wait, fail fast, or auto-fall-back to ephemeral?
- **Snippet representation** — elision/length, and whether JSON should mark matched spans (offsets) rather than emit plain text.
- **Explicit index command** — a `reindex` subcommand beyond the flag? Avoid the name `index` (collides with OKF's `index.md` generation).
- **Schema/version stability** — `okq.search/v1` is an agent contract; pin it and bump deliberately (PLAN.md §8).

## Related

- [find](find.md) — the unranked predicate counterpart; shares the collection-envelope top shape
- [get](get.md) — the section model reused here; `search → get --section` is the core expand step
- [ADR-0002](../adrs/0002-library-stack.md) — Tantivy as the day-one search backend, the index-as-derived-cache rule, `pulldown-cmark` sections
- [ADR-0003](../adrs/0003-search-index-in-xdg-cache.md) — the search index lives in the XDG cache, not in the bundle
- [PLAN.md](../../PLAN.md) — §2 the context-assembly wall, §3 token-frugal output, §5 `search` vs `find`, §6 architecture, §8 index-lifecycle open questions
