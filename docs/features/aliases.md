---
type: feature
title: Aliases — resolve concepts by their frontmatter aliases
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [aliases, obsidian, resolution, graph, get]
milestone: null
command: null # not a command — resolution behavior consumed by get / graph / wikilinks
related: ["wikilinks.md", "get.md", "graph.md", "../adrs/0011-aliases-in-resolution.md", "../guides/design-overview.md"]
---

# Aliases — resolve concepts by their frontmatter `aliases`

## Summary

okq resolves a caller-supplied identity against a concept's frontmatter
`aliases:` in addition to its filename/id. So `okq get Hooman` finds
`Work/Coworkers/Other/Hooman Bahador.md` when that file declares
`aliases: [Hooman]`, and a `[[Hooman]]` wikilink resolves to it (forming a
`wikilink` edge instead of a dead link) — matching how Obsidian resolves aliases
in its quick-switcher and its `[[...]]` links.

## Motivation

Obsidian's `aliases:` are first-class alternate names for a note: the
quick-switcher finds a note by any alias, and `[[alias]]` links resolve to it.
okq ignores aliases entirely — resolution is filename-only, in two places:
`resolve_concept` (`model.rs`, the partial-id resolver behind `get` / `neighbors`
/ `backlinks` / `path`) and `name_index` (`graph.rs`, wikilink by-name
resolution).

The cost shows up immediately on a real vault. `okq get Hooman` fails even though
`[[Hooman]]` is a meaningful, resolvable reference in Obsidian. And wikilinks
that target an alias — `[[Hooman]]`, `[[Lori]]`, `[[Nitro]]`, `[[WestJet]]` —
are reported as **dead links** purely because okq can't see the alias. Aliases
are exactly the kind of okf-adjacent frontmatter metadata that okq's query layer
should honor, the way it already honors frontmatter relations for the graph.

## Scope

### In scope

- Read `aliases:` from frontmatter (okf gives us the frontmatter mapping).
  Accept Obsidian's shapes: a YAML list (`aliases: [a, b]` or a block list) and
  a single scalar (`aliases: a`). Empty/absent is a no-op.
- Feed aliases into resolution in **two** places, case-insensitively:
  1. **`resolve_concept`** (`model.rs`) — the partial-id resolver behind `get`,
     `neighbors`, `backlinks`, `path`, and every graph node argument. After
     exact-id and segment-suffix matching fail, an alias match is a candidate.
  2. **Wikilink name resolution** (`name_index` / `resolve_wikilink`,
     `graph.rs`) — a bare `[[alias]]` resolves to the aliased concept, so it
     becomes a `wikilink` edge and drops out of `deadlinks`.
- Deterministic, documented precedence and ambiguity handling (see Behavior).

### Out of scope

- **Aliases as display titles.** `concept_title` (`model.rs`) is unchanged — an
  alias is a resolution key, not a rename; the title stays the frontmatter
  `title` or the filename.
- **The wikilink *display* alias `[[Note|Alias]]`.** That is link display text,
  already parsed off and discarded by [[wikilinks]] — unrelated to frontmatter
  `aliases:`. This spec is only about the frontmatter field.
- **Editing aliases.** Read/query only.
- **Alias-boosted full-text ranking** in `search` (indexing aliases as
  searchable text). Deferred — see open questions.

## Behavior

Aliases are a **resolution source**, not a command; no new flags, no output
shape changes. The resolved record is always the real concept, by its true id
and path.

- **Precedence (the key decision).** Exact concept id wins; then segment-suffix
  filename match (today's behavior); then **alias match, lowest priority**. A
  real file named `X` must never be shadowed by another file's alias `X` — so
  aliases only fill gaps, never override a filename.
- **Ambiguity.** Two concepts declaring the same alias `X`: for
  `resolve_concept`, mirror the existing partial-id behavior — return
  `ConceptAmbiguous` listing the candidates. For wikilink `name_index`, mirror
  the existing bare-name tie-break — id-sorted first, for a deterministic graph.
- **Alias vs filename collision.** If `X` is both a filename and some other
  file's alias, the filename wins (precedence above); the alias is consulted only
  when no filename matches.
- **Case-insensitive**, like Obsidian. Aliases are indexed lowercased.
- `okq get <alias>`, `neighbors <alias>`, `backlinks <alias>`, `path <alias> …`
  all resolve. `deadlinks` no longer reports a wikilink that targets an alias.
- **Exit codes** unchanged: an alias that matches nothing is still `not found`
  (4); an ambiguous alias behaves like an ambiguous partial id today.

## Acceptance criteria

- [x] A concept with `aliases: [Hooman]` is returned by `get Hooman`,
  `neighbors Hooman`, `backlinks Hooman`, `path Hooman …`; the resolved id/path
  is the real file, not the alias string.
- [x] A `[[Hooman]]` wikilink forms a `wikilink` edge to that concept and no
  longer appears in `deadlinks`.
- [x] Matching is case-insensitive; a filename always beats an equally named
  alias; two concepts sharing an alias error in `resolve_concept` and tie-break
  deterministically for wikilinks.
- [x] Both list and scalar `aliases:` shapes are read; empty/absent is a no-op;
  malformed frontmatter never panics.
- [x] Unit + integration + `insta` snapshots, including a fixture where an alias
  collides with a real filename (filename must win).

## Open questions

- **Surface the alias hit?** In human mode, a stderr note like
  `resolved "Hooman" via alias of Work/…/Hooman Bahador.md` is friendly; stdout
  stays clean data. **Lean:** stderr note in human mode only, nothing in `--json`.
- **`search` alias boosting.** Index aliases as searchable text so
  `search Hooman` ranks the aliased note. Deferred; noted so it isn't forgotten.
- **ADR needed. → Resolved:** the resolution-contract decision (precedence order;
  aliases below filenames; ambiguity semantics) is recorded in
  [ADR-0011](../adrs/0011-aliases-in-resolution.md).
- **Sequencing with [[phantom-links]].** Aliases should land **before** (or with)
  the phantom/broken split, so an alias target isn't first mislabeled a phantom
  and then reclassified.

## Related

- [wikilinks](wikilinks.md) — aliases complete its resolution story; its
  out-of-scope note about "alias semantics" refers to the *link display* alias,
  a different thing from the frontmatter `aliases:` this spec resolves.
- [get](get.md) — the primary consumer of `resolve_concept`.
- [graph](graph.md) — `neighbors`/`backlinks`/`path`/`deadlinks` all resolve
  through the same machinery aliases extend.
- [ADR-0011](../adrs/0011-aliases-in-resolution.md) — the resolution-contract
  decision (aliases below filename; case-insensitive; ambiguity) this spec builds on.
- [design overview](../guides/design-overview.md) — determinism and the
  resolution model this decision extends.
