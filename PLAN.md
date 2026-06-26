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

1. **Deterministic & local-first.** Pure frontmatter + graph queries. No embeddings, no vector store, no network calls. Same bundle → same answer, every time.
2. **Agent-runnable.** Every command has a fully non-interactive path and `--json` output, so it's a clean tool-call for an LLM. (Mirrors the bidirectional human/agent contract OKF itself sets.)
3. **Fast & dependency-light.** Rust. Lean on the upstream [`okf`](https://crates.io/crates/okf) crate for parsing, the data model, validation, and the link graph; `okq` adds the *query* surface on top rather than reimplementing the parser.
4. **Format-tolerant.** Targets OKF v0.1, but should degrade gracefully on OKF-*shaped* bundles (any Markdown-with-frontmatter tree, e.g. an existing `docs/adrs` folder) so it's useful before a repo formally adopts OKF.
5. **Composable.** Plays well with `jq`/`fzf`/`fd`; output is a stream of paths or JSON records, not a walled UI.

## 5. Command surface (draft)

| Command | Purpose |
|---|---|
| `okq find` | Filter concepts by frontmatter — `--tag`, `--type`, `--where field=value`, `--match` (substring/regex on title/body). |
| `okq neighbors <concept>` | Adjacent concepts via links; `--depth N`, `--direction in\|out\|both`. |
| `okq backlinks <concept>` | Concepts that link *to* this one. |
| `okq path <a> <b>` | Shortest link path between two concepts. |
| `okq orphans` | Concepts with no inbound links (stale-doc candidates). |
| `okq deadlinks` | Links pointing to missing/renamed concepts. |
| `okq stats` | Bundle overview: counts by type/tag, link density, hub concepts. |
| `okq get <concept>` | Print one concept's frontmatter and/or body (`--frontmatter`, `--body`, `--json`). |

Global flags: `--json`, `--bundle <dir>` (default: cwd), `--no-color`, exit codes that are script-friendly.

## 6. Architecture sketch

```
okq (bin)
 ├─ cli        — arg parsing, output formatting (human table + --json)
 ├─ query      — frontmatter predicates, tag/type/where filters
 ├─ graph      — neighbors / backlinks / path / orphans / deadlinks
 └─ okf (dep)  — parse, model, validate, link-graph  ← upstream crate
```

Open question: how much of `okf`'s graph is reusable vs. what `okq` must build. First milestone validates that.

## 7. Milestones

- **M0 — Spike (this).** Repo, plan, README. Evaluate the `okf` crate: does its parser + link graph give us the primitives, or do we vendor/fork? Decide MSRV, error model, output schema.
- **M1 — Read & find.** Load a bundle via `okf`; implement `find` (tag/type/where/match) + `get`, with `--json`. Dogfood against a real `docs/` tree.
- **M2 — Graph.** `neighbors`, `backlinks`, `path`, `orphans`, `deadlinks` over the link graph.
- **M3 — Health & stats.** `stats`, CI-friendly checks (orphans/deadlinks as non-zero exit), stable JSON schemas documented.
- **M4 — Release.** Publish `okq` to crates.io; prebuilt binaries; install docs. (Name confirmed free as of 2026-06-26.)
- **Later — Agent ergonomics.** Optional MCP server (`okq mcp`) exposing find/neighbors/path as tools; `--watch` for incremental reindex.

## 8. Open questions

- Reuse depth of the `okf` crate vs. building our own index.
- Concept identity: file path (OKF) vs. a frontmatter `id` — support both?
- How strict on conformance — query a non-conformant/OKF-shaped bundle anyway, with warnings?
- JSON schema versioning (agents will depend on output stability — treat it like a contract from day one).

## 9. Prior art / references

- OKF spec & reference impls — `GoogleCloudPlatform/knowledge-catalog`
- `W4G1/okf` — pure-Rust OKF library + CLI (validate/graph/parse/fmt) — likely dependency
- `scaccogatto/okf-skills` — Claude Code plugin (author/validate/visualize)
- Karpathy's "LLM Wiki" note and the surrounding 2026 interlinked-markdown-KB ecosystem — the demand signal `okq` serves.
