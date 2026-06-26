# okq — Project Plan

> Living design document. Created 2026-06-26. Status: **planning / pre-alpha.**

## 1. Vision

`okq` is the **query and navigation layer** for [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) (OKF) document bundles — collections of Markdown files with YAML frontmatter, cross-linked into a knowledge graph.

It exists to make a large OKF bundle *answerable*: by a person at a terminal and by an AI agent assembling context, using the **same deterministic, local, no-API tool**.

## 2. Problem statement

OKF (and the broader "LLM-wiki" pattern it standardizes) is spreading because Markdown + frontmatter is the simplest portable way to give humans and agents shared, curated knowledge. But the format outran its tooling:

- **The context-assembly wall.** Agents must assemble relevant knowledge before they can act. Past ~100 documents, the bundle's `index.md` no longer fits in a context window, and an agent that reads files linearly "loses the middle." The accepted fix is *programmatic* multi-stage retrieval — search across files, then traverse links — but it's hand-rolled per agent today.
- **Humans pay the same tax.** Answering "which ADRs are security-related?" or "what depends on this concept?" means improvising `fd` + `yq` + `rg` pipelines (with mise-shim PATH gotchas and quoting pain). Not repeatable, not shareable.
- **No existing OKF tool queries.** Surveyed tooling — `GoogleCloudPlatform/knowledge-catalog`, `W4G1/okf` (Rust lib + CLI: validate/info/index/graph/parse/fmt), `scaccogatto/okf-skills` (Claude Code plugin: author/maintain/validate/visualize) — covers **validate / visualize / author**. **None do search or graph-aware query.** That is the gap.

## 3. Differentiator

Search-by-tag is table stakes. The unique value is **graph-aware navigation** over the cross-link structure frontmatter + OKF naturally provide:

- neighbors / N-hop neighborhood of a concept
- backlinks (inbound references)
- shortest path between two concepts
- orphans (no inbound links) and dead links (point to missing concepts)
- bundle-level structure metrics (link density, most-connected hubs, tag distribution)

No other OKF tool does this, and it maps directly onto how both humans and agents actually explore a knowledge base: *start somewhere relevant, then move outward.*

## 4. Design principles

