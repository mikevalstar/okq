---
type: readme
title: okq documentation
---

# Documentation

This project is **documentation-first**: decisions, features, and workflows are written here before (or alongside) the code that implements them.

It is also, deliberately, **dogfooding**. `okq` is a query and navigation tool for [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) (OKF) bundles — collections of Markdown files with YAML frontmatter, cross-linked into a knowledge graph. This `docs/` tree is intentionally OKF-*shaped*: one concept per file, frontmatter on every doc, cross-links between them. That means `okq` is run against its own documentation as a real test corpus ([ADR-0005](adrs/0005-dogfood-okq-for-docs.md)). We eat our own dog food.

## Structure

| Folder | What goes here | When to write one |
|--------|----------------|-------------------|
| [adrs/](adrs/) | Architecture Decision Records — why we chose X over Y | Any time we pick a technology, library, pattern, or approach that would be expensive to reverse (Rust, the `okf` crate, the search backend, edge-type taxonomy…) |
| [features/](features/) | Feature specs — what a command/capability does, its scope, and its acceptance criteria | Before building a new user-visible command (`search`, `neighbors`, `init`…) |
| [workflows/](workflows/) | End-to-end flows `okq` supports, from the point of view of a human *or* an agent (e.g. "explore a bundle: search → neighbors → get") | When defining or changing how a user or agent accomplishes a goal |
| [guides/](guides/) | Developer guides — how to work on this repo, how the libraries we depend on behave, conventions, gotchas | When you learn something a future developer (or AI agent) will need |

Each folder contains a `_template.md` showing the expected format for that doc type. Copy it as the starting point for new docs.

## Frontmatter

Every doc starts with YAML frontmatter so docs can be searched, filtered, and graph-traversed programmatically — by `okq` itself, by other OKF tooling, and by agents:

```yaml
---
type: adr            # REQUIRED for OKF conformance — adr | feature | guide | workflow | …
title: Short human-readable title
status: draft        # draft | accepted | active | superseded | deprecated
created: 2026-06-26  # ISO date
updated: 2026-06-26  # ISO date, bump when meaningfully edited
tags: [cli, graph]   # lowercase, kebab-case
related: []          # paths to related docs — these become typed graph edges
---
```

`type` is the one **OKF-required** key (a bundle is conformant when every concept has a non-empty `type`); the rest are well-known OKF keys or this repo's producer extensions, which consumers preserve. Doc types add fields on top (ADRs have `supersedes`/`superseded-by`, features have `milestone`, etc.) — see each `_template.md`. The reserved `index.md` (directory listing; the root carries `okf_version`) and `log.md` are **not** concepts and carry no `type`.

The frontmatter here is kept compatible with [OKF v0.1](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) so this tree stays a valid query target. Where OKF and this repo's needs diverge, the divergence is itself an ADR.

## Conventions

- **ADRs are numbered**: `0001-documentation-first.md`, `0002-…`. Numbers are never reused.
- **Other docs are kebab-case**: `explore-a-bundle.md`.
- **One concept per file** — the OKF rule, and what makes the graph queries meaningful.
- **Cross-link liberally.** Use relative paths (`../adrs/0001-….md`) and the `related:` frontmatter field. Links are the edges `okq neighbors`/`backlinks`/`path` traverse; an unlinked doc is an orphan.
- **Decisions become immutable at commit, not at `accepted`.** While a doc is still uncommitted (being drafted in the working tree), edit it freely — even an `accepted` one. Once it's committed, history exists and others may rely on it: don't rewrite it — write a new ADR that supersedes it and flip the old one's `status` to `superseded` (and set `superseded-by`).
- **Statuses flow forward**: `draft` → `accepted`/`active` → `superseded`/`deprecated`.

## See also

- [design overview](guides/design-overview.md) — the durable design picture: vision, principles, architecture, and where everything is tracked now.
- [CHANGELOG.md](../CHANGELOG.md) — release history.
- [OKF specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) — the canonical format this project targets and this docs tree conforms to.
