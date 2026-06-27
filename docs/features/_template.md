---
type: feature
title: Feature name (often a command, e.g. "okq search")
status: draft # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: []
milestone: null # optional, historical; milestones are retired — releases live in CHANGELOG.md
command: null # the CLI command this spec defines, e.g. "okq search", or null
related: [] # paths to related docs
---

# Feature name

## Summary

One or two sentences: what this command/capability lets the user (human or agent) do.

## Motivation

Why this exists — the problem it removes. Tie it back to a real scenario (the context-assembly wall, hand-rolled `rg`/`yq` pipelines, an agent losing the middle of a long index).

## Scope

### In scope

- What this feature covers.

### Out of scope

- What it deliberately does not cover (and where that lives instead, if known).

## Behavior

How it works from the caller's perspective:

- **Invocation & flags** — the command line, including the non-interactive/`--json` path (every command is agent-runnable).
- **Output** — human table vs. `--json` shape. Honor token-frugality: default to ranked `path:line` + frontmatter + a short snippet, never full bodies ([design overview](../guides/design-overview.md) §principles).
- **Exit codes** — what success/empty/error map to, so scripts and CI can branch on `$?`.

## Acceptance criteria

- [ ] Concrete, checkable statements that mean "done".
- [ ] Has a fully non-interactive path with `--json`.
- [ ] Output is locations-first, not content dumps.

## Open questions

- Anything unresolved, with enough context that someone else could pick it up. Keep them here in the spec they belong to; promote a durable, cross-cutting one to a new ADR or the [design overview](../guides/design-overview.md).

## Related

- Links to relevant ADRs, workflows, or other features: `../adrs/0001-….md`
