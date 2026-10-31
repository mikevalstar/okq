---
type: adr
title: ADR-0017 — Keep our process status values, outside OKF's lifecycle set
description: This repo's docs keep status draft/accepted/active/superseded even though OKF v0.2 defines only draft/stable/deprecated.
status: accepted
created: 2026-10-31
updated: 2026-10-31
tags: [process, documentation, okf, status, conformance]
generated: { by: human:mikevalstar, at: 2026-10-31T00:00:00Z }
supersedes: null
superseded-by: null
related:
  - "0001-documentation-first-okf-shaped.md"
  - "0005-dogfood-okq-for-docs.md"
  - "../features/trust.md"
  - "../features/validate.md"
  - "../README.md"
---

# ADR-0017: Keep our process status values, outside OKF's lifecycle set

## Context

OKF v0.2 (§5.4) defines `status` as a content lifecycle: `draft`, `stable`,
`deprecated`, with absent meaning `stable`. It also says consumers must accept
values they don't recognise.

This repo's docs predate that and use `status` for where a decision sits in our
process: `draft` (being written), `accepted` (agreed, not built), `active`
(shipped), `superseded`/`deprecated` (replaced or retired). The
[docs README](../README.md) and the build process in `CLAUDE.md` both rely on the
`accepted` → `active` step to say whether a spec has been implemented.

`okq validate` warns on each non-OKF value. Today that is 39 of the bundle's
warnings, and they bury the warnings that point at real problems.

## Options considered

### Option A — map onto OKF's values

`accepted`/`active` become `stable`, `superseded` becomes `deprecated`. The
bundle is warning-free on status, but we lose "agreed but not built", and
`superseded` loses its distinct meaning (the `superseded-by` key still points at
the replacement).

### Option B — map, and move process state to an extension key

OKF values in `status`, our process state in a separate key such as `decision`.
Nothing is lost, but every doc carries two lifecycle fields that have to agree.

### Option C — keep our values

Leave `status` as it is. The spec allows it, and okq reports unknown values
verbatim ([trust](../features/trust.md)), so `okq find --status active` keeps
working on this bundle.

## Decision

Option C. The process states are what we actually query when working in this
repo, and the spec says consumers must tolerate them. Of the two costs, a known
and documented warning count is cheaper than a second status field or losing
the `accepted` step.

## Consequences

- `okq --bundle docs validate` reports one `unknown status` warning per
  `accepted`/`active`/`superseded` doc. That is expected. Every other warning
  is still worth fixing.
- An OKF consumer reading this bundle sees non-standard statuses. A strict
  consumer may treat them as unknown instead of `stable`.
- If okq gains per-rule suppression in `validate`, this is the first rule to
  silence for this bundle.

## Related

- [ADR-0001](0001-documentation-first-okf-shaped.md): the docs tree is OKF-shaped, and divergences from OKF get an ADR
- [ADR-0005](0005-dogfood-okq-for-docs.md): we query our own docs with okq
- [trust](../features/trust.md): how okq surfaces `status`, including unknown values
- [validate](../features/validate.md): where the warning comes from
