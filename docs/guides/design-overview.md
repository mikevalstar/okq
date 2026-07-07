---
type: guide
title: okq design overview
status: active # draft | active | deprecated
created: 2026-06-27
updated: 2026-06-27
tags: [design, architecture, principles, overview]
audience: both # dev | agent | both
related:
  - ../adrs/0001-documentation-first-okf-shaped.md
  - ../adrs/0002-library-stack.md
  - ../adrs/0008-scope-non-goals.md
  - ../adrs/0009-okf-spaces-fork.md
  - ../features/search.md
  - ../features/graph.md
---

# okq design overview

## Purpose

The durable picture of what okq is, why it exists, and how it's shaped — the
stable reference that outlives any single feature. This document **supersedes the
former `PLAN.md`**: the milestone roadmap is done (it now lives as release history
in [CHANGELOG.md](../../CHANGELOG.md)), the command surface is specified in
[features/](../features/), and the standing design decisions are ADRs.
Older docs that cite "PLAN.md §N" refer to sections now folded into this overview,
the feature specs, and the ADRs.

## Background

### Vision

`okq` is the **query and navigation layer** for [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)
(OKF) document bundles — collections of Markdown files with YAML frontmatter,
cross-linked into a knowledge graph. It makes a large OKF bundle *answerable*: by
a person at a terminal and by an AI agent assembling context, using the **same
deterministic, local, no-API tool**.

### The problem it removes

Markdown + frontmatter is the simplest portable way to give humans and agents
shared, curated knowledge — but the format outran its tooling:

- **The context-assembly wall.** An agent must assemble relevant knowledge before
  it can act. Past ~100 documents, a bundle's `index.md` no longer fits in a
  context window, and an agent reading files linearly "loses the middle." The fix
  is *programmatic* multi-stage retrieval — search, then traverse links — but it's
  hand-rolled per agent otherwise.
- **Humans pay the same tax.** "Which ADRs are security-related?" or "what depends
  on this concept?" means improvising `fd` + `yq` + `rg` pipelines — not
  repeatable, not shareable.
- **No other OKF tool queries.** Surveyed tooling covers validate / visualize /
  author. None do search or graph-aware query. That is okq's gap to fill.

### Differentiator

Search-by-tag is table stakes. okq's unique value is **graph-aware navigation**
over the cross-link structure frontmatter and OKF naturally provide: neighbors and
N-hop neighborhoods, backlinks, shortest path between concepts, orphans and dead
links, and bundle-level structure metrics. This maps onto how humans and agents
actually explore a knowledge base: *start somewhere relevant, then move outward.*

## The guide

### Design principles

These are the standing constraints every command honors. Several are enforced by
their own ADR.

1. **Deterministic & local-first.** Frontmatter + lexical + graph queries only. No
   embeddings, no vector store, no network calls in a query. Same bundle → same
   answer, every time. (The one opt-in exception is `okq skills install
   --from-repo`, scoped and documented in [ADR-0007](../adrs/0007-opt-in-network-for-skill-install.md);
   vectors stay deferred per [ADR-0008](../adrs/0008-scope-non-goals.md).)
2. **Agent-runnable.** Every command has a fully non-interactive path and `--json`
   output, so it's a clean tool-call for an LLM ([ADR-0004](../adrs/0004-exit-code-taxonomy.md)
   pins the exit-code contract).
3. **Token-frugal output by default.** The win for agents is *precision +
   locations*, not content dumps. Results default to a ranked shortlist of
   `path:line` + frontmatter + a short snippet — never full bodies. The caller
   expands what it chooses via `get`.
