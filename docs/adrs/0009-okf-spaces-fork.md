---
type: adr
title: ADR-0009 — Track a fork of okf until it allows spaces in file names
status: superseded
created: 2026-07-07
updated: 2026-07-07
tags: [okf, dependencies, fork, filenames, concept-id]
supersedes: null
superseded-by: "0010-okf-unicode-filenames-fork.md"
related:
  - "0002-library-stack.md"
  - "../guides/design-overview.md"
---

# ADR-0009: Track a fork of okf until it allows spaces in file names

> **Superseded by [ADR-0010](0010-okf-unicode-filenames-fork.md): the fork is
> widened to permit emoji and arbitrary Unicode in file names, and okq re-pins to
> that permissive-filenames branch.** The permissive rule is a superset of the
> spaces rule below, so this ADR's decision (depend on the fork temporarily,
> pinned to a commit) still holds — only the pinned commit and the breadth of the
> character rule change. The spaces branch itself remains the pending upstream
> ticket; see ADR-0010 for why the two are kept as sibling branches.

## Context

[ADR-0002](0002-library-stack.md) makes `okf` a hard dependency and the sole
owner of okq's **data layer** — parse, frontmatter, the concept model, and the
link graph. okq queries what `okf` loads; it does not reimplement it.

The published `okf` (`0.1.0-alpha.1` on crates.io) forbids spaces in a concept
file name: its concept-id validation rejects any segment containing an interior
space, so a document like `Quarterly Report.md` is not a valid concept and is
dropped. For a documentation-first tool pointed at real repos and human-authored
`docs/` trees, that is a real limitation — space-containing filenames are common,
and okq can only surface what `okf` is willing to load.

A fork addresses this: [mikevalstar/okf#1](https://github.com/mikevalstar/okf/pull/1)
(forked from [W4G1/okf](https://github.com/W4G1/okf)) relaxes concept-id
validation to permit **interior** spaces in a segment — segments must still begin
with an alphanumeric or underscore and may not have leading/trailing spaces — and
teaches link resolution and index generation to round-trip percent-encoded spaces
(`%20`) so cross-links to space-containing files still resolve. It keeps `okf`'s
zero-dependency design and adds test coverage; the change is not yet in an
upstream release.

We want the capability now without abandoning the "reuse `okf`, don't fork the
data layer" posture from ADR-0002. The question is how to depend on the fix while
it lives only on a branch.

## Options considered

### Option A — Wait for an upstream okf release

Keep the crates.io dependency and do without spaces in filenames until okf ships
the change. Zero maintenance, but blocks a capability that is already written and
tested, on an upstream release cadence we don't control.

### Option B — Reimplement space handling in okq

Route around `okf` by relaxing our own id/link handling on top of the loader.
Directly contradicts ADR-0002 (okf owns the data layer) and duplicates
validation/link logic okq deliberately does not own — the exact reimplementation
that ADR warns against.

### Option C — Depend on the fork, pinned to a commit, until upstream ships it

Point the `okf` dependency at `mikevalstar/okf`, pinned to the fork commit, and
treat it as a **temporary** swap: revert to the crates.io release the moment
upstream okf allows spaces in file names. Gets the capability now, keeps the
data layer in `okf`, and the pin keeps builds reproducible.

## Decision

**Option C.** okq depends on the `mikevalstar/okf` fork for its data layer until
upstream `okf` allows spaces in file names.

- The dependency is a **git dependency pinned to a specific commit**
  (`cedbbc76841e7afdc61fe5c060128675ab0c883f`), not a floating branch, so the
  build is deterministic and `Cargo.lock` pins one resolved source — consistent
  with okq's determinism / reproducible-build principle.
- This is explicitly **temporary**. The exit condition is a released upstream
  `okf` (`W4G1/okf`, via crates.io) that permits spaces in concept file names.
  When that lands, revert the `Cargo.toml` dependency to the crates.io release,
  re-run the suite, and mark this ADR superseded.
- The swap does not change okq's architecture: `okf` still owns the data layer
  (ADR-0002). We are changing *which* build of `okf` we consume, not the
  data/query split.

## Consequences

- **Spaces in filenames work end to end.** Concepts like `Quarterly Report.md`
  load, and links to them resolve, so okq's `get` / `find` / `search` / graph
  commands surface them like any other concept — no okq-side code change, because
  the fix lives in the data layer where it belongs.
- **We carry a git dependency for now.** A pinned-commit git source is less tidy
  than a crates.io version and would block a `cargo publish` of okq (crates.io
  disallows git dependencies). Accepted while this is in force: okq is not cutting
  a crates.io release that depends on the fork, and the pin is a single line to
  revert.
- **A clear, bounded exit.** The revert is mechanical and this ADR names its
  trigger, so the fork can't quietly become permanent. Track upstream okf for a
  release carrying the space-filename support.
- **Reproducibility is preserved.** Pinning to a commit (plus committed
  `Cargo.lock`) means every build resolves the same `okf`, same as before.

## Related

- [ADR-0002](0002-library-stack.md) — makes `okf` the data layer; this ADR swaps the build of `okf` without changing that split
- [design-overview.md](../guides/design-overview.md) — the data/query split this preserves
- [mikevalstar/okf#1](https://github.com/mikevalstar/okf/pull/1) — the fork change adding optional spaces in file names
