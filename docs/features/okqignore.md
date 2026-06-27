---
type: feature
title: .okqignore — exclude files from a bundle
status: active
created: 2026-06-27
updated: 2026-06-27
tags: [ignore, bundle, filtering, config]
milestone: M3
command: null
related:
  - ../adrs/0006-okqignore-filtering.md
  - stats.md
  - search.md
  - graph.md
---

# .okqignore — exclude files from a bundle

## Summary

A `.okqignore` file (full `.gitignore` syntax) marks markdown files that live in
the tree but are not part of the bundle. Ignored files disappear from every okq
command; `--no-ignore` reveals the full tree again.

## Motivation

A bundle's directory often holds more than its concepts: deliberately malformed
test fixtures (this repo's `docs/tests/`), drafts, scratch notes, vendored docs,
generated files. Today okf treats every `.md` as a concept, so those files
inflate `stats` (bogus `parse_errors`, fake orphans), clutter `search`, and show
up as orphans with no inbound links. There is no lever to say "in the tree, not
in the bundle". `.okqignore` is that lever, using the syntax everyone already
knows from git.

## Scope

### In scope

- A `.okqignore` file using full gitignore semantics (comments, negation `!`,
  anchoring `/`, `**`, per-directory files with standard precedence), via the
  `ignore` crate (see [ADR-0006](../adrs/0006-okqignore-filtering.md)).
- **Nested** files: a `.okqignore` in any directory governs that directory and
  below; the deepest matching file wins, and within a file the last matching
  pattern wins — exactly like git.
- **Global effect:** ignored files are removed from the bundle for *all*
  commands — `search`, `find`, `get`, `stats`, `orphans`, `deadlinks`,
  `neighbors`, `backlinks`, `path`.
- A global `--no-ignore` flag that disables all `.okqignore` processing.
- The search index (ADR-0003) reflects the filtered set and stays correct across
  modes and ignore-file edits.

### Out of scope

- Ignoring non-`.md` files (okf only ever loads markdown; nothing else is a
  concept anyway).
- A CLI to edit/generate `.okqignore` (write it in your editor). `okq init`
  scaffolding a starter file is a possible follow-up, not part of this spec.
- Honoring `.gitignore` itself. okq reads `.okqignore` only; the two are
  independent on purpose (you may want git to ignore something okq still indexes,
  and vice versa).
- Ignoring reserved files (`index.md`/`log.md`) — they are already not concepts.

## Behavior

### Invocation & flags

There is no `okqignore` subcommand; the file is ambient config that every
command reads. The only new surface is a global flag:

```sh
okq stats                       # fixtures under docs/tests/ excluded (if ignored)
okq --no-ignore stats           # full tree, nothing excluded
okq search "malformed"          # ignored files never match
okq --bundle docs orphans       # ignored files are not orphans
```

A `.okqignore` at the bundle root, e.g.:

```gitignore
# fixtures exist to test graceful failure, not as real concepts
tests/

# keep one canonical example even though we ignore drafts
drafts/
!drafts/example.md
```

### Semantics

- **Ignored = not in the bundle.** An ignored file is treated as if it were
  absent: it is not a concept, not a search hit, not an orphan, not a stats
  count. `okq get <ignored-id>` returns **not found** (exit 4). A link pointing
  at an ignored concept becomes a **dead link** (it now points at nothing),
  surfaced by `deadlinks` like any other broken link.
- **Patterns match concept paths relative to the directory of the `.okqignore`
  file that contains them**, with gitignore precedence across nested files. A
  matched directory excludes everything beneath it.
- **`--no-ignore`** turns the feature off entirely for that invocation: all
  `.okqignore` files are ignored and the full tree loads. Useful for auditing
  what your rules hide, and how okq's own tests still reach `docs/tests/`.

### Output

No new output shape. Every command behaves exactly as before, just over the
reduced concept set. The `--json` contracts are unchanged. Human-facing notes
about exclusion (if any are added later, e.g. an "ignored N files" line) go to
**stderr**, never stdout, preserving the one-JSON-document rule.

### Exit codes

Unchanged taxonomy. The only visible interaction: `get`/section lookups on an
ignored id resolve to **4** (concept not found), because the concept genuinely
is not in the bundle.

## Acceptance criteria

- [x] A `.okqignore` at the bundle root excludes matching files from `stats`,
      `orphans`, `deadlinks`, `find`, `search`, `get`, and graph commands.
- [x] Full gitignore syntax works: comments, `!` negation, `/` anchoring, `**`,
      and `dir/` directory excludes.
- [x] **Nested** `.okqignore` files are honored with correct precedence (deepest
      file wins; last matching pattern within a file wins).
- [x] `--no-ignore` disables all `.okqignore` processing for that invocation.
- [x] `get` on an ignored id exits 4; a link into an ignored concept appears in
      `deadlinks`.
- [x] The Tantivy index reflects the filtered set, does not bleed across
      `--no-ignore`, and rebuilds when a `.okqignore` file changes (ADR-0003).
- [x] Malformed/edge-case `.okqignore` (unreadable, empty, only comments) degrade
      gracefully — never panic; an unreadable file is reported on stderr and
      skipped.
- [x] `--json` output for every command is byte-identical to pre-feature output
      when no `.okqignore` exists.

## Open questions

- **Transparency.** Should `stats` (or a `--verbose`) report the ignored-file
  count, so a typo'd pattern that hides real concepts is noticeable? Leaning yes
  for `stats`, deferred until the core lands.
- **`init` scaffolding.** Should `okq init` drop a commented starter `.okqignore`?
  Cheap, but separate from this feature.
- **A `--no-ignore` short form / env var** (`OKQ_NO_IGNORE`)? Only if demand
  appears; promote to [PLAN.md](../../PLAN.md) §8 if so.

## Related

- [ADR-0006 — .okqignore filtering](../adrs/0006-okqignore-filtering.md) — the
  decision and matching semantics.
- [stats](stats.md) — the command most distorted without ignore.
- [search](search.md) — the index that must stay consistent (ADR-0003).
- [graph](graph.md) — orphans/deadlinks/neighbors over the filtered set.
