---
type: feature
title: okq validate (alias doctor)
status: active # draft | accepted | active | deprecated
created: 2026-06-27
updated: 2026-06-27
tags: [cli, health, conformance, validate, ci]
milestone: null # milestones retired — see CHANGELOG.md
command: okq validate
related:
  - ./stats.md
  - ./graph.md
  - ./find.md
  - ./scaffold.md
  - ../adrs/0004-exit-code-taxonomy.md
  - ../guides/design-overview.md
---

# okq validate (alias doctor)

## Summary

`okq validate` (aliased `okq doctor`) reports the documents okq can't fully use —
unparseable frontmatter, missing required/recommended fields, malformed reserved
files, and unresolved links — each with its file and reason and a severity, so a
silently-dropped doc becomes a clear, fixable message.

## Motivation

okq loads permissively: a doc with broken frontmatter goes to `parse_errors` and
is **silently skipped**, so it just never shows up in `search`/`find`/`get` and
the author is left grepping to find out why. [`stats`](./stats.md) shows only a
*count* (`Parse errors: 3`), not *which* files or *why*. This is the failure mode
most likely to bite when a real bundle is put through okq, and it's invisible
today. `validate` surfaces it — and resolves the open question already recorded in
[find.md](./find.md) ("malformed docs are skipped; whether/how to surface them is
an open question").

It's also cheap and honest to build: the upstream `okf` crate already ships
`validate_bundle()`, which returns a `Report` of severity-tagged diagnostics
(unparseable docs and missing `type` as errors; missing recommended fields, bad
`timestamp`, malformed `index.md`/`log.md` as warnings; unresolved links as info).
`okq validate` wraps that in okq's envelope, `--json`, and exit-code contract.

## Scope

### In scope

- Run okf's bundle validation and present every diagnostic with **severity**
  (error / warning / info), **path**, and **message**.
- Conformance errors: unparseable frontmatter, missing required `type`.
- Warnings: missing recommended fields (`title`, `description`, `timestamp`),
  non-ISO-8601 `timestamp`, reserved-file structure issues (`index.md` carrying
  frontmatter, root `index.md` declaring more than `okf_version`, bad `log.md`
  dates).
- Severity filtering and a `--check` mode for CI.
- The `doctor` alias as a friendlier name for the same command.

### Out of scope

- *Fixing* the problems — `validate` reports; the author (or a future
  `--fix`/`okq new` round-trip) edits. A `--fix` is an open question, not v1.
- Re-implementing validation rules — okf owns the rule set; okq presents it. New
  okq-specific rules, if any, are a later decision.
- Replacing [`deadlinks`](./graph.md): `validate` includes unresolved links as
  *info*, but `deadlinks` stays the focused, graph-aware link checker. `validate`
  is the broad conformance sweep; `deadlinks`/`orphans` are targeted health views.

### Relationship to existing commands

- `stats` keeps its one-line `parse_errors` **count** as an at-a-glance signal;
  `validate` is the **detailed** drill-down behind it.
- `deadlinks`/`orphans` remain single-purpose; `validate` is the umbrella health
  check (a natural `doctor`).

## Behavior

### Invocation & flags

```sh
okq validate                    # full conformance report
okq doctor                      # alias — identical behavior
okq validate --check            # exit 3 if any error-severity issue (CI gate)
okq validate --severity warning # show warnings and errors (filter the floor)
okq --bundle docs validate --json
```

- `--check` (CI/pre-commit): exit non-zero when the bundle is **non-conformant**
  (any `error` diagnostic). A `--strict` could extend the gate to warnings — see
  open questions.
- `--severity <error|warning|info>` sets the minimum severity shown (default:
  show all, or at least warning+error — see open questions).
- Honors `--bundle`, `--no-ignore`, `--json`. Ignored files are not validated
  unless `--no-ignore`.

### Output

- Human: diagnostics ordered by severity (errors first, then path/message), each
  as `severity  path  message` on stdout; a summary line (`conformant: N
  error(s), M warning(s), K info(s)`) goes to **stderr**.
- `--json`: one `okq.validate/v1` document on stdout — `conformant` (bool),
  per-severity counts (`errors`/`warnings`/`infos`, computed over the
  ignore-filtered bundle), and a `diagnostics` array of `{severity, path,
  message}` (the shown, severity-floored subset). okf does not attach line
  numbers, so diagnostics are file-level. This is the agent/CI contract.

### Exit codes

Shared taxonomy ([ADR-0004](../adrs/0004-exit-code-taxonomy.md)):

- `0` — ran successfully (even if warnings/infos exist; reporting issues is not an
  error), **and** without `--check`, or `--check` with zero errors.
- `1` — could not run (bad bundle / I/O).
- `2` — usage error (e.g. bad `--severity` value).
- `3` — `--check` and the bundle is non-conformant (≥1 error).

## Acceptance criteria

- [ ] `okq validate` lists each conformance issue with severity, path, and reason,
      sourced from `okf::validate_bundle`.
- [ ] `okq doctor` is accepted as an alias and behaves identically.
- [ ] A doc with broken frontmatter (silently dropped from queries today) appears
      as an **error** in `validate`.
- [ ] `--check` exits 3 on a non-conformant bundle, 0 on a conformant one, writing
      nothing.
- [ ] `--severity` filters the minimum level shown.
- [ ] `--json` emits the documented `okq.validate/v1` envelope; a clean bundle
      reports `conformant: true` with empty diagnostics.
- [ ] Respects `.okqignore` (and `--no-ignore`).
- [ ] Degrades gracefully on a wildly malformed bundle — reports, never panics.

## Open questions

- **Default severity floor:** show info-level (unresolved links) by default, or
  warning-and-up to keep output focused (with `--severity info` to opt in)? Lean:
  warning-and-up by default, since `deadlinks` already covers links.
- **`--check` strictness:** gate on errors only (default), with `--strict` to also
  fail on warnings? Lean: yes.
- **`--fix`:** a future opt-in to auto-add missing recommended fields / stamp
  dates via the templates — deferred; would pair with `okq new`'s template set.
- **okq-specific rules:** beyond okf's rule set, does okq want its own (e.g.
  flag this repo's `status`/`related` extension conventions)? Defer until a real
  need appears.

## Related

- [stats.md](./stats.md) — the `parse_errors` count `validate` drills into
- [graph.md](./graph.md) — `deadlinks`/`orphans`, the focused health views `validate` complements
- [find.md](./find.md) — records the "surface skipped malformed docs" open question this resolves
- [scaffold.md](./scaffold.md) — `init`/`new`; a future `validate --fix` would round-trip with these
- [ADR-0004](../adrs/0004-exit-code-taxonomy.md) — the exit-code contract (`--check` → 3)
