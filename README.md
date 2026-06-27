# okq

**A fast, deterministic CLI for searching and navigating [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) (OKF) document bundles — for humans *and* AI agents.**

> Status: 🌱 **alpha.** Retrieval (`get`, `find`, `search`), graph navigation (`neighbors`, `backlinks`, `path`, `orphans`, `deadlinks`), and bundle `stats` all work today. See [PLAN.md](PLAN.md) for the full roadmap.

---

## The problem

Engineering knowledge lives as large collections of Markdown files with YAML frontmatter — ADRs, decision logs, runbooks, design docs, wikis. Google's **Open Knowledge Format (OKF)** standardizes exactly this: Markdown + frontmatter, one concept per file, cross-linked into a knowledge graph.

The format is great; the *navigation* is not — for either audience:

- **Humans** can't easily ask "which decisions are security-related?" or "what's orphaned?" without hand-rolling `grep`/`yq`/`fd` pipelines.
- **AI agents** hit a wall: past ~100 docs an `index.md` no longer fits in context, and an agent reading files sequentially "gets lost in the middle." The fix is *programmatic* multi-stage retrieval — search, then follow links — but no OKF tool shipped that. Existing tooling only **validates**, **visualizes**, or **authors** bundles.

`okq` fills that gap: a single, scriptable, **local, deterministic** tool that both a person and an agent can use to *query* a bundle.

## Install

```sh
cargo install okq
```

## Quickstart

Point `okq` at any OKF (or OKF-*shaped*) Markdown tree with `--bundle` (default: the current directory).

```sh
# Rank sections across the bundle by relevance (BM25)
okq search "retrieval latency"

# Filter concepts by exact predicate — tags, type, frontmatter, text
okq find --type adr --tag security
okq find --where status=accepted

# Expand one concept — or just one section of it
okq get adrs/0006-agent-runnable-commands
okq get adrs/0006-agent-runnable-commands --section Decision
```

The core ergonomic is **locate, then expand** — `search`/`find` return locations-only shortlists; `get` expands what you choose:

```sh
# Find the most relevant section, then print it
path=$(okq search "xdg cache" --json | jq -r '.results[0].path')
okq get "${path%.md}" --section "The index lifecycle"
```

## Built for agents, too

Every command has a `--json` mode (one document on stdout, logs on stderr) and script-friendly exit codes, so each invocation is a clean tool-call for an LLM — no embeddings, no vector DB, no API key, fully reproducible.

```sh
okq search "auth" --json | jq -r '.results[].path'
okq find --tag security --json | jq '.count'
```

Output is **token-frugal by design**: results are ranked `path:line` + frontmatter + a short snippet — never full document dumps. The caller expands precisely what it needs with `get`.

## Commands

| Command | What it does |
|---------|--------------|
| `okq search <query>` | Ranked, section-level full-text retrieval (BM25). The "find the doc(s) about X" backbone. |
| `okq find` | Filter concepts by exact predicate: `--tag`, `--type`, `--where field=value`, `--match` (`--regex`). |
| `okq get <concept>` | Expand one concept: frontmatter and/or body, or a single `--section`. |
| `okq neighbors <concept>` | Adjacent concepts via the link graph: `--depth`, `--direction`, `--edge`. |
| `okq backlinks <concept>` | Concepts that link *to* this one (the inbound view). |
| `okq path <a> <b>` | Shortest link path between two concepts (`--undirected`). |
| `okq orphans` | Concepts with no inbound links (stale-doc candidates); `--check` for CI. |
| `okq deadlinks` | Links pointing at missing/renamed concepts; `--check` for CI. |
| `okq stats` | Bundle overview: counts by type/tag, link density, edge-type distribution, hubs. |

Run `okq <command> --help` for details and examples. The graph commands draw edges from **both** inline markdown links and frontmatter relations (`related`, `supersedes`, …).

## How it works

- **Deterministic & local-first** — pure frontmatter + lexical + (soon) graph queries. Same bundle → same answer, every time. No ML, no network.
- **Section-level** — documents are chunked by heading, so `search` ranks and `get --section` expands at the right granularity.
- **Ranked search** is a persisted [Tantivy](https://github.com/quickwit-oss/tantivy) BM25 index, cached per-bundle in your XDG cache directory (never written into the bundle). It auto-builds and refreshes when files change; `--reindex` forces a rebuild and `--ephemeral` runs fully in-memory.
- **Format-tolerant** — targets OKF v0.1 but degrades gracefully on any Markdown-with-frontmatter tree, and skips malformed docs instead of failing.
- **Built on the [`okf`](https://crates.io/crates/okf) crate** for parsing, the data model, and the link graph; `okq` adds the query surface.

## Exit codes

Scripts and CI can branch on `$?` without parsing output:

| Code | Meaning |
|------|---------|
| `0` | Success (including zero results — an empty query is not an error) |
| `2` | Usage error (bad flags, malformed `--where`, invalid query/regex) |
| `4` | Concept not found |
| `5` | Section not found / ambiguous |
| `1` | Other error (bad bundle, I/O, index failure) |

## Design & roadmap

`okq` is documentation-first. The full design, command surface, and decisions live in [PLAN.md](PLAN.md) and [docs/](docs/) (architecture decision records and feature specs).

## License

Licensed under the [Apache License 2.0](LICENSE) — matching OKF and the upstream `okf` crate.
