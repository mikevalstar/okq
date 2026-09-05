---
type: feature
title: okq lint — opinionated bundle hygiene beyond conformance
status: active # draft | accepted | active | deprecated
created: 2026-08-02
updated: 2026-09-05
tags: [cli, lint, health, hygiene, ci, okf, validate]
milestone: null
command: okq lint
related:
  - "../adrs/0016-okf-workspace-split.md"
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

`okq lint` reports the authoring and formatting problems a continuously-authored
corpus drifts into — a body with no heading, a section left empty, a declared
source nobody cited, an orphan nothing links to — each tagged with a rule code so
CI can pin or silence individual checks.

## Motivation

[`okq validate`](validate.md) answers a narrow question: *does this bundle
conform to OKF §11?* It is deliberately narrow, because conformance is the
spec's call and not okq's. That leaves a whole class of real problems unreported,
because none of them makes a bundle non-conformant:

- a concept nobody links to and no index lists
- a body that never opens with a heading, or whose heading levels skip a rung
- a section heading with nothing under it — a stub someone meant to come back to
- a source declared in frontmatter that the prose never actually cites
- a verifier written in some ad-hoc form instead of `human:<id>`
- trailing whitespace and stacked blank lines that make every diff noisier

These are exactly what goes wrong in a docs tree that agents and people both
write to. okf ships them as `lint_bundle()`, returning the same `Report` type
`validate` already wraps — so okq gets them for the cost of a presentation layer,
and the corpus gets a health gate that is honest about being an opinion rather
than a standard.

Where the line falls between the two commands is **okf's call, and it has
moved**. okf 0.2.7 pushed the frontmatter and link checks into `validate` as
`V`-coded warnings and left lint the authoring-hygiene half, renumbering the
codes that survived. okq follows that split rather than preserving its own
([ADR-0016](../adrs/0016-okf-workspace-split.md)); the rule set is not okq's to
keep stable.

## Scope

### In scope

- Run okf's `lint_bundle()` and present every finding with **severity**,
  **rule code**, **path**, and **message**.
- Filter by rule (`--rule`) or suppress rules (`--ignore`), so a team can adopt
  the checks it agrees with without forking.
- Severity filtering and `--check` for CI, matching `validate`.
- `.okqignore` awareness, matching every other command.

### Out of scope

- **Conformance.** Lint never emits an error severity and never affects
  `okq validate`'s `conformant` verdict
  ([ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md)). A bundle with lint
  findings is still conformant.
- **Fixing anything.** `lint` reports; it does not rewrite. `okq index` already
  regenerates the listings [`validate`](validate.md)'s V35 complains about. okf
  0.2.7 marks each finding `fixable` where `okf fix` could repair it; okq does
  not surface that yet.
- **Staleness.** It moved to [`validate --today`](validate.md) (V12) when okf
  0.2.7 rebalanced the two commands.
- **okq-specific rules.** The rule set is okf's. If okq wants a rule okf lacks,
  that is a change to okf, not a local addition — the data layer owns document
  semantics ([ADR-0002](../adrs/0002-library-stack.md)).

## Behavior

### Rules

| Code | Severity | Finding |
|------|----------|---------|
| L1 | warning | body has no top-level `#` heading |
| L2 | info | frontmatter keys not in canonical order |
| L3 | warning | heading hierarchy drift (levels skipped, or multiple `#`) |
| L4 | warning | a heading with no content under it |
| L5 | warning | a `sources` entry declared but never cited with a footnote |
| L6 | info | non-standard actor identity in `generated`, `verified`, or `sources.author` |
| L7 | warning | `# Computation` code block with no language tag |
| L8 | info | trailing whitespace or excess blank lines in the body |
| L9 | warning | orphan: no inbound links **and** not listed in any `index.md` |
| L10 | info | self-link |
| L11 | info | no `verified` events; trust tier is `unverified` |
| L12 | info | `status: draft` |
| L13 | warning | `okf_version` in the root `index.md` is unquoted |

