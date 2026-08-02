---
type: adr
title: ADR-0014 — Surfacing OKF v0.2 semantics — derived trust in the envelope, lint beside validate
status: accepted
created: 2026-08-02
updated: 2026-08-02
tags: [okf, trust, lint, envelope, json-contract, validate, exit-codes]
supersedes: null
superseded-by: null
related:
  - "0013-back-to-upstream-okf.md"
  - "0004-exit-code-taxonomy.md"
  - "0002-library-stack.md"
  - "../features/trust.md"
  - "../features/lint.md"
  - "../features/validate.md"
  - "../guides/design-overview.md"
---

# ADR-0014: Surfacing OKF v0.2 semantics — derived trust in the envelope, lint beside validate

## Context

[ADR-0013](0013-back-to-upstream-okf.md) moved okq onto upstream `okf` 0.2,
which implements OKF spec v0.2. That brought modules okq does not yet use:
`trust`, `actor`, `provenance`, `footnotes`, `lint`, `diff`, `computation`,
`log`. Two of them answer questions okq's users already have and its existing
commands are already shaped for:

- **`trust`** answers *"should I believe this document, and is it still
  current?"* from frontmatter alone — `generated`, `verified`, `status`,
  `stale_after`. That is exactly the decide-whether-to-expand metadata the
  shortlist envelope exists to carry.
- **`lint`** answers *"where is this corpus drifting?"* with 16 coded hygiene
  rules (L1–L16) that go beyond §11 conformance.

Both raise a contract question okq has to answer once, deliberately, because the
`--json` shapes are an agent contract that is expensive to change.

Three specifics force the decision:

1. **The trust tier is *derived*, never stored.** `TrustTier::derive()` reads
   the `verified` list and yields unverified / machine-confirmed /
   human-reviewed. No `--where` predicate can express it, because there is no
   such frontmatter key. Surfacing it means putting a *computed* value in an
   envelope that has so far carried only things you could read off the file.
2. **Absence is meaningful in v0.2, not invalid.** A concept with no trust
   frontmatter is `unverified` and `status: stable`, and must never be rejected
   (§11). So there is always a value to report, for every concept in every
   bundle — including the overwhelming majority that carry no trust keys at all.
3. **okf's lint rule codes are not structured.** `lint_bundle_at()` returns the
   same `Report`/`Diagnostic` type as `validate_bundle()`, and `Diagnostic` has
   no `code` field — the rule code is formatted into the message as `[L7] …`.
   Filtering by rule means parsing okf's message text.

## Options considered

### Option A — Always emit trust fields on every record

Predictable for consumers: `status` and `trust` are always present. But it adds
two fields to every concept in every `find`/`search`/graph result, on corpora
that overwhelmingly do not use trust frontmatter — paying tokens on every query
to say "unverified, stable" over and over. That fights the token-frugality
principle the envelope exists to serve.

### Option B — A separate `okq trust` command

Keeps the shared envelope untouched. But it makes trust a thing you go *ask
about* rather than something you *see while shortlisting*, which is backwards:
the whole value is filtering a result set down to what is verified or not stale
before you expand anything.

### Option C — Emit at the record, omit at the default (chosen)

Trust lives in the shared envelope, but each field is omitted when it holds its
spec-defined default. Absent `status` means `stable`; absent `trust` means
`unverified`; absent `stale` means not stale.

### Option D — `validate --strict` instead of a separate `lint`

Fold lint's rules into `validate` behind a flag. Rejected below.

## Decision

**Option C for trust, and lint as a sibling command.** Concretely:

1. **Derived trust belongs in the shared concept envelope** (`ConceptRecord`),
   beside `type` and `tags` — not in a command of its own. `find` gains
   `--status`, `--trust`, and `--stale` filters; `get` and `stats` report it.