4. **Fast & dependency-light.** Rust, leaning on the upstream [`okf`](https://crates.io/crates/okf)
   crate for the data layer ([ADR-0002](../adrs/0002-library-stack.md)); okq adds
   the *query* surface rather than reimplementing the parser. (okq currently tracks
   a temporary fork of `okf` for spaces-in-filenames support, per
   [ADR-0009](../adrs/0009-okf-spaces-fork.md).)
5. **Format-tolerant.** Targets OKF v0.1 but degrades gracefully on any
   Markdown-with-frontmatter tree, so it's useful before a repo formally adopts OKF.
6. **Composable.** Plays well with `jq`/`fzf`/`fd`; output is a stream of paths or
   JSON records, not a walled UI.

### Command surface

The two retrieval modes are the core ergonomic: `search`/`find` answer *"where do
I start?"*; `neighbors`/`backlinks`/`path` answer *"what's connected to here?"*.
The expensive agent loop — find one doc, then read outward to map an area —
collapses into **`search → neighbors → get`**.

Each command has its own spec under [features/](../features/):
`search`, `find`, `get`, `neighbors`/`backlinks`/`path`/`orphans`/`deadlinks`
([graph](../features/graph.md)), `stats`, `schema`, `init`/`new`
([scaffold](../features/scaffold.md)), and `skills`
([skills-install](../features/skills-install.md)). Global flags: `--json`,
`--bundle <dir>`, `--no-color`, `--no-ignore`.

Two design choices that cut across the surface:

- **Section-granular chunking.** Index and return at heading-delimited section
  level, not whole-file, so snippets stay tight and `get --section` expands
  precisely.
- **Typed edges.** Graph edges carry a type, sourced from frontmatter relations
  (`supersedes`, `related`, `depends-on`, …) and inline links (generic `link`),
  so "what *supersedes* this?" beats an untyped blob of connected docs.

### Architecture

```
okq (bin)
 ├─ cli        — arg parsing, output formatting (human table + --json)
 ├─ query      — frontmatter predicates, tag/type/where filters
 ├─ search     — ranked lexical retrieval over sections (persisted Tantivy BM25 index)
 ├─ graph      — neighbors / backlinks / path / orphans / deadlinks; typed edges
 └─ okf (dep)  — parse, model, validate, link-graph  ← upstream crate
```

One load of the bundle feeds three views (query, search, graph). The search
backend is a persisted **Tantivy BM25** index ([ADR-0002](../adrs/0002-library-stack.md)),
kept as a derived cache in a per-bundle **XDG cache directory**
(`~/.cache/okq/<bundle-key>/`, never inside the bundle —
[ADR-0003](../adrs/0003-search-index-in-xdg-cache.md)), rebuilt from the concept
docs and never a source of truth.

### Where things are tracked now

- **Releases:** [CHANGELOG.md](../../CHANGELOG.md) (the roadmap's milestones became
  release history).
- **Decisions:** [adrs/](../adrs/).
- **Command behavior & open questions:** the per-command [features/](../features/)
  specs — open questions live in the spec they belong to, not a central list.
- **Scope boundaries (what okq won't do):** [ADR-0008](../adrs/0008-scope-non-goals.md).

## Gotchas

- This overview is *reference*, not a plan. It describes okq as it is. New work is
  a feature spec or an ADR, not an edit here — keep it stable.
- okq queries its own `docs/` bundle ([ADR-0005](../adrs/0005-dogfood-okq-for-docs.md)).
  Keep this tree OKF-shaped and cross-linked so that keeps working.

## References

- [OKF specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) — the format okq targets.
- [`okf` crate](https://crates.io/crates/okf) — the data layer okq builds on.
- Prior art: `GoogleCloudPlatform/knowledge-catalog` (spec + reference impls),
  `W4G1/okf` (pure-Rust OKF library + CLI), `scaccogatto/okf-skills` (Claude Code
  plugin for author/validate/visualize), and Karpathy's "LLM Wiki" note and the
  surrounding interlinked-markdown-KB ecosystem — the demand signal okq serves.
- [ADR index](../adrs/), [feature index](../features/), [CHANGELOG](../../CHANGELOG.md).
