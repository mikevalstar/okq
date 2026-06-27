---
type: workflow
title: Workflow name (verb phrase, e.g. "Explore a bundle")
status: draft # draft | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: []
actors: [user] # who/what participates: user, agent, okq, ci
related: [] # paths to related docs
---

# Workflow name

## Goal

What the user or agent is trying to accomplish, in one sentence.

## Preconditions

What must be true before this workflow starts (e.g. "an OKF or OKF-shaped bundle exists at the target dir", "`okq` is on PATH").

## Steps

1. Each step from the caller's point of view, noting the exact `okq` command run underneath.
2. Include the real invocation (e.g. `okq search "auth" --json`, then `okq neighbors <hit> --depth 1`).
3. Note decision points and branches ("if `search` returns nothing, fall back to `find --match`").
4. For agent workflows, show how output of one command feeds the next (the search → neighbors → get composition is the core ergonomic — PLAN.md §5).

## Outcome

The end state when the workflow succeeds — what the user has learned or what the agent has assembled into context.

## Failure modes

| What can go wrong | How the caller finds out | Recovery |
|-------------------|--------------------------|----------|
| e.g. concept not found | non-zero exit + message | check `okq find --match`, fix the reference |

## Related

- Features (commands) that implement this workflow, relevant ADRs, the agent skill that teaches it.
