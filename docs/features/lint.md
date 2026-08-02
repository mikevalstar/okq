---
type: feature
title: okq lint — opinionated bundle hygiene beyond conformance
status: active # draft | accepted | active | deprecated
created: 2026-08-02
updated: 2026-08-02
tags: [cli, lint, health, hygiene, ci, okf, validate]
milestone: null
command: okq lint
related:
  - "../adrs/0014-surfacing-okf-v02-semantics.md"
  - "../adrs/0013-back-to-upstream-okf.md"
  - "../adrs/0004-exit-code-taxonomy.md"
  - "validate.md"
  - "graph.md"
  - "trust.md"
  - "index-command.md"
---

# okq lint — opinionated bundle hygiene beyond conformance

## Summary

`okq lint` reports the hygiene problems a continuously-authored corpus drifts
into — an untitled concept, a body with no heading, a link to something
deprecated, a verification that predates the last regeneration, an `index.md`
that has fallen behind its directory — each tagged with a stable rule code so CI
can pin or silence individual checks.

## Motivation

[`okq validate`](validate.md) answers a narrow question: *does this bundle
conform to OKF §11?* It is deliberately narrow, because conformance is the
spec's call and not okq's. That leaves a whole class of real problems unreported,
because none of them makes a bundle non-conformant:

- a concept nobody links to and no index lists
- a doc whose `verified` date is older than its `generated` date — it was
  regenerated after the last human looked at it
- a link into a `status: deprecated` concept
- an `index.md` that no longer matches its directory
- two concepts with the same title, so a search hit is ambiguous to a human
- legacy v0.1 `timestamp` and `# Citations` blocks left behind by the v0.2 move

These are exactly what goes wrong in a docs tree that agents and people both
write to. okf 0.2 ships all sixteen checks as `lint_bundle_at()`, returning the
same `Report` type `validate` already wraps — so okq gets them for the cost of a
presentation layer, and the corpus gets a health gate that is honest about being
an opinion rather than a standard.

## Scope

### In scope

- Run okf's `lint_bundle_at()` and present every finding with **severity**,
  **rule code**, **path**, and **message**.
- Filter by rule (`--rule`) or suppress rules (`--ignore`), so a team can adopt
  the checks it agrees with without forking.
- `--today` for the staleness rule (L11), so a lint run is reproducible.
- Severity filtering and `--check` for CI, matching `validate`.
- `.okqignore` awareness, matching every other command.

### Out of scope

- **Conformance.** Lint never emits an error severity and never affects
  `okq validate`'s `conformant` verdict
  ([ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md)). A bundle with lint
  findings is still conformant.
- **Fixing anything.** `lint` reports; it does not rewrite. `okq index` already
  regenerates the listings L16 complains about.
- **okq-specific rules.** The rule set is okf's. If okq wants a rule okf lacks,
  that is a change to okf, not a local addition — the data layer owns document
  semantics ([ADR-0002](../adrs/0002-library-stack.md)).

## Behavior

### Rules

| Code | Severity | Finding |
|------|----------|---------|
| L1 | warning | missing `title` |
| L2 | warning | missing `description` |
| L3 | warning | missing `generated` (and no legacy `timestamp`) |
| L4 | info | no `verified` events; trust tier is `unverified` |
| L5 | warning | legacy v0.1 `timestamp` present |
| L6 | warning | legacy v0.1 body `# Citations` list present |
| L7 | warning | body is empty |
| L8 | warning | body has no top-level `#` heading |
| L9 | warning | latest `verified.at` predates `generated.at` |
| L10 | warning | links to a `status: deprecated` concept |
| L11 | warning | past `stale_after` (needs `--today` or the system date) |
| L12 | info | `status: draft` |
| L13 | info | self-link |
| L14 | warning | `title` shared with another concept |
| L15 | warning | orphan: no inbound links **and** not listed in any `index.md` |
| L16 | warning | an existing `index.md` is out of sync with its directory |

### Which health command to reach for

| Question | Command |
|----------|---------|
| Does this conform to OKF? | [`validate`](validate.md) |
| Is this corpus drifting? | `lint` |
| What links are broken? | [`deadlinks`](graph.md) |
| What has no inbound links at all? | [`orphans`](graph.md) |