2. **Fields are omitted at their default**, matching how `type` is already
   handled and how the spec treats absence. The defaults are normative and
   documented in the JSON Schema description for each field, so a consumer
   reading `okq schema find` learns them without reading okq's source. A record
   with no trust keys is byte-identical to what okq emitted before this change.

3. **`lint` is a separate command, never a mode of `validate`** (rejecting
   Option D). The two answer different questions and a bundle can legitimately
   fail one and pass the other:

   | | `validate` | `lint` |
   |---|---|---|
   | Question | Does this conform to OKF §11? | Is this corpus drifting? |
   | Authority | The spec | Opinion, from okf |
   | Errors possible | Yes | No — warnings and info only |
   | Affects `conformant` | Yes | **Never** |

   Merging them would let an opinion fail a conformance gate, which is exactly
   the confusion the separation prevents. A bundle with lint findings is still
   conformant.

4. **okq lifts the rule code into a structured `rule` field**, parsing the
   `[Lnn] ` prefix off okf's message and reporting the code separately. This is
   a deliberate bet on an *unstructured* upstream detail: if okf changes the
   prefix format, the code lands as `null` and the message is reported
   unchanged. That degradation is tested. The alternative — making agents
   regex okf's prose to pin a rule — pushes the same fragility onto every
   consumer instead of holding it in one place.

5. **`lint --check` exits 3**, joining `orphans`, `deadlinks`, `index`, and
   `validate` under the "a `--check` run found issues" code from
   [ADR-0004](0004-exit-code-taxonomy.md). It never exits 1 for findings.

6. **Overlap with `orphans` is accepted and documented.** Lint's L15 is
   *narrower* than `okq orphans`: it excludes concepts an `index.md` lists, so a
   deliberately-indexed leaf is not nagged about. `okq orphans` stays the pure
   link-graph view. Neither is redundant; the specs say which to reach for.

7. **The clock is an explicit input.** Staleness (`stale_after`, lint L11) is
   the only okq predicate whose answer depends on the date. `--today
   <YYYY-MM-DD>` makes it reproducible; absent the flag, okq uses the system
   date. Determinism holds in the form that matters: *same bundle plus same
   `--today` → same answer.*

## Consequences

- **The envelope grows for the first time since `tags`.** The change is
  additive and default-omitted, so existing agent parsers keep working and
  existing snapshots for trust-free bundles are unchanged. Adding a *required*
  field later would not be so cheap; this sets the precedent that envelope
  growth is opt-out-by-default.
- **okq now reports a computed value.** `trust` is not in anyone's frontmatter.
  The schema description says so, and `get` shows the `verified` events it was
  derived from, so the derivation is auditable rather than magic.
- **We own a parse of okf's message format.** Small, isolated to
  `commands/lint.rs`, and degrades to `rule: null` rather than breaking. Worth
  revisiting if okf ever adds a real `code` field — that would be a one-line
  simplification, not a contract change, since the `rule` field's shape would
  not move.
- **Two health commands to explain.** `validate` for conformance, `lint` for
  hygiene, `orphans`/`deadlinks` for focused link questions. The README and
  both specs carry the same one-line disambiguation so a user picking between
  them does not have to guess.
- **Provenance, diff, computation, and log stay unsurfaced** for now, for
  reasons recorded in [trust.md](../features/trust.md)'s open questions rather
  than decided here.

## Related

- [ADR-0013](0013-back-to-upstream-okf.md) — the move to okf 0.2 that made these
  modules available
- [ADR-0004](0004-exit-code-taxonomy.md) — the shared exit codes `lint --check`
  joins
- [ADR-0002](0002-library-stack.md) — okf owns the data layer; okq presents it
- [trust.md](../features/trust.md) — the trust & lifecycle feature
- [lint.md](../features/lint.md) — the lint command
- [validate.md](../features/validate.md) — the conformance command lint sits
  beside
- [design-overview.md](../guides/design-overview.md) — token-frugality and the
  agent contract this weighs against
