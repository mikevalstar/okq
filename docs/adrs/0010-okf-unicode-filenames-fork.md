---
type: adr
title: ADR-0010 — Widen the okf fork to allow emoji and Unicode in file names
status: accepted
created: 2026-07-07
updated: 2026-07-07
tags: [okf, dependencies, fork, filenames, concept-id, unicode, emoji]
supersedes: "0009-okf-spaces-fork.md"
superseded-by: null
related:
  - "0009-okf-spaces-fork.md"
  - "0002-library-stack.md"
  - "../features/emoji-filenames.md"
  - "../features/graph.md"
  - "../guides/design-overview.md"
---

# ADR-0010: Widen the okf fork to allow emoji and Unicode in file names

## Context

[ADR-0009](0009-okf-spaces-fork.md) pinned okq's data layer to a fork of `okf`
([mikevalstar/okf](https://github.com/mikevalstar/okf)) that permits **interior
spaces** in a concept file name, so `Quarterly Report.md` loads and its
percent-encoded links resolve. That fix relaxed exactly one thing — the
concept-id segment rule in `okf`'s `concept_id.rs` — and taught link resolution
and index generation to round-trip percent-encoded spaces.

Spaces are only the first ASCII step past the reference rule
(`[A-Za-z0-9_][A-Za-z0-9_.\-]*`). Real bundles also carry **emoji** (`🚀 Launch.md`),
**accented Latin** (`café.md`), and **CJK** (`设计.md`) in file names. All of them
hit the same gate and are dropped by the same validation, for the same reason
spaces were. Widening once, deliberately, is cheaper than a fork per script.

Two facts make the wider change low-risk and let it land in the same data layer:

- The gate is a **single function**, `validate_segment`; nothing else in `okf`
  filters file names by character (bundle discovery does not).
- The link machinery is **already Unicode-correct**: `okf`'s `percent_decode`
  accumulates raw bytes and finishes with `from_utf8_lossy`, so a percent-encoded
  emoji link (`%F0%9F%9A%80`) reassembles to `🚀` with no further work. Spaces made
  this path exist; emoji reuse it unchanged.

This keeps the ADR-0009 posture from [ADR-0002](0002-library-stack.md) — `okf`
owns the data layer, okq does not reimplement it. The only questions are how wide
to open the character rule, and how to keep the widening cleanly mergeable
against a spaces change that is still under upstream review.

## Options considered

### Option A — Keep the spaces-only pin

Leave okq on the spaces commit and do without emoji/Unicode. Zero new
maintenance, but drops a class of real, human-authored filenames the tool is
meant to surface, and defers a change that is a few lines in the same place the
spaces change already lives.

### Option B — Emoji-only allowlist

Extend the segment rule to permit emoji specifically. Deceptively fiddly:
"is this an emoji" needs Unicode tables or a crate (against `okf`'s
zero-dependency design), and it still arbitrarily rejects `café.md` / `设计.md`
that every modern filesystem stores fine.

### Option C — Permissive denylist (chosen)

Flip the segment rule from an ASCII **allowlist** to a **denylist**: allow any
character *except* the ones that break paths or round-tripping — control
characters, the path separators `/` and `\`, the Windows-reserved set
(`: * ? " < > |`), and a leading `.`/`-` or a leading/trailing space. Everything
else — emoji (first character included), accents, CJK, `_`/`.`/`-`/space —
is allowed. One rule, no dependency, and it **subsumes** the spaces change.

## Decision

**Option C.** okq pins its `okf` dependency to a **permissive-filenames branch**
of the fork whose `validate_segment` is the denylist above.

- Because the denylist already permits spaces, this branch is a **superset** of
  the spaces branch, not a stack on top of it. Both are sibling branches off the
  same upstream base (`W4G1/okf`'s initial commit), so upstream can adopt either
  the conservative (spaces-only) or the permissive rule as *one or the other*,
  and okq simply pins to whichever it wants live. The spaces branch — the pending
  upstream ticket — is left untouched.
- The dependency stays a **git dependency pinned to a specific commit**
  (`1999badeb5679442e2157c458af96d07f84c9587`, the `permissive-filenames` branch
  tip), not a floating branch, so builds are deterministic and `Cargo.lock` pins
  one source (okq's reproducibility principle). This supersedes the ADR-0009 pin.
- One okq-side change ships with it: the graph's dead-link check must
  **percent-decode** a link target before deciding it is out of scope, so a
  *broken* encoded link (`Quarterly%20Reprot.md`, `%F0%9F…`) is still reported by
  [`deadlinks`](../features/graph.md) instead of silently slipping through. The
  data layer resolves *working* encoded links; okq owns catching the broken ones.
- Still **temporary**, same as ADR-0009. Exit condition: an upstream `okf`
  release that permits these filenames. When it lands, revert `Cargo.toml` to the
  crates.io release, re-run the suite, and mark this ADR superseded.

## Consequences

- **Emoji, accented, and CJK filenames work end to end.** Such concepts load and
  are surfaced by `get` / `find` / `search` / graph like any other, because the
  fix is in the data layer — no okq query-side change beyond the dead-link decode.
- **Encoded broken links are caught.** After the decode fix, a mistyped `%20`- or
  `%F0%9F…`-encoded link is reported by `deadlinks`, closing the gap the spaces
  work left (working encoded links resolved; broken ones were invisible).
- **A wider charset means wider inputs to the doc graph.** Values that were inert
  because they failed the old id rule (e.g. a free-text `supersedes:` containing
  `()`) may now parse as segments; re-run `okq --bundle docs deadlinks` after the
  re-pin and reconcile anything newly surfaced.
- **We still carry a pinned git dependency**, so okq still cannot cut a crates.io
  release while this is in force (crates.io disallows git deps). Same accepted
  trade as ADR-0009; the pin is one line to revert.
- **Reproducibility is preserved.** A pinned commit plus committed `Cargo.lock`
  resolves the same `okf` on every build.

## Related

- [ADR-0009](0009-okf-spaces-fork.md) — the spaces fork this supersedes; the
  permissive rule is a superset of it
- [ADR-0002](0002-library-stack.md) — `okf` owns the data layer; this changes
  *which* build we consume, not the data/query split
- [emoji-filenames.md](../features/emoji-filenames.md) — the okq-visible capability and its acceptance criteria
- [graph.md](../features/graph.md) — `deadlinks`, whose decode fix ships here
- [design-overview.md](../guides/design-overview.md) — the data/query split this preserves