`lint`'s L15 is deliberately **narrower** than [`orphans`](graph.md): it excludes
concepts an `index.md` lists, so a leaf you deliberately indexed is not nagged
about. `orphans` stays the pure link-graph view. Reach for `orphans` when you
want the graph fact, `lint` when you want the judgment.

### Invocation & flags

```sh
okq lint                                   # every finding, warnings and info
okq lint --check                           # exit 3 if anything is found (CI)
okq lint --severity warning                # drop the info-level noise
okq lint --rule L15 --rule L16             # only the two index/orphan rules
okq lint --ignore L4 --ignore L12          # everything except unverified/draft
okq lint --today 2026-08-02 --json         # reproducible staleness, as JSON
```

`--rule` and `--ignore` are repeatable and take codes case-insensitively
(`l15` works). They are mutually exclusive: passing both is a usage error (exit
2) rather than a silently-resolved precedence puzzle. An unknown code is also a
usage error, listing the known codes — a typo'd `--ignore L99` must not silently
lint everything.

### Output

Human output is one finding per line, matching `validate`'s shape with the rule
code between severity and path:

```
warning  L15  guides/orphan-note.md  no inbound links and not listed in any index.md
warning  L9   adrs/0002-library-stack.md  latest `verified.at` predates `generated.at`
   info  L12  features/draft-thing.md  `status: draft`
```

`--json` emits one `okq.lint/v1` document:

```json
{
  "schema": "okq.lint/v1",
  "findings": 3,
  "warnings": 2,
  "infos": 1,
  "diagnostics": [
    {
      "severity": "warning",
      "rule": "L15",
      "path": "guides/orphan-note.md",
      "message": "no inbound links and not listed in any index.md"
    }
  ]
}
```

The rule code is lifted out of okf's message text into its own field, and the
message is reported with the `[Lnn] ` prefix stripped. If okf ever changes that
prefix format, `rule` degrades to `null` and the message passes through
unchanged ([ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md)).

Ordering is deterministic: severity descending, then rule, then path, then
message.

### Exit codes

| Code | When |
|------|------|
| 0 | ran successfully, findings or not (findings are not an error) |
| 3 | `--check` and at least one finding survived the filters |
| 2 | usage: unknown rule code, or `--rule` together with `--ignore` |
| 1 | bad bundle or I/O |

`--check` respects `--severity`, `--rule`, and `--ignore`, so a team can gate CI
on the subset it has agreed to enforce.

## Acceptance criteria

- [ ] Every finding carries severity, rule code, bundle-relative path, and a
      message with the `[Lnn] ` prefix stripped.
- [ ] `--rule` and `--ignore` filter by code, case-insensitively; passing both,
      or an unknown code, is a usage error (exit 2).
- [ ] `--check` exits 3 when findings survive the filters, 0 when none do.
- [ ] `--today` makes L11 reproducible.
- [ ] Lint never reports an error severity, and never changes what
      `okq validate` calls conformant.
- [ ] Findings for `.okqignore`d files are dropped, like `validate`.
- [ ] `--json` emits exactly one `okq.lint/v1` document on stdout; `okq schema
      lint` returns its schema.
- [ ] A message that does not carry a recognizable `[Lnn] ` prefix yields
      `rule: null` and an unmodified message, without a panic.
- [ ] A malformed bundle lints without panicking.

## Open questions

- **Per-rule severity overrides** (`--error L15` to make one rule fail a gate)
  would let a team escalate a specific check. Deferred: `--check` plus `--rule`
  already expresses "fail on exactly these", which covers the same ground with
  fewer flags.
- **A config file for the rule set.** Repeating `--ignore L4 --ignore L12` in
  every CI invocation will get old. Worth revisiting if okq ever grows a config
  file for anything else — inventing one for lint alone is not worth it.

## Related

- [ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md) — why lint is a
  sibling of `validate` and how the rule code is surfaced
- [ADR-0013](../adrs/0013-back-to-upstream-okf.md) — the okf 0.2 move that
  brought the lint module
- [ADR-0004](../adrs/0004-exit-code-taxonomy.md) — the shared exit codes
  `--check` uses
- [validate.md](validate.md) — the conformance command lint sits beside
- [graph.md](graph.md) — `orphans` and `deadlinks`, the focused link views
- [trust.md](trust.md) — the trust values L4, L9, L11, and L12 report on
- [index-command.md](index-command.md) — `okq index`, which fixes what L16 finds
