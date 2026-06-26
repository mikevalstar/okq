---
title: ADR-0004 — Exit-code taxonomy
status: accepted
created: 2026-06-26
updated: 2026-06-26
tags: [cli, exit-codes, contract, agents, ci]
supersedes: null
superseded-by: null
related: ["../features/get.md", "../features/find.md", "../features/search.md", "../../CLAUDE.md"]
---

# ADR-0004: Exit-code taxonomy

## Context

Exit codes are part of okq's agent-runnable contract: a script or LLM branches on
`$?` without parsing output. Four commands (`get`, `find`, `search`) already map
errors to codes via `AppError::exit_code` (`src/error.rs`), and each feature spec
flagged "promote the exit-code taxonomy to an ADR so new commands map onto it
rather than inventing codes." M2 (graph) adds five more commands, including
health checks (`orphans`, `deadlinks`) that need a CI-gating signal. Now is the
moment to fix the taxonomy as a stable contract.

The taxonomy must distinguish the cases an agent actually branches on — *did it
run? was my invocation wrong? does the thing exist? is this an internal failure?*
— while treating an **empty result of a query as success**, not an error.

## Decision

okq uses this fixed taxonomy. `src/error.rs` (`AppError::exit_code`) is the single
source of truth; every command maps onto these codes and none invents new ones.

| Code | Meaning | Mapped from |
|------|---------|-------------|
| **0** | Success — *including a valid empty answer* (no matches, no path, no neighbors). A query that ran and found nothing succeeded. | — |
| **1** | Other / internal error: bad bundle, I/O, search-index failure. | `AppError::Bundle`, `AppError::Index` |
| **2** | Usage error: bad flags/args (clap), or a malformed runtime argument (`--where` without `=`, invalid `--regex`, empty/unparseable query). | clap, `AppError::Usage` |
| **3** | *Reserved:* a health check ran cleanly but **found issues**, under an explicit opt-in (e.g. `deadlinks --check`, `orphans --check`). Lets CI gate on findings without conflating them with errors. Not used unless the command opts in. | (M2) |
| **4** | Concept not found / not resolvable (also: a non-unique partial id, with candidates listed). | `AppError::ConceptNotFound`, `AppError::InvalidConcept` |
| **5** | Section not found / ambiguous within a resolved concept. | `AppError::SectionNotFound`, `AppError::SectionAmbiguous` |

Principles:

1. **0 = "ran, answer valid," empty included.** Queries (`find`, `search`,
   `neighbors`, `path`, …) return exit 0 whether or not they matched. Emptiness is
   a result, not a failure.
2. **Errors are distinguishable.** An agent can tell *my input was wrong* (2) from
   *the thing doesn't exist* (4/5) from *something broke* (1) — three different
   recovery paths.
3. **Health findings are opt-in (3).** `orphans`/`deadlinks` are queries by default
   (exit 0, list findings). Only under an explicit `--check`-style flag does
   "found issues" become a non-zero gate (3), so CI can fail the build while
   normal listing stays exit-0.
4. **Codes are stable and only extended.** This is an agent/script contract:
   existing codes are never renumbered; new needs claim a new number (the reason
   for reserving 3 now rather than shifting later).
5. **Single source of truth.** The mapping lives in `AppError::exit_code`; commands
   return typed errors, never call `exit()` with ad-hoc numbers.

## Consequences

- M2/M3 commands inherit the taxonomy: graph lookups reuse 4 for a missing
  concept; `path` with no route between two existing concepts is **exit 0** (empty
  answer); `orphans`/`deadlinks` add a `--check` flag wired to code 3.
- `src/error.rs` may gain variants, but each must map to an existing code; adding a
  *new* code requires amending this ADR.
- The exit-code tables already in the `get`/`find`/`search` specs and in CLAUDE.md
  now reference this ADR as the canonical definition.
- Mild constraint: we've spent codes 0–5; further additions should be deliberate
  and documented here to keep the contract legible.

## Related

- [get](../features/get.md), [find](../features/find.md), [search](../features/search.md) — the specs whose exit-code open question this resolves
- [graph](../features/graph.md) — M2 commands that adopt code 3 (`--check`) and the not-found/empty rules
- [CLAUDE.md](../../CLAUDE.md) — the working-notes copy of this table
