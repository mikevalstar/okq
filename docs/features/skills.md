---
type: feature
title: Agent skills (okq-* suite)
status: active # draft | accepted | active | deprecated
created: 2026-06-27
updated: 2026-06-27
tags: [skills, agents, distribution]
milestone: M4.5 # roadmap milestone from PLAN.md §7
command: null # skills are packaged assets, not a CLI command
related:
  - ./skills-install.md
  - ../adrs/0007-opt-in-network-for-skill-install.md
  - ../adrs/0005-dogfood-okq-for-docs.md
  - ../adrs/0001-documentation-first-okf-shaped.md
  - ../adrs/0004-exit-code-taxonomy.md
  - ./search.md
  - ./find.md
  - ./get.md
  - ./graph.md
---

# Agent skills (okq-* suite)

## Summary

A small suite of [Agent Skills](https://agentskills.io) — folders with a
`SKILL.md` — that teach an AI agent to use `okq` and the OKF format well: how to
explore a bundle before working, how to author a conformant document, and how to
keep a bundle healthy. Shipping the skills alongside the binary means adopting
`okq` also onboards the agents that use it.

## Motivation

`okq` already gives agents an agent-runnable contract (`--json`, stable exit
codes, `path:line` output). But a contract isn't usage: an agent that doesn't
know `okq` exists will still reach for `grep`/`rg`, dump whole files into its
context, and write OKF documents that don't parse or cross-link. Skills close
that gap — procedural knowledge that loads only when relevant, so it costs
almost nothing until an agent is actually working in a bundle.

This expands PLAN.md §7's M4.5 (originally "one skill to navigate, one to explain
the format") into four task-shaped skills, split by *when an agent needs them*
rather than by command.

## Scope

### In scope

- Four skills, all prefixed `okq-`:
  - **`okq-explore`** — find/search/navigate a bundle to prepare for work.
  - **`okq-write-okf`** — author a new or edited OKF document.
  - **`okq-maintain`** — keep a bundle healthy (links, orphans, status).
  - **`okq-reference`** — the `okq` CLI contract as background knowledge the
    other three lean on.
- Each is a directory with a `SKILL.md` (YAML frontmatter + markdown body),
  following the open Agent Skills standard so it works in Claude Code and other
  compatible agents.
- Development home: `skills/<name>/` in this repo (distributable and
  dogfoodable). A copy/symlink into `~/.claude/skills/` is the author's local use.
- Distribution via [skills.sh](https://www.skills.sh/): the registry indexes
  public GitHub repos containing `SKILL.md` files; there is no upload step, so a
  public `skills/` directory is automatically discoverable
  (`npx skills add mikevalstar/okq`).

### Out of scope

- Changing `okq` itself. Skills are documentation/configuration; if a skill wants
  a capability `okq` lacks, that gap becomes a separate feature spec (the
  fix-the-tool-don't-route-around-it rule from ADR-0005).
- An MCP server (`okq mcp`) — that's the separate "Later — Agent ergonomics"
  milestone, not a skill.
- A bundle-bootstrap skill (`okq-init-bundle`). `okq init`/`new` already cover
  scaffolding from the CLI; revisit a dedicated skill only if onboarding needs it.

## Behavior

How an agent encounters each skill. The `description` frontmatter is what an
agent reads to decide whether to load a skill, so it must state *what it does and
when to use it*, key use case first.

- **`okq-explore`** (read-only; the most-loaded skill). Triggered when the user
  asks to find, search, understand, or "look into" docs/specs/ADRs, or mentions
  okq/OKF. Teaches the assemble-context loop: `search` → `find` → `get --section`
  → `neighbors`/`backlinks`/`path`, plus `stats` for orientation. Reinforces
  token-frugality: read `path:line` + snippet, expand a section with `get`, never
  slurp whole files.

- **`okq-write-okf`** (authoring). Triggered when creating or editing an OKF
  document — ADR, feature spec, runbook. Covers frontmatter schema, section
  structure, `[[…]]` cross-linking, the draft → accepted → active status
  lifecycle, and verifying the result with `okq schema` / `okq find` / `okq
  deadlinks`. ADR vs feature-spec are two modes of this one skill, not separate
  skills.

- **`okq-maintain`** (upkeep). Triggered when checking or fixing bundle health:
  `deadlinks` after a rename, `orphans` for stale docs, status hygiene, and
  re-linking. Pairs with the `--check` CI gating from M3.

- **`okq-reference`** (background knowledge; `user-invocable: false`). Not invoked
  directly — auto-loaded when okq/OKF is in play to supply the CLI contract in one
  place: the command list, the `--json` discipline, the exit-code taxonomy
  (ADR-0004), and `schema`. Lets the other three stay lean instead of each
  repeating the basics.

### SKILL.md shape (all four)

```yaml
---
name: okq-explore
description: <what + when, key use case first; ≤1536 chars with when_to_use>
# okq-reference also sets: user-invocable: false
---

<concise, imperative body: what to do, not why>
```

Bodies stay short — once loaded, a skill stays in context every turn, so the same
conciseness test as CLAUDE.md applies. Shared CLI detail lives in `okq-reference`,
referenced rather than duplicated.

## Acceptance criteria

- [ ] Four `skills/<name>/SKILL.md` files exist, each with a `name` and a
      use-case-first `description`.
- [ ] Each skill's commands run against this repo's own `docs/` bundle and
      produce the documented result (dogfooded, per ADR-0005).
- [ ] `okq-reference` is marked `user-invocable: false`; the other three are
      directly invocable as `/okq-explore`, `/okq-write-okf`, `/okq-maintain`.
- [ ] Skills reference only commands that exist today (no M2/M3 hedging — all
      navigation/health commands are live).
- [ ] The repo's `skills/` directory is discoverable via skills.sh
      (`npx skills find okq` / `npx skills add mikevalstar/okq`).
- [ ] README points to the skills suite and the install command.

## Open questions

- ~~**Packaging for distribution.**~~ Resolved: the skills are **embedded in the
  binary** and installed by [`okq skills install`](./skills-install.md), so
  `skills/` ships in the published crate (kept out of `exclude`; re-check
  `cargo package --list`). skills.sh still indexes the repo as a second channel.
- **Versioning.** Do skills version with the binary (one tag) or independently?
  Embedding ties them to the binary by default; `okq skills install --from-repo`
  ([ADR-0007](../adrs/0007-opt-in-network-for-skill-install.md)) is the
  out-of-band escape hatch.
- **`okq-reference` overlap with CLAUDE.md.** Avoid drift between the skill's
  CLI contract and CLAUDE.md/the per-command feature specs — single source, or
  generate one from the other?

## Related

- ADR-0005 — dogfood okq for docs: `../adrs/0005-dogfood-okq-for-docs.md`
- ADR-0001 — documentation-first, OKF-shaped: `../adrs/0001-documentation-first-okf-shaped.md`
- ADR-0004 — exit-code taxonomy: `../adrs/0004-exit-code-taxonomy.md`
- Commands the skills drive: `./search.md`, `./find.md`, `./get.md`, `./graph.md`
