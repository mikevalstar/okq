---
type: adr
title: ADR-0003 — The search index lives in the XDG cache, not the bundle
status: accepted
created: 2026-06-26
updated: 2026-06-26
tags: [rust, search, tantivy, indexing, cache, xdg]
supersedes: "0002 (index-location sub-decision only)"
superseded-by: null
related: ["0002-library-stack.md", "../features/search.md", "../guides/design-overview.md"]
---

# ADR-0003: The search index lives in the XDG cache, not the bundle

## Context

[ADR-0002](0002-library-stack.md) adopted Tantivy as the day-one search backend and stated the persisted index would live in **`.okq/index/` under the bundle root** (git-ignored). While specifying `okq search`, we revisited where that derived state belongs. This ADR revises **only that location sub-decision**; everything else in ADR-0002 stands.

Writing the index *into the bundle* has real downsides:

- **Read-only and shared bundles break.** A bundle on a read-only mount, in a shared/CI checkout, or owned by another user can't accept an `.okq/` write — yet querying it should still work.
- **It touches the repo.** Even git-ignored, an in-tree `.okq/` is a stray directory in everyone's working tree, one more thing to ignore, clean, and reason about. The bundle is *input*; okq writing into its input couples the tool to the corpus's writability.
- **Per-bundle isolation is awkward** when the same tree is checked out in multiple places.

## Options considered

### Option A — `.okq/index/` in the bundle root (ADR-0002's choice)

Co-located and obvious; easy to find and delete. But requires a writable bundle, writes into the repo, and coupes querying to corpus writability.

### Option B — A per-bundle XDG cache directory

Store the index under `${XDG_CACHE_HOME:-~/.cache}/okq/<bundle-key>/`, where `<bundle-key>` derives from the bundle's canonical absolute path. Never writes into the bundle. Standard location for derived caches; survives read-only corpora. Cost: the cache is off to the side (less discoverable) and staleness is keyed by path, so moving/renaming a bundle orphans its cache.

## Decision

**Option B.** The Tantivy index and its staleness manifest live in a **per-bundle XDG cache directory** — `${XDG_CACHE_HOME:-~/.cache}/okq/<bundle-key>/` — never inside the bundle. Concretely:

1. **okq never writes into a queried bundle.** The bundle is read-only input; all derived state goes to the cache.
2. **`<bundle-key>`** is derived from the bundle's canonicalized absolute path so the same bundle maps to a stable cache across runs (exact hashing scheme is an open question in [search.md](../features/search.md)).
3. **`--ephemeral`** builds a transient in-memory index (writes nothing); okq also **falls back to ephemeral** (with a stderr note) when the cache directory can't be created.
4. The index remains a **derived, rebuildable cache, never source of truth** — the one ADR-0002 invariant that carries over unchanged.

## Consequences

- **Read-only / shared / CI bundles just work** — the motivating win. No bundle writability assumption, and nothing to git-ignore in the corpus (the `.okq/` gitignore follow-up from ADR-0002 is dropped).
- **New concern: cache lifecycle.** Per-bundle cache dirs accumulate and are keyed by path, so renaming/moving a bundle orphans its cache. This creates follow-up work — a derivation scheme and an eventual `okq cache clear`-style cleanup (tracked in [search.md](../features/search.md) open questions and PLAN.md §8).
- **Discoverability cost:** users can't see the index next to their docs; documentation must point at the cache path, and `--reindex` / ephemeral give escape hatches.
- **PLAN.md and ADR-0002** references to `.okq/index/` are superseded by this ADR; PLAN.md §6/§8 are updated to describe the XDG cache.

## Related

- [ADR-0002](0002-library-stack.md) — Tantivy as the backend (this ADR revises only its index-*location* sub-decision)
- [search.md](../features/search.md) — the feature that depends on this; carries the open questions on key derivation, staleness, and cleanup
- [PLAN.md](../guides/design-overview.md) — §6 architecture, §8 index-lifecycle open questions
