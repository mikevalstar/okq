---
type: feature
title: okq find — filter concepts by predicate
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, find, query, frontmatter, json, filter]
milestone: M1
command: "okq find"
related: ["get.md", "../adrs/0002-library-stack.md", "../../PLAN.md"]
---

# okq find — filter concepts by predicate

## Summary

`okq find` returns every concept in a bundle that satisfies a set of **frontmatter and content predicates** — `--tag`, `--type`, `--where field=value`, `--match <pattern>`. It answers *"which concepts match X?"* as a locations-only shortlist (the shared concept envelope), **by set membership, not by relevance ranking**. It is the deterministic, exact-predicate counterpart to `search` (which ranks) and the discovery counterpart to `get` (which expands one known concept).

## Motivation

Asking "which decisions are security-related?" or "what's `status: accepted` and tagged `auth`?" today means hand-rolling `fd | xargs yq | rg` pipelines (with mise-shim PATH and quoting pain, PLAN.md §2) — not repeatable, not shareable, not agent-callable. `find` turns those into one deterministic command with `--json`, so a person gets an instant answer and an agent gets a clean tool-call that returns *just the matching nodes' locations* — the right input to a `get`/`neighbors` expansion, without dumping any bodies.

`find` is the **second M1 command**: it reuses the concept envelope `get` ratified and establishes the **collection envelope** that `search` and the graph list-commands will all reuse.

## Scope

### In scope

- Predicates over a loaded bundle: `--tag`, `--type`, `--where field=value`, `--match <pattern>` (+`--regex`).
- Boolean combination of predicates (see Behavior) and a deterministic, locations-only result list.
- Human and `--json` output, the latter a stable `okq.find/v1` collection envelope.

### Out of scope

- **Ranking / relevance** — that's `search`. `find` returns matches in a fixed order, unscored.
- **Graph traversal** — `neighbors`/`backlinks`/`path`.
- **Content expansion** — `find` emits locations + frontmatter, never bodies; the caller expands a chosen hit with `get`.
- **Per-match line numbers / snippets** — `find` is concept-level membership (a concept either matches or not); per-section locations + snippets are `search`'s job (v1; see Open questions).
- **Mutation.**

## Behavior

### Predicates and how they combine

| Flag | Repeatable | Matches a concept when… |
|------|------------|--------------------------|
| `--tag <t>` | yes | its frontmatter `tags` contains `<t>` |
| `--type <ty>` | yes | its frontmatter `type` equals `<ty>` |
| `--where <field>=<value>` | yes | frontmatter `<field>` equals `<value>` (scalar), or contains it (sequence) |
| `--match <pattern>` | no (v1) | its title or body contains `<pattern>` |

