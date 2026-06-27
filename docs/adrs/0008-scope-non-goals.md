---
type: adr
title: ADR-0008 — Scope & non-goals (no MCP server; vector search not planned)
status: accepted
created: 2026-06-27
updated: 2026-06-27
tags: [scope, non-goals, mcp, vectors, roadmap]
supersedes: null
superseded-by: null
related:
  - "../guides/design-overview.md"
  - "0002-library-stack.md"
  - "../features/search.md"
---

# ADR-0008: Scope & non-goals (no MCP server; vector search not planned)

## Context

okq's full command surface is shipped and stable (see
[design-overview.md](../guides/design-overview.md) and
[CHANGELOG.md](../../CHANGELOG.md)). The former `PLAN.md` carried two "Later"
items as open possibilities: an **MCP server** (`okq mcp`) and **semantic / vector
retrieval**. With the build-out done and the project moving to a maintained,
release-driven posture, leaving these as vague "maybe later" entries invites
scope creep and repeated re-litigation. This ADR settles them so the boundary is
explicit and citable.

## Options considered

### MCP server (`okq mcp`)

A server exposing `search`/`neighbors`/`path` as MCP tools. Considered, **not
planned.** okq is already a clean agent interface: every command is
non-interactive with `--json` and a documented exit-code contract
([ADR-0004](0004-exit-code-taxonomy.md)), so an agent harness can shell out to it
directly. The bundled [agent skills](../features/skills.md) already teach agents
the retrieval loop over the CLI. An MCP layer adds a long-running server, a
protocol surface, and maintenance for capability okq delivers as a subprocess
today — cost without a matching gap. The agent skills are the chosen
agent-integration path.

### Vector / semantic retrieval

A second, embedding-based retriever. Considered, **deferred and evidence-gated —
not planned.** It conflicts with the local-first / no-network and
dependency-light principles, and pure-vector search blurs exactly the exact-token
queries (IDs, API names, error codes) that dominate technical bundles.

## Decision

1. **No MCP server.** okq's agent interface is the CLI (`--json` + exit codes) plus
   the bundled skills. Reconsider only if a concrete agent integration proves the
   subprocess model genuinely insufficient.
2. **No vector search on the roadmap.** It remains *evidence-gated*: it earns
   reconsideration only on a concrete signal — observed real queries that miss
   because the relevant doc never contains the query's literal terms (vocabulary
   mismatch). Absent that signal, it is not planned.
3. **If vectors are ever added**, the shape is already constrained: a **second
   retriever fused with the lexical one (RRF)**, never a replacement, using a
   **local embedding model only** to preserve the no-network principle. That keeps
   it a future ADR's job, not an open invitation.
4. **These are non-goals, not deprecations.** Nothing is removed; the door is
   closed on speculative build-out, not on a future evidence-backed decision.

## Consequences

- **Scope is legible.** Contributors and users have a citable answer to "does okq
  do MCP / vectors?" — no, by decision, with the conditions for revisiting spelled
  out.
- **The "Later" section of PLAN.md is retired** into this ADR; the design overview
  points here for scope boundaries.
- **Effort stays on the core.** Maintenance, ergonomics, and correctness of the
  shipped surface, rather than speculative subsystems.
- **Reversal is cheap and bounded.** Either non-goal can be reopened with a new ADR
  if its trigger condition is met; this ADR defines those triggers.

## Related

- [design-overview.md](../guides/design-overview.md) — the stable design picture this scopes
- [ADR-0002](0002-library-stack.md) — dependency-lightness and the "embeddings are a future ADR" note this formalizes
- [search.md](../features/search.md) — where the vector-deferral has lived as an open question
