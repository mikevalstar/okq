---
type: adr
title: ADR-0007 — Network is opt-in, scoped to skill install
status: accepted
created: 2026-06-27
updated: 2026-06-27
tags: [network, skills, install, dependencies, local-first]
supersedes: null
superseded-by: null
related:
  - "0002-library-stack.md"
  - "../features/skills-install.md"
  - "../features/skills.md"
  - "../../PLAN.md"
---

# ADR-0007: Network is opt-in, scoped to skill install

## Context

okq's defining contract is **local-first and deterministic**: no network, no
embeddings, no API key — same bundle, same answer, every time (PLAN.md §4,
ADR-0002). Every query command honors this.

`okq skills install` (see [skills-install.md](../features/skills-install.md))
needs to put the [okq-* agent skills](../features/skills.md) on disk. The skill
content can come from two places: the copy **embedded in the binary** at build
time, or the **latest in the GitHub repo**. The latter is a network fetch — the
first in okq — so it needs an explicit decision rather than drifting in.

The tension is only apparent. The no-network rule exists so that *queries* are
reproducible and work offline, on read-only mounts, in CI, with no credentials.
An install command is not a query: it is an explicit, user-initiated maintenance
action, like `cargo install` or `okq new` reading the clock
(`templates.rs` already notes "the determinism principle binds queries, not
writes"). Fetching on install does not weaken any query guarantee.

A network fetch also means a new dependency. okq values a small, self-contained
dependency tree (ADR-0002 chose to vendor search rather than shell out to `rg`),
so the HTTP client must be lightweight and must not pull a system TLS dependency
that would complicate the cross-platform release matrix.

## Options considered

### Option A — Embed only

Bake the skills into the binary; install just writes them out. Fully offline,
no new dependency, perfectly reproducible. Cost: skills can only be updated by
upgrading okq, so a skill fix can't reach users until the next release.

### Option B — Download only

Always fetch from GitHub. Skills update independently of the binary, but every
install needs the network — breaking offline/CI installs — and the no-network
contract is gone even for users who never wanted it.

### Option C — Embed by default, network opt-in (chosen)

Install writes the embedded skills by default (offline, reproducible). A
`--from-repo` flag fetches the latest from GitHub for users who want
newer-than-binary skills. The default path keeps the contract; the network is
present only when the user asks for it on that one command.

## Decision

**Option C.** Concretely:

1. **No query command ever touches the network.** This ADR changes nothing about
   `search`, `find`, `get`, the graph commands, or health checks. The local-first
   contract for queries is unchanged and remains absolute.
2. **`okq skills install` defaults to the embedded skills** — offline, deterministic,
   version-locked to the binary.
3. **`okq skills install --from-repo`** is the *only* code path in okq that makes a
   network request. It fetches the skill set from the GitHub repo at run time.
4. **The HTTP dependency is [`ureq`](https://crates.io/crates/ureq)** — a small,
   blocking client with a rustls/webpki-roots TLS stack (no OpenSSL), so it adds
   no system dependency and builds across the release targets.
5. **Network failure is non-fatal and never silent**: `--from-repo` reports the
   error to stderr and exits non-zero (code 1); it does not fall back to embedded
   without saying so. The plain `install` cannot fail for network reasons.

## Consequences

- **Offline installs stay the norm.** The common case (`okq skills install`)
  needs nothing but the binary, matching how the rest of okq behaves.
- **Skills can still ship fixes out-of-band** via `--from-repo`, without waiting
  for an okq release.
- **New dependency.** `ureq` (and its TLS stack) enters the tree. It is compiled
  in regardless of whether `--from-repo` is used; justified by keeping the fetch
  self-contained rather than shelling out to `curl`/`git`.
- **The binary embeds the skills**, so `skills/` must ship in the published crate
  (it is not in Cargo.toml's `exclude`); re-check `cargo package --list` if that
  changes.
- **The contract statement gets a footnote, not a rewrite.** "No network" remains
  true for queries; docs note the single, opt-in install exception.

## Related

- [ADR-0002](0002-library-stack.md) — dependency-lightness and the vendor-don't-shell-out
  precedent this fetch is weighed against
- [skills-install.md](../features/skills-install.md) — the command this decision enables
- [skills.md](../features/skills.md) — the skills being installed
- [PLAN.md](../../PLAN.md) — §4 principles (local-first / no-network)
