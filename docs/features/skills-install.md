---
type: feature
title: okq skills (install / list)
status: active # draft | accepted | active | deprecated
created: 2026-06-27
updated: 2026-06-27
tags: [skills, install, agents, distribution]
milestone: M4.5 # roadmap milestone from PLAN.md §7
command: okq skills
related:
  - ./skills.md
  - ../adrs/0007-opt-in-network-for-skill-install.md
  - ../adrs/0005-dogfood-okq-for-docs.md
---

# okq skills (install / list)

## Summary

`okq skills install` puts the [okq-* agent skills](./skills.md) on disk so an
agent can use them — copying the skills the binary already carries, or fetching
the latest from the repo, or delegating to skills.sh. `okq skills list` shows the
skills bundled with the binary.

## Motivation

The skills exist in the repo, but adopting them meant manual `cp`/symlink steps or
knowing the skills.sh incantation. A first-class command makes "I have okq, give
me the skills" one line — and lets okq itself own the install layout (a canonical
`.agents/` copy, symlinked into each agent's directory) instead of every user
reinventing it.

## Scope

### In scope

- `okq skills install` — install or update the four okq-* skills. Idempotent:
  re-running updates in place.
- `okq skills list` — list the skills embedded in this binary.
- Two install methods, per the original ask:
  1. **Native** (default) — okq writes the skills itself.
  2. **skills.sh** (`--via-skills-sh`) — delegate to `npx skills add mikevalstar/okq`.
- Two native sources: **embedded** (default) and **`--from-repo`** (latest from
  GitHub; the network exception of [ADR-0007](../adrs/0007-opt-in-network-for-skill-install.md)).
- Two scopes: **project-local** (default) and **`--global`**.

### Out of scope

- Uninstall. Removing a few symlinks/dirs by hand is trivial; revisit if asked.
- Installing into agents other than Claude Code. The layout is agent-neutral
  (`.agents/`), but only the `.claude/skills/` symlink is created for now.
- Bundling skills into the `cargo install` artifact beyond what embedding already
  achieves (the skills are *in* the binary; that is the distribution).

## Behavior

### Install layout

Skills install into a canonical `.agents/skills/<name>/` directory, then each is
symlinked into the agent's own directory — the same shape skills.sh uses:

```
.agents/skills/okq-explore/SKILL.md      # canonical copy (source of truth on disk)
.claude/skills/okq-explore -> ../../.agents/skills/okq-explore   # symlink
```

- **Project-local (default):** `./.agents` and `./.claude` in the current directory.
- **`--global`:** `~/.agents` and `~/.claude`, available across all projects.
- The relative symlink target (`../../.agents/skills/<name>`) resolves correctly
  at both scopes.
- On platforms without symlinks (Windows), the skill directory is **copied** into
  `.claude/skills/` instead, and the output says so.
- **Update** = re-run install: the `.agents` copy is overwritten and the symlink
  refreshed. An existing okq-managed symlink is replaced; a real (non-symlink)
  directory at the target is left untouched with a warning, never clobbered.

### Invocation & flags

```sh
okq skills install                 # embedded skills, project-local
okq skills install --global        # into ~/.agents + ~/.claude
okq skills install --from-repo     # fetch latest from GitHub (network; ADR-0007)
okq skills install --via-skills-sh # delegate to: npx skills add mikevalstar/okq
okq skills list                    # what's embedded in this binary
```

- `--from-repo` and `--via-skills-sh` are mutually exclusive (skills.sh is its own
  source).
- `--json` emits one `okq.skills/v1` document: scope, source, and per-skill
  `{name, verb (created|updated), linked}`. Human output goes to stderr (this
  command writes files; stdout stays clean for the JSON contract).

### Exit codes

Shared taxonomy ([ADR-0004](../adrs/0004-exit-code-taxonomy.md)):

- `0` — installed/updated, or listed.
- `1` — I/O failure, or a `--from-repo` network/fetch failure (reported, never a
  silent fallback).
- `2` — usage (e.g. `--from-repo` together with `--via-skills-sh`).

## Acceptance criteria

- [ ] `okq skills install` writes `.agents/skills/<name>/SKILL.md` for all four
      skills and symlinks each into `.claude/skills/` (project-local by default).
- [ ] Re-running updates in place (verb `updated`) without error.
- [ ] `--global` targets the home directories.
- [ ] `--via-skills-sh` shells out to `npx skills add mikevalstar/okq` and surfaces
      its exit status.
- [ ] `--from-repo` fetches from GitHub and fails loudly on network error (no
      silent fallback); no other command ever touches the network.
- [ ] `okq skills list` prints the embedded skill names; `--json` is supported.
- [ ] A real directory at a symlink target is not clobbered.

## Open questions

- **More agents.** When a second agent's convention stabilizes, add its symlink
  alongside `.claude/`. The `.agents/` canonical copy is already agent-neutral.
- **Pinning `--from-repo`.** Currently tracks the default branch. A `--ref`/tag
  selector could make network installs reproducible; deferred until wanted.

## Related

- [skills.md](./skills.md) — the skills this installs
- [ADR-0007](../adrs/0007-opt-in-network-for-skill-install.md) — why `--from-repo` is allowed
- [ADR-0005](../adrs/0005-dogfood-okq-for-docs.md) — dogfooding (the skills teach this loop)
