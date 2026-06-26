# okq

**A fast, deterministic CLI for searching and navigating [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) (OKF) document bundles — for humans *and* AI agents.**

> Status: 🌱 **planning / pre-alpha.** This repo currently holds the vision and design ([PLAN.md](PLAN.md)). No code yet. Names, commands, and scope may still change.

---

## The problem

Modern engineering knowledge lives as large collections of Markdown files with YAML frontmatter — ADRs, decision logs, runbooks, design docs, internal wikis. Google's **Open Knowledge Format (OKF)** standardizes exactly this shape: Markdown + frontmatter, one concept per file, cross-linked into a knowledge graph.

The format is great. The *navigation* is not — for either audience:

- **Humans** can't easily ask "which decisions are security-related?", "what links to this doc?", or "what's orphaned and unmaintained?" without hand-rolling `grep`/`yq`/`fd` pipelines.
- **AI agents** hit a well-documented wall: past ~100 docs an `index.md` no longer fits in a context window, and an agent forced to read files sequentially "gets lost in the middle." The repeated recommendation is *programmatic* multi-stage retrieval — search across files, then follow the links — but no OKF tool ships that today. Existing OKF tooling only **validates**, **visualizes**, or **authors** bundles. None of them lets you *query* one.

`okq` fills that gap.

## What okq does (planned)

A single, scriptable command for asking questions of an OKF bundle:

```sh
okq find --tag security              # frontmatter filter across the whole bundle
okq find --where status=accepted     # arbitrary frontmatter predicates
okq neighbors orders.md --depth 2    # adjacent concepts via the link graph
okq backlinks customers.md           # what points *to* this concept
okq path orders.md revenue.md        # shortest link path between two concepts
okq orphans                          # concepts with no inbound links (likely stale)
okq stats                            # bundle overview: types, tags, link density
```

Everything supports `--json` so it doubles as a **retrieval primitive for an LLM agent** — no embeddings, no vector DB, no API key, fully deterministic and reproducible.

## Why it benefits humans *and* AI

OKF's whole premise is that the *same* Markdown serves people and agents. `okq` keeps that contract on the query side:

- **For people** — instant answers over a doc repo ("find the security ADRs", "what's downstream of this table?") instead of bespoke shell incantations.
- **For agents** — a fast, structured tool call that returns *just* the relevant nodes and their neighborhood, so the model spends its context on the right pages instead of scrolling an index.
- **For teams** — a healthier documentation flow: `okq` surfaces orphans, broken links, and untagged docs, turning "is our knowledge base any good?" into a command you can run in CI.

## Design principles

- **Deterministic & local-first** — graph + frontmatter queries, no ML, no network. Same inputs → same output.
- **Agent-runnable** — every command has a non-interactive path with `--json`.
- **Fast** — built in Rust, on top of the [`okf`](https://crates.io/crates/okf) crate's parser / model / link-graph.
- **Format, not platform** — works on any OKF (or OKF-shaped) bundle in any git repo or filesystem. No lock-in.

## Install

_Not yet published._ When it lands:

```sh
cargo install okq        # planned
```

## Status & roadmap

See [PLAN.md](PLAN.md) for the full design, command surface, and milestones.

## License

Licensed under the [Apache License 2.0](LICENSE) — matching OKF and the upstream `okf` crate.