1. **Deterministic & local-first.** Pure frontmatter + lexical + graph queries. No embeddings, no vector store, no network calls in v1. Same bundle → same answer, every time. (Vectors are a deliberately deferred, evidence-gated option — see §7 and §8 — not a permanent "never.")
2. **Agent-runnable.** Every command has a fully non-interactive path and `--json` output, so it's a clean tool-call for an LLM. (Mirrors the bidirectional human/agent contract OKF itself sets.)
3. **Token-frugal output by default.** The token win for agents comes from *precision + locations*, not content dumps. Results default to a ranked shortlist of `path:line` + frontmatter (id/type/title) + a 1–2 line snippet — never full bodies. The caller expands what it chooses via `get`. Graph traversal returns nodes as `id + title + edge-type + path:line`, not doc contents. A tool that pre-dumps matched files is *less* efficient than `rg`; the whole design fights that.
4. **Fast & dependency-light.** Rust. Lean on the upstream [`okf`](https://crates.io/crates/okf) crate for parsing, the data model, validation, and the link graph; `okq` adds the *query* surface on top rather than reimplementing the parser.
5. **Format-tolerant.** Targets OKF v0.1, but should degrade gracefully on OKF-*shaped* bundles (any Markdown-with-frontmatter tree, e.g. an existing `docs/adrs` folder) so it's useful before a repo formally adopts OKF.
6. **Composable.** Plays well with `jq`/`fzf`/`fd`; output is a stream of paths or JSON records, not a walled UI.

## 5. Command surface (draft)

| Command | Purpose |
|---|---|
| `okq search <query>` | **Ranked** full-text retrieval over titles/headings/body. Returns a scored shortlist of section-level hits with `path:line` + snippet — the backbone for "find the doc(s) about X" at scale. Distinct from `find`: `search` *ranks by relevance*, `find` *filters by exact predicate*. |
| `okq find` | Filter concepts by frontmatter — `--tag`, `--type`, `--where field=value`, `--match` (substring/regex on title/body). Set-membership, not ranking. |
| `okq neighbors <concept>` | Adjacent concepts via links; `--depth N` (default **1**), `--direction in\|out\|both`, `--edge <type>` (filter by link relation, e.g. `supersedes`, `related`, `depends-on`). |
| `okq backlinks <concept>` | Concepts that link *to* this one (the inbound view you can't get by reading the doc itself). |
| `okq path <a> <b>` | Shortest link path between two concepts. |
| `okq orphans` | Concepts with no inbound links (stale-doc candidates). |
| `okq deadlinks` | Links pointing to missing/renamed concepts. |
| `okq stats` | Bundle overview: counts by type/tag, link density, hub concepts, edge-type distribution. |
| `okq get <concept>` | Print one concept's frontmatter and/or body (`--frontmatter`, `--body`, `--section <heading>`, `--json`). The expand-on-demand counterpart to `search`/`neighbors` shortlists. |

**Authoring / onboarding commands** — query is the point, but a bundle has to *exist* before it can be queried, and the chicken-and-egg of "what does a conformant bundle even look like?" is a real adoption barrier:

| Command | Purpose |
|---|---|
| `okq init` | Scaffold a starter OKF bundle in an empty/existing dir: the standard folder layout (decisions / features / guides / …), a seed `index.md`, a `README` explaining the bundle, and the OKF-conformant frontmatter conventions wired in. Gets a repo from zero to a queryable, conformant skeleton in one command. |
| `okq new <type> [title]` | Create a single concept from the matching template (`decision`, `feature`, `guide`, …) with frontmatter pre-filled (id, type, created date) and a body skeleton. The repeatable "add one more doc" counterpart to `init`. |

Templates ship embedded in the binary (no template-dir bootstrapping problem) and degrade to OKF v0.1 defaults; a bundle can override them with its own `.okq/templates/` later if needed. **Positioning honesty:** scaffolding/authoring overlaps existing tooling (`scaccogatto/okf-skills`, `okf fmt`) — `okq`'s angle is *one integrated tool* where the same thing that scaffolds a bundle also queries and navigates it, not a novel capability.

Global flags: `--json`, `--bundle <dir>` (default: cwd), `--no-color`, exit codes that are script-friendly.

**Search vs. graph — the two retrieval modes.** `search`/`find` answer *"where do I start?"*; `neighbors`/`backlinks`/`path` answer *"what's connected to here?"*. The expensive agent loop — find one doc, then grep-and-read outward to map an area — collapses into `search → neighbors → get`. That composition is the core ergonomic.

**Chunking.** Index and return at **section granularity** (heading-delimited), not whole-file. OKF docs have clean headings + frontmatter; exploiting that keeps snippets tight and lets `get --section` expand precisely.

**Edge types.** Graph edges are *typed*, sourced from (a) frontmatter relations (`supersedes`, `related`, `depends-on`, …) and (b) inline links, with backlinks derived. Typed traversal ("what *supersedes* this?") beats an untyped blob of 30 connected docs. The taxonomy is an open question (§8) — start with whatever the bundle's frontmatter actually uses, plus a generic `link` for inline references.

## 6. Architecture sketch

```
okq (bin)
 ├─ cli        — arg parsing, output formatting (human table + --json)
 ├─ query      — frontmatter predicates, tag/type/where filters
 ├─ search     — ranked lexical retrieval over sections (persisted Tantivy BM25 index)
 ├─ graph      — neighbors / backlinks / path / orphans / deadlinks; typed edges
 └─ okf (dep)  — parse, model, validate, link-graph  ← upstream crate
```

`search` backend is **decided: a persisted Tantivy BM25 index** ([ADR-0002](docs/adrs/0002-library-stack.md)), built from the same parse pass that feeds `query` and `graph` — one load of the bundle, three views over it. The index is a derived, git-ignored cache under `.okq/index/` (rebuilt from the concept docs, never source of truth); what remains open is its lifecycle (staleness/invalidation, format-version pinning, writer-lock) — see §8. The full library stack lives in [ADR-0002](docs/adrs/0002-library-stack.md).

Open question: how much of `okf`'s graph is reusable vs. what `okq` must build. First milestone validates that.

## 7. Milestones

- **M0 — Spike (this).** Repo, plan, README. Evaluate the `okf` crate: does its parser + link graph give us the primitives, or do we vendor/fork? Decide MSRV, error model, output schema.
- **M1 — Read, find & search.** Load a bundle via `okf`; implement `find` (tag/type/where/match), section-level ranked `search`, and `get` (incl. `--section`), all with `--json` and `path:line` output. Dogfood against a real `docs/` tree.
- **M2 — Graph.** `neighbors`, `backlinks`, `path`, `orphans`, `deadlinks` over the link graph, with typed edges and a default depth of 1.
- **M3 — Health & stats.** `stats`, CI-friendly checks (orphans/deadlinks as non-zero exit), stable JSON schemas documented.
- **M3.5 — Scaffold & author.** `okq init` (starter bundle: layout + seed `index.md` + embedded templates) and `okq new <type>` (single doc from template). Closes the loop — `okq` can now *create* the bundles it queries, lowering the adoption barrier.
- **M4 — Release.** Publish `okq` to crates.io; prebuilt binaries; install docs. (Name confirmed free as of 2026-06-26.)
- **M4.5 — Agent skills.** Ship a small set of bundled, installable agent skills (see §9) — one teaching agents to *navigate* a bundle via `okq` (search → neighbors → get), one explaining the OKF *format* itself (referencing the Google spec as the canonical base). Distributed alongside the binary so adopting `okq` also onboards the agents that use it.
- **Later — Agent ergonomics.** Optional MCP server (`okq mcp`) exposing search/neighbors/path as tools; `--watch` / incremental reindex on changed files (hash/mtime) to keep any index from going stale.
- **Later, evidence-gated — semantic retrieval.** *Only if* observed real queries miss on vocabulary mismatch (query terms that never appear literally in the relevant doc) does vector search earn its cost. Even then: add it as a **second retriever fused with the lexical one (RRF)**, not a replacement — pure-vector blurs exactly the exact-token queries (IDs, API names, error codes) that dominate technical bundles. Local embedding model only, to preserve the local-first/no-network principle. This is explicitly *not* a v1 concern.

## 8. Open questions

- Reuse depth of the `okf` crate vs. building our own index.
- Concept identity: file path (OKF) vs. a frontmatter `id` — support both?
- How strict on conformance — query a non-conformant/OKF-shaped bundle anyway, with warnings?
- JSON schema versioning (agents will depend on output stability — treat it like a contract from day one).
- ~~`search` backend: in-memory BM25 vs. SQLite FTS5 vs. Tantivy~~ — **decided: persisted Tantivy** ([ADR-0002](docs/adrs/0002-library-stack.md)). What's still open is the *index lifecycle*: staleness/invalidation policy (mtime vs. content hash; partial vs. full reindex), index-format version pinning, and writer-lock behavior when two `okq` processes share an index.
- Edge-type taxonomy: which relations are first-class (`supersedes`, `related`, `depends-on`, generic `link`)? Derive from frontmatter conventions, or define a fixed set and map onto it?
- Vector deferral: what concrete signal (a query miss-rate threshold? a corpus size?) flips the decision to add semantic retrieval — so it stays evidence-gated, not vibes-gated.
- Templates: embedded-in-binary defaults vs. a bundle-local `.okq/templates/` override — and how `okq new`'s templates stay in lockstep with the OKF version `init` scaffolds.
- Skill packaging: Claude Code skill bundle first, but how portable should the skill *content* be (OKF model + retrieval loop) across agent/skill formats — and how does it stay pinned to a moving OKF spec without a rewrite each version?

## 9. Agent skills (bundled)

`okq` is a tool; a skill is the *instruction* that teaches an agent when and how to reach for it. Shipping both means adopting `okq` also onboards the agents that will use it. Two skills, kept deliberately small:

1. **`okf-navigate` — how to explore a bundle with `okq`.** Teaches the retrieval loop: start with `okq search` (ranked, locations-only), expand with `okq get --section`, then traverse with `okq neighbors`/`backlinks` to map an area — instead of reading files linearly and "losing the middle." Encodes the token-frugal contract (work from shortlists, expand on demand). This is the skill that makes the *tool* pay off.

2. **`okf-explain` — what the OKF format *is*.** Teaches an agent (or a human via the agent) the bundle anatomy: the folder taxonomy, required frontmatter, ID/naming conventions, the cross-link model, and how to author a conformant doc (pairs with `okq new`). **The Google OKF spec is the canonical base** the skill references and quotes.

**On the canonical reference, honestly:** OKF is young and Google-originated. The skill cites the [Google spec](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) as the source of truth *for now* — but the format is expected to evolve as the OSS community takes it up (cf. the W4G1/okf and okf-skills efforts already diverging on details). So the skill is written to **reference the spec by pointer, not by hardcoding the schema**, and pins the OKF version it targets, so that when the community standard moves, updating one pointer + version pin re-aligns the skill rather than a rewrite. The plan should *expect* the canonical reference to shift away from Google over time.

Format/packaging is an open question (§8): a Claude Code skill bundle is the obvious first target (cf. `scaccogatto/okf-skills`), but the *content* — the OKF mental model + the `okq` retrieval loop — should be portable to whatever skill/agent format matters, not welded to one vendor.

## 10. Prior art / references

- OKF spec & reference impls — `GoogleCloudPlatform/knowledge-catalog`
- `W4G1/okf` — pure-Rust OKF library + CLI (validate/graph/parse/fmt) — likely dependency
- `scaccogatto/okf-skills` — Claude Code plugin (author/validate/visualize)
- Karpathy's "LLM Wiki" note and the surrounding 2026 interlinked-markdown-KB ecosystem — the demand signal `okq` serves.
