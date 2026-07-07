---
type: feature
title: Emoji & Unicode in file names
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [filenames, unicode, emoji, okf, concept-id, deadlinks, graph, robustness]
milestone: null
command: null # cross-cutting: a data-layer widening plus one graph fix
related:
  - "../adrs/0010-okf-unicode-filenames-fork.md"
  - "../adrs/0009-okf-spaces-fork.md"
  - "graph.md"
  - "get.md"
  - "find.md"
  - "search.md"
  - "../guides/design-overview.md"
---

# Emoji & Unicode in file names

## Summary

A concept whose file name contains **emoji** (`🚀 Launch.md`), **accented Latin**
(`café.md`), or **CJK** (`设计.md`) is a valid concept. It loads and is surfaced by
`get` / `find` / `search` / graph like any other, and percent-encoded links to it
(`Q1%20%F0%9F%9A%80%20Launch.md`) both **resolve** and are **dead-link-checked**.

## Motivation

This continues the [spaces work](../adrs/0009-okf-spaces-fork.md): file names in
real, human-authored bundles are not ASCII. The upstream reference id rule
(`[A-Za-z0-9_][A-Za-z0-9_.\-]*`) rejects every emoji, accent, and CJK character,
so those files are dropped by the `okf` loader and okq can't surface what it never
loads. [ADR-0010](../adrs/0010-okf-unicode-filenames-fork.md) widens the fork's
one validation gate to a permissive denylist, which fixes the load path in the
data layer where it belongs.

The spaces work left one asymmetry: a *working* `%20` link resolved (`okf` decodes
it), but a *broken* one — a typo like `Quarterly%20Reprot.md` — was silently
absent from [`deadlinks`](graph.md), because okq re-derived the target from the
raw, still-encoded string without decoding it. Emoji links (`%F0%9F…`) would fall
in the same hole. That is the one okq-side fix this feature carries.

## Scope

### In scope

- **Loading** concepts with emoji/Unicode file names (via the ADR-0010 fork
  re-pin). The denylist rejects only control chars, `/`, `\`, `: * ? " < > |`, a
  leading `.`/`-`, and a leading/trailing space; a leading emoji is allowed.
- **Dead-link decode fix.** The graph decodes a percent-encoded link target
  before classifying it, so a broken encoded link is reported as a dead link
  instead of dismissed as out-of-scope. Working encoded links continue to resolve
  via the data layer unchanged.
- Tests: the fork's `validate_segment`/link tests plus okq integration coverage
  for an emoji concept (load, find, get, search, neighbors/backlinks) and a broken
  encoded link (`deadlinks`).

### Out of scope

- **Humanizing or normalizing** the name. `🚀 Launch` stays `🚀 Launch`; okq does
  not strip emoji, transliterate, or Unicode-normalize (NFC/NFD) — the segment is
  stored and displayed verbatim, as [title inference](frontmatter-optional-title.md)
  already does for filenames.
- **Re-implementing the character rule in okq.** The gate lives in `okf`
  (ADR-0002); okq consumes it. The re-pin is mechanical.
- **Search tokenization of emoji.** Emoji aren't word tokens; a concept is still
  found by its title (its file name) and body text as usual. No special ranking.

## Behavior

- **Load / find / get / search / graph.** An emoji/Unicode concept behaves like
  any other: it appears in `find`, resolves in `get` (by full id), is indexed and
  title-boosted in `search`, and participates in the graph. No okq query-side code
  changes — the widening is entirely in the data layer.
- **Links.** A percent-encoded link to the concept resolves to a real `link` edge
  (`okf` decodes multi-byte UTF-8 correctly). A raw-Unicode link (`[x](<🚀 note.md>)`)
  resolves the same way.
- **`deadlinks`.** A *broken* encoded link now appears in the `okq.deadlinks/v1`
  results with its `raw` target as written, alongside plain and frontmatter-relation
  dead links. `--check` still exits **3** when any are found.
- **Exit codes.** Unchanged (shared taxonomy).

## Acceptance criteria

- [ ] A file whose name contains an emoji loads as a concept and appears in `find`;
      a name that **begins** with an emoji loads too.
- [ ] Accented (`café`) and CJK (`设计`) file names load as concepts.
- [ ] `okq get "<emoji id>"` resolves by full id; `search` matches and title-boosts it.
- [ ] A percent-encoded link to an emoji concept resolves to a `link` edge
      (`neighbors` / `backlinks` traverse it).
- [ ] A **broken** percent-encoded link (`%20`- or `%F0%9F…`-encoded) is reported by
      `okq deadlinks`; `--check` exits 3.
- [ ] A name with a path-hostile character (`/`, control, `: * ? " < > |`) or a
      leading/trailing space is still rejected by the data layer (not a panic — a
      skipped parse error, surfaced by `validate`).
- [ ] Malformed input still degrades gracefully (no panic).

## Related

- [ADR-0010 — widen the okf fork to emoji/Unicode](../adrs/0010-okf-unicode-filenames-fork.md)
- [ADR-0009 — the spaces fork this builds on](../adrs/0009-okf-spaces-fork.md)
- [okq graph — deadlinks](graph.md), [okq get](get.md), [okq find](find.md), [okq search](search.md)
- [Optional frontmatter — infer title from filename](frontmatter-optional-title.md)
- [design overview](../guides/design-overview.md)
