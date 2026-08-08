---
type: feature
title: okq spec
description: Print the OKF specification this build implements — the exact upstream v0.2 text, embedded in the binary.
tags: [okf, spec, reference]
status: active
command: okq spec
generated: { by: okq/0.7.1, at: 2026-08-08 }
verified: { by: "human:mikevalstar", at: 2026-08-08 }
related:
  - ../adrs/0015-embed-the-okf-spec-text-in-the-binary.md
---

# okq spec

## Summary

`okq spec` prints the Open Knowledge Format specification, verbatim, from a
copy embedded in the binary — the format rules okq itself implements, available
offline with no bundle required. `--section` narrows to one heading, the same
expand-on-demand move `get --section` makes.

## Motivation

An agent asked to *author or judge* OKF needs the normative rules (trust tiers,
actor convention, conformance), not just okq's usage docs. The authoritative
text lives in an external repo, which means a network fetch — off-brand for a
local-first tool — and it may describe a newer revision than the installed okq
implements. Shipping the exact text pins "the spec" to "the spec this build
targets" ([ADR-0015](../adrs/0015-embed-the-okf-spec-text-in-the-binary.md)).

## Scope

### In scope

- The whole document, byte-for-byte identical to the vendored `spec/SPEC.md`.
- `--section <heading>` — one section by heading text or slug, reusing `get`'s
  matcher semantics.
- `--json` — an `okq.spec/v1` envelope carrying `okf_version`, `source`,
  `license`, and the text; registered with `okq schema`.

### Out of scope

- Fetching the latest spec from the network (the pin is the point; see
  ADR-0015).
- A table of contents / listing mode — `okq spec | less` and the section flag
  cover it until real demand shows up.

## Behavior

- **Invocation** — `okq spec [--section <HEADING>] [--json]`. Needs no bundle;
  works in an empty directory. Never prompts.
- **Output** — stdout is the exact spec text (or the one section), with no
  added header or footer, so `okq spec > SPEC.md` reproduces the upstream file.
  `--json` emits the envelope instead; `section` and `line` appear only when
  `--section` was given.
- **Exit codes** — `0` on success; `5` when `--section` matches no heading or
  more than one (the shared section taxonomy); `2` for usage errors.

## Acceptance criteria

- [x] `okq spec` output is byte-identical to `spec/SPEC.md` (tested by
      comparing against the same `include_str!`).
- [x] `--section` selects by heading text or slug and stops at the next
      sibling-or-shallower heading; misses exit `5`.
- [x] `--json` carries `okf_version: "0.2"`, provenance, and license; the
      envelope is in `okq schema spec`.
- [x] Works with no bundle present.

## Open questions

- Should `okq spec --section` support bare section numbers (`--section 5.3`)?
  Deferred until the heading/slug forms prove insufficient.

## Related

- [ADR-0015](../adrs/0015-embed-the-okf-spec-text-in-the-binary.md) — the
  vendoring decision, provenance, and update procedure.
- [ADR-0007](../adrs/0007-opt-in-network-for-skill-install.md) — why okq
  doesn't fetch the spec from the network.
