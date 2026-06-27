---
type: adr
title: ADR-0001 — Documentation-first, in an OKF-shaped docs tree
status: accepted
created: 2026-06-26
updated: 2026-06-26
tags: [process, documentation, okf, dogfooding]
supersedes: null
superseded-by: null
related: ["../README.md", "../../PLAN.md"]
---

# ADR-0001: Documentation-first, in an OKF-shaped docs tree

## Context

`okq` is in planning / pre-alpha ([PLAN.md](../../PLAN.md)): the design is rich and the code is nonexistent. Decisions are being made now — Rust, the upstream `okf` crate, the search backend, the edge-type taxonomy — that are expensive to reverse and easy to forget the *why* of. We want a place to capture those decisions and the feature/workflow designs alongside (or ahead of) the code.

There is also a second, project-specific pull: `okq` is itself a query and navigation tool for [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) bundles — Markdown + frontmatter, one concept per file, cross-linked. Any docs tree we build is *already* the shape of `okq`'s input. We can either build ordinary docs, or build docs that double as a real test corpus.

## Options considered

### Option A — Code-first, document later

Write the tool, backfill docs near release. Cheapest up front. But it loses decision rationale (the most perishable artifact), gives reviewers nothing to react to before code exists, and produces no corpus to dogfood against until late.

### Option B — Documentation-first, in an OKF-shaped `docs/` tree

Adopt the proven four-type layout (adrs / features / workflows / guides), each with a `_template.md` and YAML frontmatter, mirroring a structure that already works in a sibling project. Additionally constrain it to be OKF-conformant (frontmatter on every doc, one concept per file, cross-links as graph edges) so `okq` can be run against its own documentation — the dogfooding [M1 in PLAN.md](../../PLAN.md) already calls for.

## Decision

**Option B.** This repo is documentation-first, and the `docs/` tree is intentionally OKF-shaped. Conventions and frontmatter schema are defined in [docs/README.md](../README.md); each doc type has a `_template.md`. Where the repo's needs diverge from OKF v0.1, that divergence is recorded as its own ADR rather than silently bending the format.

## Consequences

- Decision rationale is captured while it's fresh; reviewers and agents can react to designs before code exists.
- `okq` gets a free, real, evolving test corpus — its own docs — so search and graph commands can be validated against a bundle that actually changes over time.
- Small ongoing tax: every doc carries frontmatter and links, and divergences from OKF must be justified in an ADR rather than waved through.
- Opens a follow-up question for [PLAN.md](../../PLAN.md) §8: when the OKF spec moves, this tree (and the templates) must move with it — keeping conformance is now a maintenance commitment, not a one-time setup.

## Related

- [docs/README.md](../README.md) — the conventions and frontmatter schema this ADR ratifies
- [PLAN.md](../../PLAN.md) — vision, command surface, and the M1 dogfooding milestone
