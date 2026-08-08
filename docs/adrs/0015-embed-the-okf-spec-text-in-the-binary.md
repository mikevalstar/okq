---
type: adr
title: Embed the OKF spec text in the binary
description: okq spec prints a vendored, byte-exact copy of the upstream OKF v0.2 SPEC.md, embedded at compile time.
tags: [okf, spec, vendoring]
status: accepted
generated: { by: okq/0.7.1, at: 2026-08-08 }
verified: { by: "human:mikevalstar", at: 2026-08-08 }
related:
  - ../features/okq-spec.md
---

# ADR-0015: Embed the OKF spec text in the binary

## Status

Accepted.

## Context

okq implements OKF v0.2, but the authoritative spec lives in someone else's
repository ([GoogleCloudPlatform/knowledge-catalog](https://github.com/GoogleCloudPlatform/knowledge-catalog)).
An agent (or human) working against a bundle needs the format rules — what
`verified` means, how trust tiers derive, what conformance requires — and today
that means a network fetch of a document that may have drifted ahead of what
this okq build actually implements. okq is local-first and offline by design
([ADR-0007](0007-opt-in-network-for-skill-install.md) made the sole network
exception explicit), so "go read the website" is off-brand, and paraphrasing
the spec in our own docs invites divergence.

Options considered:

- **A — Link to the upstream URL** in `--help`/README. No binary weight, but
  offline agents get nothing, and the URL serves whatever the spec says
  *today*, not what this build implements.
- **B — Paraphrase the spec** in okq's own docs/skills. Already partly true
  for usage guidance, but a paraphrase is a second source of truth that rots.
- **C — Vendor the exact upstream text** and print it verbatim from the
  binary (`okq spec`).

## Decision

Option C. `spec/SPEC.md` is a byte-exact copy of upstream `okf/SPEC.md`,
pinned to a commit (provenance in `spec/README.md`), embedded via
`include_str!` — the same pattern the skills already use — and printed
verbatim by `okq spec`. Licensing is clean: the spec is Apache-2.0 and so is
okq; the upstream license ships alongside the copy (`spec/LICENSE.md`).

The version pin is a feature, not a limitation: the embedded text is the spec
*as this okq build implements it*. Updating the copy is part of adopting a new
spec revision, never a routine sync — the vendored text and okq's behavior
move together (procedure in `spec/README.md`).

## Consequences

- Easier: agents get the authoritative format rules offline, from the binary
  they already trust, pinned to the implemented version. Verbatim output means
  zero drift between what we ship and what upstream wrote at the pinned commit.
- Harder: the binary grows ~30 KB, and each spec revision adds a manual
  vendoring step (mitigated by the checklist in `spec/README.md` and a test
  that guards the copy's identity).
- A deliberate consequence: if upstream moves ahead of okq, `okq spec` keeps
  printing the version okq actually implements — that gap is visible in
  `okf_version`, not silently papered over.
