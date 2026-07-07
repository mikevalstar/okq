---
type: feature
title: Inline tags — Obsidian #tags in the body as first-class tags
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [tags, obsidian, find, stats, indexing]
milestone: null
command: null # not a command — a tag source consumed by find / stats / search / get
related: ["wikilinks.md", "find.md", "stats.md", "../guides/design-overview.md"]
---

# Inline tags — Obsidian `#tags` in the body as first-class tags

## Summary

okq recognizes **inline `#tag` tokens** in a concept body and treats them as
tags, unified with the frontmatter `tags:` list. So `find --tag`, `stats`, and
the concept envelope (`get`/`find`/`search` records) reflect a bundle's *real*
tag set — the one an Obsidian author sees — where inline `#tags` and frontmatter
`tags:` share a single namespace.

## Motivation

Obsidian unifies two tagging mechanisms: frontmatter `tags:` and inline `#tag`
anywhere in the note body. Both feed the same tag pane, both are searchable, and
many vaults tag almost entirely inline. okq today reads **only** frontmatter
`tags()` (`model.rs`, `stats.rs`, `find.rs`, `search.rs` all call
`frontmatter.tags()`), so a tag-driven vault looks nearly untagged.

Concretely, run against a real vault: a note whose body contains `#KGPortal` is
invisible to `okq find --tag KGPortal`, and `#Obsidian` never appears in
`okq stats`. This is the same shape of gap that [[wikilinks]] closed for links —
the data is sitting in the body okf already hands us; okq does a small second
parse and contributes the missing signal, rather than pushing it upstream and
waiting.

## Scope

### In scope

- A dependency-free scanner (a sibling of `src/wikilinks.rs`) that extracts
  inline `#tag` tokens from a concept body, reusing the same code-safety
  discipline: **skip fenced code blocks and inline code spans**.
- Obsidian's tag grammar, so we match what Obsidian matches and nothing else:
  - a tag is `#` immediately followed by at least one **letter** (Unicode
    letter), then letters, digits, `-`, `_`, and `/` for nesting (`#area/work`);
  - **not** a tag: `#` followed by a space (an ATX Markdown heading `# Title`),
    a pure number (`#123`, so issue/PR refs are left alone), a `#` inside a URL
    fragment (`example.com/#section`), or a hex color (`#fff`, `#0a0a0a` — they
    start with a digit or are all hex-after-`#`; excluded because a color is not
    a letter-led token… see open questions for the `#deadbeef` edge).
- **Merging** inline tags with frontmatter tags at every point tags are
  consumed: `ConceptRecord.tags` (`model.rs`), the `stats` tag distribution, the
  `find --tag` predicate, `search` hit records, and the Tantivy tag field, so
  the search facet and `stats` counts stay honest. Deduped and sorted.

### Out of scope

- **Editing / adding tags.** okq stays read/query only.
- **Tag hierarchy queries.** Matching `#area/work` when you ask for `--tag area`
  (prefix / parent rollup, which Obsidian does) is deferred — see open questions.
- **Frontmatter `tags:` parsing.** okf already owns that; we only *add* the body
  source and merge.

## Behavior

Inline tags are not a command; they are a **tag source**, the way [[wikilinks]]
are an edge source. Once a bundle has body `#tags`:

- `okq find --tag KGPortal` matches a concept whose body contains `#KGPortal`
  (subject to the case decision below), not just frontmatter-tagged ones.
- `okq stats` counts inline tags in the `Tags:` distribution alongside
  frontmatter tags.
- The `tags` array in `get`/`find`/`search` `--json` records is the **union** of
  frontmatter and inline tags, deduped and deterministically ordered.
- Exit codes are unchanged; this only widens what counts as a tag.

## Acceptance criteria

- [x] Inline `#tag`, nested `#a/b`, and tags with `-`/`_` are extracted; `#123`,
  `# Heading`, URL fragments, and code-fenced/inline-code `#tags` are **not**.
- [x] `find --tag <t>` matches inline-tagged concepts; `stats` counts them; the
  `--json` `tags` array is the deduped, sorted union with frontmatter tags.
- [x] Case handling is consistent and documented (see open questions) across
  `find --tag`, `stats`, and record output.
- [x] Malformed input never panics; a fixture with `#tags` in code fences and a
  heading confirms they are ignored. Unit + integration + `insta` snapshots.

## Open questions

- **Case normalization. → Resolved:** inline tags are lowercased by the scanner
  (`#KGPortal` → `kgportal`); frontmatter tags keep their spelling; the merged
  list dedupes case-insensitively (frontmatter spelling wins) and preserves order
  (frontmatter declaration order, then inline document order — not sorted, so
  existing output is undisturbed). `find --tag` matches case-insensitively, so
  `--tag kgportal` and `--tag KGPortal` both hit.
- **Nested-tag prefix matching.** `--tag area` matching `#area/work`. Deferred;
  exact-match only for now.
- **Provenance.** Should a record distinguish a tag that came from the body vs
  frontmatter? Probably not worth it — one merged list is simpler. Revisit only
  if a caller needs it.
- **The `#deadbeef` / all-hex edge.** A word tag that happens to be all hex
  characters is indistinguishable from a CSS color by grammar alone. Obsidian
  treats `#deadbeef` as a tag (it leads with a letter). **Lean:** letter-led ⇒
  tag; only `#` + digit-led is excluded — accept the rare false positive.
- **ADR?** This widens the tag-namespace contract but is additive and
  low-risk; a feature spec should suffice. Flag if we'd rather record it as an
  ADR.

## Related

- [wikilinks](wikilinks.md) — the sibling "second parse of the body" feature;
  inline tags are to `find`/`stats` what wikilinks are to the graph.
- [find](find.md) — `--tag` is the main consumer; its predicate widens to the
  merged set.
- [stats](stats.md) — the `Tags:` distribution counts inline tags too.
- [design overview](../guides/design-overview.md) — token-frugal, deterministic
  output principles the merged tag list must still honor.
