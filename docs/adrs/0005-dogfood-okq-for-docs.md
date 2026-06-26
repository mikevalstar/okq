---
title: ADR-0005 — Dogfood okq for our own docs, specs, and features
status: accepted
created: 2026-06-26
updated: 2026-06-26
tags: [process, dogfooding, documentation, testing]
supersedes: null
superseded-by: null
related: ["0001-documentation-first-okf-shaped.md", "../features/graph.md", "../../CLAUDE.md"]
---

# ADR-0005: Dogfood okq for our own docs, specs, and features

## Context

[ADR-0001](0001-documentation-first-okf-shaped.md) made this repo's `docs/` tree
an intentionally OKF-shaped bundle so okq could be run against its own
documentation. We've done that opportunistically (tests query `docs/`, we run ad
hoc searches), but the practice isn't yet a rule. Meanwhile we keep reaching for
`grep`/`rg`/manual file-reading to find related specs, check cross-links, and
navigate decisions — the exact pain okq exists to remove.

If we believe okq is the right way to query an OKF bundle, the most honest test is
to *use it that way ourselves*, on the bundle we touch every day. Dogfooding turns
real work into continuous validation and surfaces UX gaps as motivation, not
afterthoughts.

## Decision

**For documentation / spec / feature work in this repo, reach for okq first.**

- **Finding & reading docs:** use `okq search` (ranked) and `okq find`
  (predicate) instead of `grep`/`rg`/`fd` to locate specs, ADRs, and features;
  use `okq get --section` to read a specific part.
- **Navigating relationships:** use `okq neighbors` / `backlinks` / `path` (once
  M2 lands) to see what a doc connects to and what depends on it, rather than
  manually following links.
- **Doc health before committing:** use `okq deadlinks` / `okq orphans` (M2) to
  check that new cross-links resolve and no spec is left dangling, ideally wired
  into CI.
- **Keep `docs/` queryable:** the tree must stay OKF-shaped and cross-linked
  (frontmatter + `related:` + inline links) so these commands keep working — an
  obligation already established by ADR-0001.
- **Gaps are signal, not friction:** when okq can't yet do something we want for
  our own docs, that gap is the next piece of feature work. We fix our tool, not
  route around it.

**Honest bounds.** Dogfooding applies to what okq *can* do today; before a command
exists we fall back to ordinary tools (and note the gap). okq is read/query, not
authoring — editing docs stays a normal editor job until `init`/`new` land. We
don't contort a task to force okq through it.

## Consequences

- The `docs/` bundle is simultaneously our documentation **and** okq's primary
  real-world test corpus; changes to either are felt immediately.
- okq's UX is shaped by our own daily use — rough edges get found and filed fast.
- Early friction (missing commands) is expected and *intended* as roadmap pressure;
  it's why M2 graph navigation is prioritized.
- The development process (and CLAUDE.md) gain explicit "use okq" steps:
  locating related docs when writing a spec, and a link/orphan check before
  committing docs once those commands exist.
- A standing reason to keep the docs tree conformant; if okq can't parse or query
  our own docs, that's a release blocker, not a curiosity.

## Related

- [ADR-0001](0001-documentation-first-okf-shaped.md) — made `docs/` an OKF-shaped, dogfoodable bundle
- [graph](../features/graph.md) — the M2 commands (`neighbors`/`backlinks`/`deadlinks`/`orphans`) this practice leans on
- [CLAUDE.md](../../CLAUDE.md) — where the "use okq" steps live operationally
