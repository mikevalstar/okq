---
type: feature
title: Wikilinks — Obsidian-style [[links]] as graph edges
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [graph, links, wikilinks, obsidian, edges]
milestone: null
command: null # not a command — an edge source consumed by the graph commands
related: ["graph.md", "../guides/design-overview.md"]
---

# Wikilinks — Obsidian-style `[[links]]` as graph edges

## Summary

okq recognizes **Obsidian-style wikilinks** (`[[Note]]`, `![[Note]]`, and their
alias / heading / block / path variants) in concept bodies and turns each into a
`wikilink` graph edge. Bundles authored in Obsidian — or anything that uses the
`[[…]]` convention — become fully navigable with `neighbors`, `backlinks`,
`path`, `orphans`, and `deadlinks`, not just bundles that use CommonMark
`[text](dest)` links. Resolves [issue #5](https://github.com/mikevalstar/okq/issues/5).

## Motivation

okf — okq's data layer — models cross-links as CommonMark links only. A large
share of real knowledge bases (every Obsidian vault, plus most wiki-flavored
note systems) express their links as `[[wikilinks]]` instead. Without wikilink
support those bundles look almost edgeless to okq: `neighbors` returns nothing,
everything is an `orphan`, and the graph commands — okq's whole differentiator
(see [graph](graph.md)) — go dark. Rather than push this upstream and wait, okq
does a small second parse of the body it already has from okf and contributes
the extra edges itself.

## Scope

### In scope

- A dependency-free scanner (`src/wikilinks.rs`) that extracts wikilink targets
  from a concept body, skipping fenced code blocks and inline code spans.
- The full spread of Obsidian internal-link shapes:
  `[[Note]]`, `[[Note|Alias]]`, `[[folder/Note]]`, `[[Note.md]]`,
  `[[Note#Heading]]`, `[[Note#^block-id]]`, `[[Note#Heading|Alias]]`,
  same-note `[[#Heading]]` (no edge), and embeds `![[Note]]` / `![[Note#…]]`.
- **Lenient resolution** (issue #5): a bare `[[Users]]` matches a concept named
  `users` *anywhere* in the bundle, case-insensitively (Obsidian's by-name
  resolution); a `/`-bearing target is read as a bundle-root-relative path first,
  then relative to the source concept.
- A new `wikilink` edge kind, filterable with `--edge wikilink` and reported by
  `deadlinks` when a wikilink resolves to nothing in the bundle.

### Out of scope

- **Link rewriting / editing.** okq stays read/query only.
- **Alias and heading semantics.** The alias and `#heading`/`#^block` anchor are
  parsed off and discarded — the edge is note-to-note; anchors don't change which
  concept is referenced.
- **A distinct `embed` edge kind.** Transclusions (`![[…]]`) are recognized but
  recorded as ordinary `wikilink` edges; splitting them out is an open question.
- **Ambiguity ranking.** When two concepts share a bare name, okq picks the
  id-sorted first for determinism rather than Obsidian's "shortest path" heuristic.

## Behavior

Wikilinks are not a command; they are an **edge source** consumed by the existing
graph commands. Once a bundle contains `[[…]]` links:

- `okq neighbors <c>` / `okq backlinks <c>` include concepts reached by
  wikilinks, each carrying `edge: "wikilink"`.
- `okq neighbors <c> --edge wikilink` restricts to wikilink edges;
  `--edge link` restricts to CommonMark links (the two coexist).
- `okq path <a> <b>` may route over wikilink edges.
- `okq deadlinks` reports a wikilink whose target matches no concept, with the
  written target as `raw` and `edge: "wikilink"`. `--check` still exits 3.
- `okq orphans` no longer flags a concept that is only reached by wikilinks.

Resolution is deliberately lenient (issue #5): bare names match by filename
case-insensitively across the bundle; paths tolerate `.md`, `.`/`..`, and a
leading `/`. A wikilink that is a plausible in-bundle reference but resolves to
nothing is a dead link; one that escapes the bundle root is out of scope, not
dead — mirroring how CommonMark links are treated. Deduped per source, so the
same `[[Note]]` written twice is one edge.

## Acceptance criteria

- [x] Bare, aliased, heading, block, path, and embed wikilinks all produce a
  `wikilink` edge to the right concept; same-note `[[#heading]]` produces none.
- [x] Bare-name resolution is case-insensitive and finds a concept in any
  subdirectory (`[[users]]` → `tables/users`).
- [x] `--edge wikilink` filters to these edges; `backlinks` shows them inbound.
- [x] An unresolvable in-bundle wikilink is reported by `deadlinks` with edge
  `wikilink`; `--check` exits 3.
- [x] Wikilinks inside code fences / inline code are ignored; malformed
  (unterminated) `[[` never panics.
- [x] Deterministic ordering and ambiguity tie-break; unit + integration tests
  with fixtures.

## Open questions

- **A separate `embed` edge kind** for `![[…]]` transclusions, so callers can
  distinguish "references" from "includes". Held off to keep the taxonomy lean;
  revisit if a use case appears.
- **Ambiguous bare names.** okq picks the id-sorted first; Obsidian uses the
  shortest vault path. Worth aligning if it bites in practice.
- **Should `wikilink` fold into `link`?** Kept separate so `--edge` can tell the
  two syntaxes apart and so stats can show the mix; an alias could unify them.

## Related

- [graph](graph.md) — the commands wikilink edges feed; wikilinks are the third
  edge source alongside inline links and frontmatter relations. This spec also
  points at [[graph]] as a live wikilink, so `okq --bundle docs neighbors
  wikilinks --edge wikilink` demonstrates the feature on our own docs.
- [design overview](../guides/design-overview.md) — §8 edge-taxonomy / graph-reuse