The codes are okf's and okf renumbered them at 0.2.7. Where the rules okq used
to document went:

| Was | Now | Finding |
|-----|-----|---------|
| L1, L2, L3 | `validate` V3 | missing `title` / `description` / `generated` |
| L4 | lint L11 | no `verified` events |
| L5, L6 | `validate` V19, V20 | legacy v0.1 `timestamp` / `# Citations` |
| L7 | `validate` V4 | body is empty |
| L8 | lint L1 | body has no top-level `#` heading |
| L9 | `validate` V8 | latest `verified.at` predates `generated.at` |
| L10 | `validate` V29 | links to a `status: deprecated` concept |
| L11 | `validate` V12 | past `stale_after` |
| L13 | lint L10 | self-link |
| L14 | `validate` V30 | `title` shared with another concept |
| L15 | lint L9 | orphan |
| L16 | `validate` V35 | `index.md` out of sync with its directory |

There is no alias table: `--rule L15` is a usage error, not a silent remap onto
L9. A pinned CI code that changed meaning is more dangerous than one that fails
loudly ([ADR-0016](../adrs/0016-okf-workspace-split.md)).

### Which health command to reach for

| Question | Command |
|----------|---------|
| Does this conform to OKF? | [`validate`](validate.md) |
| Is this corpus drifting? | `lint` |
| What links are broken? | [`deadlinks`](graph.md) |
| What has no inbound links at all? | [`orphans`](graph.md) |

`lint`'s L9 is deliberately **narrower** than [`orphans`](graph.md): it excludes
concepts an `index.md` lists, so a leaf you deliberately indexed is not nagged
about. `orphans` stays the pure link-graph view. Reach for `orphans` when you
want the graph fact, `lint` when you want the judgment.

### Invocation & flags

```sh
okq lint                                   # every finding, warnings and info
okq lint --check                           # exit 3 if anything is found (CI)
okq lint --severity warning                # drop the info-level noise
okq lint --rule L9                         # only the orphan rule
okq lint --ignore L11 --ignore L12         # everything except unverified/draft
okq lint --json                            # for CI or an agent
```

`--rule` and `--ignore` are repeatable and take codes case-insensitively
(`l9` works). They are mutually exclusive: passing both is a usage error (exit
2) rather than a silently-resolved precedence puzzle. An unknown code is also a
usage error, listing the known codes — a typo'd `--ignore L99` must not silently
lint everything.

### Output

Human output is one finding per line, matching `validate`'s shape with the rule
code between severity and path:

```
warning  L9   guides/orphan-note.md  orphan concept: no other concept links to it and no index.md lists it
warning  L4   adrs/0002-library-stack.md  heading `Consequences` has no content
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
      "rule": "L9",
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
- [ ] Codes okf retired (L14–L16) are a usage error, never silently remapped.
- [ ] Lint never reports an error severity, and never changes what
      `okq validate` calls conformant.
- [ ] Findings for `.okqignore`d files are dropped, like `validate`.
- [ ] `--json` emits exactly one `okq.lint/v1` document on stdout; `okq schema
      lint` returns its schema.
- [ ] A message that does not carry a recognizable `[Lnn] ` prefix yields
      `rule: null` and an unmodified message, without a panic.
- [ ] A malformed bundle lints without panicking.

## Open questions

- **Per-rule severity overrides** (`--error L9` to make one rule fail a gate)
  would let a team escalate a specific check. Deferred: `--check` plus `--rule`
  already expresses "fail on exactly these", which covers the same ground with
  fewer flags.
- **A config file for the rule set.** Repeating `--ignore L11 --ignore L12` in
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
- [ADR-0016](../adrs/0016-okf-workspace-split.md) — the okf 0.2.7 rebalance that
  renumbered these rules and moved most of them into `validate`
- [trust.md](trust.md) — the trust values L11 reports on
- [index-command.md](index-command.md) — `okq index`, which fixes what
  `validate`'s V35 finds
