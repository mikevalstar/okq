---
type: feature
title: Optional frontmatter — infer title from filename
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [frontmatter, title, okf, get, find, search, robustness]
milestone: null
command: null # cross-cutting behavior, not a single command
related: ["get.md", "find.md", "search.md", "stats.md", "index-command.md", "../adrs/0001-documentation-first-okf-shaped.md", "../guides/design-overview.md"]
---

# Optional frontmatter — infer title from filename

## Summary

A concept file with **no YAML frontmatter** is a valid concept. Everywhere okq
surfaces a concept `title`, a frontmatter-less file now reports its **filename**
(the concept id's last segment) as the title, so it is titled, searchable, and
navigable like any other concept — without inventing frontmatter that isn't
there.

## Motivation

OKF only *requires* a non-empty `type` for spec conformance (§9), and the `okf`
loader is permissive: a file that doesn't begin with a `---` delimiter parses
with an **empty frontmatter and the whole text as the body** — it is not a parse
error and already loads as a concept. But okq read `title` straight from the
frontmatter, so these files showed up everywhere with an empty title: a blank
column in `find`, no title boost in `search`, `null` in the JSON envelopes.

Plenty of real bundles are just folders of Markdown notes with no frontmatter at
all. They should be first-class: the filename is a perfectly good title, and it's
the one piece of identity every file already has. This closes [issue #6](https://github.com/mikevalstar/okq/issues/6).

## Scope

### In scope

- Deriving a display `title` for any concept whose frontmatter has no (or an
  empty) `title`: use the concept id's final segment (the filename minus `.md`),
  **verbatim**.
- Applying that fallback consistently in `find`, `get`, `search` (including the
  Tantivy title field so inferred titles are matched and boosted), and `stats`.

### Out of scope

- **Humanizing** the filename. `my-note` stays `my-note`; we do not title-case,
  de-slugify, or strip numeric prefixes. If a human title is wanted, add a
  `title` to the frontmatter — that is the explicit signal.
- **Synthesizing frontmatter.** `get --frontmatter` (and the JSON `frontmatter`
  object) reflect the file's *true* frontmatter — empty stays empty. The inferred
  title is a display value, not a data rewrite.
- Changing what counts as a concept. `okf` already decides that; files with
  unterminated/`non-mapping` frontmatter remain parse errors, and reserved
  `index.md`/`log.md` remain non-concepts. **No `okf` change is required.**

## Behavior

- **Resolution.** `title` = the frontmatter `title` if present and non-empty,
  otherwise the concept id's last segment. Every concept has a non-empty id
  segment, so a title is **always** available.
- **Output.** The `title` field in the `find`, `get`, `search`, and `stats` JSON
  envelopes is now **always present** (a plain string, no longer nullable) — an
  agent never has to special-case a missing title. Human output shows the same
  value in the title column.
- **Search.** The inferred title is indexed in the Tantivy `title` field, so a
  query matching a frontmatter-less file's name ranks it via the same title boost
  as any other concept.
- **`get --frontmatter`.** Unchanged: prints only the real frontmatter block, so
  a frontmatter-less file prints an empty one. The inferred title never leaks
  into the frontmatter surface.
- **Exit codes.** Unchanged (shared taxonomy).

## Acceptance criteria

- [ ] A `.md` file with no frontmatter loads as a concept and appears in `find`.
- [ ] Its `title` (JSON and human) is the filename minus `.md`, verbatim — no
      humanizing.
- [ ] `okq search` matches and title-boosts that file by its inferred title.
- [ ] `okq get <id> --json` reports the inferred `title`, while
      `okq get <id> --frontmatter` prints an empty frontmatter block and the JSON
      `frontmatter` object omits the title.
- [ ] An explicit frontmatter `title` still wins over the filename.
- [ ] Malformed input still degrades gracefully (no panic); frontmatter-less is
      not malformed.

## Related

- [okq get](get.md), [okq find](find.md), [okq search](search.md), [okq stats](stats.md), [okq index](index-command.md)
- [ADR-0001 — documentation-first, OKF-shaped](../adrs/0001-documentation-first-okf-shaped.md)
- [design overview](../guides/design-overview.md)
