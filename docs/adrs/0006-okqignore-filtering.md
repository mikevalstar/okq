---
type: adr
title: ADR-0006 — .okqignore for excluding files from a bundle
status: accepted
created: 2026-06-27
updated: 2026-06-27
tags: [ignore, bundle, filtering, config]
supersedes: null
superseded-by: null
related:
  - ../features/okqignore.md
  - ../features/stats.md
  - 0003-search-index-in-xdg-cache.md
---

# ADR-0006: `.okqignore` for excluding files from a bundle

## Context

okf loads a bundle by walking the directory tree and parsing every non-reserved
`.md` file (`index.md`/`log.md` excepted). It has no notion of exclusion — every
markdown file under the root is a concept. That is correct for okf (the data
layer) but wrong for some real bundles:

- **This repo.** `docs/tests/` holds deliberately malformed fixtures. They exist
  to prove okq degrades gracefully, but they pollute `stats` (inflated
  `parse_errors`, fake orphans), `orphans`, and `search` results against our own
  `docs/` bundle.
- **Any bundle** mixing knowledge docs with scratch notes, drafts, vendored
  copies, or generated files that shouldn't count as concepts.

There is no way today to say "these files are in the tree but not part of the
bundle". Users want the familiar `.gitignore` lever. okf won't grow this — it's
a query-layer concern, not a format one — so okq owns it.

Two things make this a real decision rather than an obvious one: it adds a
dependency and defines matching semantics (expensive to reverse once people
write `.okqignore` files against them), and it changes what "the bundle" means
for every command, which ripples into the derived search index (ADR-0003).

## Options considered

### Option A — Simple globs (`globset`)

A single `.okqignore` at the root, plain glob patterns matched against concept
paths. Light, few surprises in the matcher itself.

- **Pro:** small surface; easy to reason about.
- **Con:** users will *expect* `.gitignore` behavior — negation (`!keep.md`),
  anchoring (`/drafts` vs `drafts`), `**`, per-directory files. Globs silently
  lack all of that, so the tool feels broken precisely when someone reaches for
  a non-trivial pattern. We'd be reimplementing gitignore badly.

### Option B — Full gitignore semantics (`ignore` crate)

Use ripgrep's `ignore` crate (`ignore::gitignore::Gitignore`) — the same engine
git users already know: comments, negation, anchoring, `**`, and per-directory
files with the usual precedence (deeper files and later patterns win).

- **Pro:** zero surprises; "it works like `.gitignore`" is the whole spec.
  Battle-tested matcher, so we don't own the edge cases. The crate matches
  *paths we hand it* — it doesn't have to drive the walk — so it composes with
  okf's existing load.
- **Con:** one more dependency (well-maintained, already transitively common in
  the Rust CLI ecosystem). Nested files mean we apply matchers in ancestor order
  ourselves, since we feed paths rather than letting the crate walk.

### Option C — A `[ignore]` list in bundle config / root `index.md`

Put exclusions in frontmatter or a config file instead of a dotfile.

- **Con:** invents okq-specific config where a well-known convention exists;
  worse ergonomics; still needs a matcher. No upside over B.

## Decision

**Option B.** A `.okqignore` file uses full `.gitignore` syntax via the `ignore`
crate. Specifics:

- **Nested, per-directory.** A `.okqignore` may sit in any directory; it governs
  that directory and below, with standard gitignore precedence (the deepest
  matching file wins; within a file, the last matching pattern wins). The
  root file is the common case; nesting is there for the same reason git has it.
- **Global filter.** Ignored files are removed from *the bundle*, not from a
  subset of commands. `search`, `find`, `get`, `stats`, `orphans`, `deadlinks`,
  and the graph all see the same reduced concept set. One mental model:
  *ignored = not in the bundle*. `get` on an ignored id returns "not found"
  (exit 4); links pointing at an ignored concept become dead links, exactly as
  if the file were deleted.
- **`--no-ignore` escape hatch.** A global flag disables all `.okqignore`
  processing, so the full tree is visible. This is how okq's own integration
  tests keep exercising `docs/tests/` fixtures, and how a user audits what their
  ignore rules are hiding.
- **Filtering lives in the okq query layer**, applied to the concept list okf
  returns. okf stays ignorant of exclusion. okq exposes a single filtered view
  so commands don't each re-implement the filter.
- **The search index honors it (ADR-0003).** The Tantivy cache is built from the
  filtered concept set. To stay correct: the effective ignore mode is part of
  the cache key (default vs `--no-ignore` are separate caches, so toggling never
  reads a mismatched index), and every `.okqignore` file's stamp goes into the
  staleness manifest (editing ignore rules rebuilds the index). The index is
  still a derived cache in the XDG dir, never written into the bundle.

## Consequences

- **Easier:** a clean bundle view by default; `okq stats` against `docs/` stops
  counting fixtures; users get a lever they already understand.
- **Harder / committed to:** we now own a small amount of nested-precedence glue
  and a new dependency. "The bundle" is mode-dependent (`--no-ignore` changes
  results) — every command's behavior section and `--json` contract must note
  that ignore filtering applies. The cache key gains a dimension.
- **Determinism preserved:** same tree + same `.okqignore` files + same mode →
  same answer. No network, no heuristics; the matcher is deterministic.
- **Follow-ups:** `okq init` could scaffold a starter `.okqignore`; `stats`
  could report how many files were ignored (transparency, so a silent filter
  never hides a typo). Both deferred to the feature spec's open questions.

## Related

- [okqignore feature spec](../features/okqignore.md) — the user-facing behavior.
- [ADR-0003 — search index in XDG cache](0003-search-index-in-xdg-cache.md) — the
  derived cache this must stay consistent with.
- [stats](../features/stats.md) — the command most distorted by un-ignored fixtures.
