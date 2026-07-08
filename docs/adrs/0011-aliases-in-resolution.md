---
type: adr
title: ADR-0011 — Frontmatter aliases participate in concept resolution, below filename
status: accepted
created: 2026-07-07
updated: 2026-07-07
tags: [aliases, obsidian, resolution, graph, concept-id]
supersedes: null
superseded-by: null
related:
  - "../features/aliases.md"
  - "../features/wikilinks.md"
  - "../guides/design-overview.md"
---

# ADR-0011: Frontmatter aliases participate in concept resolution, below filename

## Context

okq resolves a caller-supplied identity to a concept in one place —
`resolve_concept` (`model.rs`) — and resolves a bare `[[wikilink]]` name in
another — `name_index` / `resolve_wikilink` (`graph.rs`). Both resolve **by
filename only**: an exact concept id, then a segment-aligned filename suffix, then
(for wikilinks) a case-insensitive bare-name match. Frontmatter `aliases:` — an
Obsidian first-class feature, where a note declares alternate names that its
quick-switcher and `[[alias]]` links resolve to — is ignored entirely.

On a real Obsidian vault this bites immediately: `okq get Hooman` fails though
the note declares `aliases: [Hooman]`, and `[[Hooman]]`/`[[Lori]]`/`[[Nitro]]`
wikilinks are reported as dead links purely because okq can't see the alias. The
[aliases feature spec](../features/aliases.md) proposes honoring `aliases:` in
both resolvers. Because resolution is a **contract** — agents and scripts come to
depend on what `get <id>` / `neighbors <id>` accept, and on which edges the graph
forms — changing it is expensive to reverse once shipped, so the rule is recorded
here rather than left implicit in the implementation.

The decision is not *whether* to honor aliases (the spec settles that) but the
**precedence and ambiguity rules**, which must be deterministic and must not let
one note's alias silently shadow another note's real file.

## Options considered

### Option A — Aliases at the same tier as filenames

Fold aliases into the same by-name index as filenames, so a bare name matches a
filename or an alias with equal priority. Simple to implement, but a note that
aliases `Users` would compete head-to-head with a real `Users.md`, turning a
previously unambiguous `get Users` into an ambiguity error — or worse, silently
resolving to the aliased note. Surprising and reversal-prone.

### Option B — Aliases as the lowest-priority resolver

Consult aliases **only after** exact-id and filename-suffix matching find
nothing. A real file always wins; aliases fill gaps they don't currently fill.
No existing resolution changes outcome; the feature is purely additive.

### Option C — Don't resolve aliases; only stop reporting alias targets as dead

Narrower: teach `deadlinks` that an alias target isn't broken, without making
`get`/`neighbors` accept aliases. Fixes the false dead links but not the
findability gap (`get Hooman` still fails), and splits alias knowledge across
resolvers inconsistently. Half the value for similar cost.

## Decision

**Option B.** Frontmatter `aliases:` participate in resolution, at the **lowest
precedence**, in both resolvers:

1. **Precedence.** Exact concept id → segment-suffix filename match → **alias
   match**. Aliases are consulted only when filename resolution yields nothing, so
   a real filename can never be shadowed by another file's alias.
2. **Case-insensitive.** Alias matching lowercases both sides, matching Obsidian.
3. **Shapes.** Both a YAML list (`aliases: [a, b]`) and a single scalar
   (`aliases: a`) are read; empty/absent is a no-op.
4. **Ambiguity.** Two concepts declaring the same alias: `resolve_concept`
   returns the existing `ConceptAmbiguous` error listing candidates; wikilink
   by-name resolution takes the id-sorted first, matching how a bare-name
   filename collision is already tie-broken (deterministic graph).
5. **Aliases are a resolution key only** — never a display title (`concept_title`
   is unchanged) and never rewritten into output. This is distinct from the
   wikilink *display* alias `[[Note|Alias]]`, which is link display text and is
   already discarded (see [wikilinks](../features/wikilinks.md)).

## Consequences

- **Findability and graph fidelity improve for Obsidian bundles.** `get`,
  `neighbors`, `backlinks`, and `path` accept an alias; `[[alias]]` forms a real
  `wikilink` edge and drops out of `deadlinks`. No okq command changes shape.
- **Resolution stays deterministic and backward-compatible.** Because aliases sit
  below filenames, every input that resolves today resolves the same way; only
  inputs that previously failed can now succeed. A bundle with no `aliases:`
  frontmatter (e.g. okq's own `docs/`) is entirely unaffected.
- **We commit to the precedence order.** Once shipped, callers may rely on alias
  resolution, so a later reversal is breaking — hence this ADR. Moving aliases
  *above* filenames, or making an alias override a real file, would be a new ADR
  superseding this one.
- **A follow-on stays open:** boosting `search` full-text ranking by alias (so
  `search Hooman` ranks the aliased note) is deferred; it's recorded in the
  [aliases spec](../features/aliases.md), not committed here.

## Related

- [aliases feature spec](../features/aliases.md) — the behavior this ADR fixes the
  contract for
- [wikilinks](../features/wikilinks.md) — the other resolver aliases extend; its
  "alias semantics" out-of-scope note refers to the link *display* alias, a
  different thing
- [design overview](../guides/design-overview.md) — the deterministic,
  local-first resolution model this decision extends