- **Across different flags: AND.** `--type adr --tag security` ⇒ ADRs that are also tagged security.
- **Repeated `--tag`: AND** (must have *all* listed tags). Repeated `--where`: AND.
- **Repeated `--type`: OR** among the values (a concept has one type; match if it's any listed) — ANDed against the other predicate kinds.
- **No predicates** ⇒ every concept (a plain listing). Useful with `--json` as a bundle inventory.

### Value semantics

- `--where field=value` compares against the frontmatter value's display string for scalars; for a **sequence** field, it matches if `value` is a member (so `--where tags=pii` ≡ `--tag pii`). Only `=` (equality/membership) ships in v1; other operators are an open question.
- `--match` is **case-insensitive substring** by default over **title + body**; `--regex` treats the pattern as a regular expression (via the `regex` crate, ADR-0002). An invalid regex is a usage error (exit 2), not a crash.

### Output

Locations-only, token-frugal (PLAN.md §3) — never bodies.

**Human:** one concept per line, `path:line` first for clickability, then type and title; e.g.

```
adrs/0006-agent-runnable-commands.md:1   adr   ADR-0006 — Every command is agent-runnable
```

Order is **deterministic**: concept-id (path) order, since nothing is ranked. Color honors `--no-color`/`NO_COLOR`/non-TTY.

**`--json`:** one document — the **collection envelope** (the shape `search`/graph lists reuse):

```json
{
  "schema": "okq.find/v1",
  "count": 1,
  "results": [
    { "id": "adrs/0006-agent-runnable-commands", "type": "adr",
      "title": "ADR-0006 — Every command is agent-runnable",
      "path": "adrs/0006-agent-runnable-commands.md", "line": 1,
      "tags": ["cli", "automation"] }
  ]
}
```

Each `results` element is the **shared concept envelope** ratified by [get](get.md) (`id`/`type`/`title`/`path`/`line`), plus `tags`. `line` is the concept start (`1`) — `find` reports concepts, not match sites.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Ran successfully — **including zero matches** (an empty result is not an error; `--json` gives `count: 0`) |
| 2 | Usage error: bad flags, malformed `--where` (no `=`), or an invalid `--regex` pattern |
| 1 | Bundle could not be loaded (bad `--bundle`, I/O) |

`find` is a query, so empty ≠ error (unlike the health commands `orphans`/`deadlinks`, which PLAN.md §7 makes non-zero for CI). A `--exit-nonzero-on-empty` opt-in for CI use is an open question.

### Bundle edge cases

- Reserved `index.md`/`log.md` are not concepts (okf excludes them at load), so they never appear in results.
- Files okf could not parse (`Bundle::parse_errors`) are skipped; whether/how to surface them (a stderr note, a `--strict`) is an open question.

## Cross-cutting contracts this feature ratifies

- **The collection envelope** `okq.find/v1` = `{ schema, count, results: [<concept envelope>] }` — the list shape `search` and the graph list-commands reuse (so every "many results" command looks the same to an agent).
- **Deterministic ordering** for unranked results: concept-id order.
- It also provides the **concept-enumeration + name-matching substrate** that [get](get.md)'s planned partial-path resolution builds on (hence get's partial mode is sequenced after `find`).

## Acceptance criteria

- [ ] `--tag`, `--type`, `--where`, `--match` each filter correctly in isolation.
- [ ] Cross-flag AND, repeated-`--tag` AND, repeated-`--type` OR all behave as specified.
- [ ] `--where field=value` handles scalar equality and sequence membership.
- [ ] `--match` is case-insensitive substring by default; `--regex` enables regex; invalid regex → exit 2.
- [ ] `--json` emits the `okq.find/v1` collection envelope with accurate `count`, results in concept-id order, reusing the get concept envelope.
- [ ] Zero matches → exit 0 (empty list / `count: 0`); bad predicate → exit 2; bad bundle → exit 1.
- [ ] No bodies are ever emitted (token-frugal); locations + frontmatter only.
- [ ] Reserved and parse-error files never appear in results.
- [ ] Works on this repo's own `docs/` tree (format-tolerant); fully non-interactive; output snapshot-tested (`insta`).

## Open questions

- **Multi-`--tag` semantics** — AND (chosen) vs. OR; do we add `--any`/`--all` to switch, or `--tag a,b` as OR-within-field?
- **`--where` operators** — beyond `=`: `!=`, `~regex`, presence/absence (`field=`), numeric/date comparisons? Keep `=` only for v1, design the parse so operators are additive.
- **`--match` scope** — title + body only, or also headings and frontmatter values? And should `--match` optionally report the matched line numbers (blurring toward `search`)?
- **Empty-result exit code** — add `--exit-nonzero-on-empty` for CI predicate-gating, or leave that to the health commands?
- **Parse-error visibility** — silently skip unparseable files, warn on stderr, or fail under a `--strict` flag?
- **Frontmatter `id` / partial input** — once get's partial resolution lands, should `find` share the same matcher (and could `find` gain a positional name filter)?

## Related

- [get](get.md) — the concept envelope reused here; its planned partial-path resolution builds on `find`'s matcher
- [ADR-0002](../adrs/0002-library-stack.md) — `okf` load + frontmatter accessors, the `regex` crate for `--match`, `schemars`/`serde_json` for the envelope
- [PLAN.md](../../PLAN.md) — §3 token-frugal output, §5 `find` vs `search` (filter vs rank), §7 M1, §8 schema-versioning
- Future: `search` — the ranked counterpart; reuses the collection envelope ratified here
