---
type: feature
title: Trust & lifecycle — status, trust tier, and staleness
status: active # draft | accepted | active | deprecated
created: 2026-08-02
updated: 2026-08-02
tags: [trust, lifecycle, status, staleness, okf, find, get, stats, agents]
milestone: null
command: null # cross-cutting: envelope fields plus filters on find
related:
  - "../adrs/0014-surfacing-okf-v02-semantics.md"
  - "../adrs/0013-back-to-upstream-okf.md"
  - "find.md"
  - "get.md"
  - "stats.md"
  - "lint.md"
  - "../guides/design-overview.md"
---

# Trust & lifecycle — status, trust tier, and staleness

## Summary

Every concept okq returns carries the OKF v0.2 answer to *"should I believe
this, and is it still current?"* — its lifecycle `status`, its derived **trust
tier**, and whether it is past its `stale_after` date. `find` filters on all
three, `get` shows the evidence behind them, and `stats` reports how the bundle
is distributed.

## Motivation

A shortlist exists so a caller can decide what to expand without reading
anything. Until now okq answered "what is this about" (`type`, `title`, `tags`)
but not "how much should I trust it" — so an agent assembling context could not
tell a human-reviewed, current runbook from a machine-generated draft that went
stale four months ago. It would expand both, spend tokens on both, and weight
them equally.

OKF v0.2 puts that signal in frontmatter: `generated: {by, at}` records who
wrote it, `verified: [{by, at}]` records who confirmed it, `status` records
where it sits in its lifecycle, and `stale_after` records when it stops being
safe to trust. okf 0.2 ([ADR-0013](../adrs/0013-back-to-upstream-okf.md)) parses
all four and **derives** the trust tier from `verified`.

That derivation is why this can't be `--where`: there is no `trust:` key to
match on. `--where verified=…` cannot express "confirmed by a human at some
point", and the spec requires reading a bare `verified: {by, at}` mapping as a
one-element list. The logic belongs in one place, and okf already has it.

## Scope

### In scope

- **Three derived values per concept**, from okf: `status` (draft / stable /
  deprecated), `trust` (unverified / machine-confirmed / human-reviewed), and
  `stale` (past `stale_after` on a given day).
- **Envelope fields** on the shared concept record, so `find`, `search`, and the
  graph commands all report them identically — omitted when at their default
  ([ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md)).
- **`find` filters**: `--status`, `--trust`, `--stale`, plus `--today` to pin
  the day staleness is evaluated against.
- **`get` detail**: the `generated` and `verified` events the tier was derived
  from, so the derivation is auditable.
- **`stats` distributions**: counts by trust tier and by status, beside the
  existing type and tag distributions.

### Out of scope

- **Writing or updating trust frontmatter.** okq is read/query only outside
  `init`/`new`; recording a verification is an editor's job.
- **Ranking by trust in `search`.** Retrieval stays lexical (BM25); trust is a
  filter and a display value, not a scoring input. Boosting verified docs is a
  plausible future change, but it would need evidence that it helps, not just
  that it sounds right.
- **Provenance (`sources`) and per-claim attribution.** A separate feature; see
  Open questions.
- **Enforcing a policy** ("fail if anything is unverified"). That is lint's
  job — L4 and L11 already say it. See [lint.md](lint.md).

## Behavior

### Values

| Field | Values | Default when absent | Derived from |
|-------|--------|---------------------|--------------|
| `status` | `draft`, `stable`, `deprecated` | `stable` | frontmatter `status` |
| `trust` | `unverified`, `machine-confirmed`, `human-reviewed` | `unverified` | `verified[]` actors |
| `stale` | `true` / `false` | `false` | `stale_after` vs. `--today` |

The tier is derived, not stored: **any** `verified` entry by a `human:<id>`
actor makes it `human-reviewed`; entries by non-human actors only make it
`machine-confirmed`; no entries at all is `unverified`.

Actors follow the §7 convention: `human:<id>` for a person, `process:<id>` for
an automated process, and `<producer>/<version>` for an agent or tool
(`okq/0.7.0`). Only the `human:` prefix moves the tier. Anything else is kept
verbatim rather than rejected — the spec itself uses forms like
`team:ga4-docs`.

An unknown `status` value is reported verbatim rather than coerced — §5.4
defines three values but requires consumers to accept others.

### Invocation & flags

```sh
okq find --status draft                    # everything still in progress
okq find --trust human-reviewed            # only what a person signed off
okq find --stale --today 2026-08-02        # past its stale_after, reproducibly
okq find --status deprecated --json        # for a script
okq get runbooks/deploy                    # shows generated + verified events
okq stats                                  # distributions by tier and status
```

`--status` and `--trust` are repeatable and OR within themselves, AND across
flags — matching `--type`'s established behavior. `--stale` needs no argument;
`--today <YYYY-MM-DD>` pins the evaluation date and defaults to the system date.

### Output

Envelope fields are **omitted at their default**, so a bundle with no trust
frontmatter produces exactly the JSON it did before this feature:

```json
{
  "id": "runbooks/deploy",
  "type": "runbook",
  "title": "Deploying to production",
  "path": "runbooks/deploy.md",
  "line": 1,
  "tags": ["ops"],
  "status": "draft",
  "trust": "human-reviewed",
  "stale": true
}
```

Human output appends the non-default values to the existing line rather than
adding a column, so `find` stays scannable:

```
runbooks/deploy.md:1  Deploying to production  [draft, human-reviewed, stale]
```

`get` prints the underlying events, since the tier is a derivation and a caller
may want the evidence:

```
generated  writer/1.0   2026-04-01
verified   human:mike   2026-07-14
```

### Exit codes

Unchanged, from the shared taxonomy. A filter that matches nothing is **0** with
an empty result — a trust filter finding no verified docs is an answer, not an
error. There is deliberately no `--check` here; gating on trust is
[lint](lint.md)'s job.

## Acceptance criteria

- [ ] A concept with no trust frontmatter is `stable` / `unverified` / not
      stale, and its JSON record is unchanged from before this feature.
- [ ] A bare `verified: {by, at}` mapping is read as a one-element list (§5.2).
- [ ] Any `human:` verifier yields `human-reviewed`; non-human verifiers only
      yield `machine-confirmed`; no `verified` key yields `unverified`.
- [ ] An unknown `status` value is reported verbatim, not coerced or dropped.
- [ ] `find --status` / `--trust` are repeatable, OR within a flag and AND
      across flags; an empty result exits 0.
- [ ] `find --stale --today <date>` is reproducible: same bundle plus same
      `--today` gives the same answer.
- [ ] `get` shows `generated` and `verified` events when present, and omits the
      block entirely when absent.
- [ ] `stats` reports trust-tier and status distributions.
- [ ] `okq schema find` documents each field's default.
- [ ] Malformed trust frontmatter (`verified: "yesterday"`, a scalar
      `generated`) degrades to the default without a panic.

## Open questions

- **Provenance (`sources`) and footnote attribution** — okf 0.2 ships
  `provenance::attributions()`, which joins body footnotes to `sources[]` by
  label and flags labels that resolve to nothing. That is a genuine doc-rot
  check in the same family as `deadlinks`, and the strongest grounding story okq
  could tell an agent. Deferred because no bundle we can dogfood against carries
  v0.2 `sources` frontmatter yet — including okq's own `docs/`. Revisit when one
  does.
- **`okq diff <a> <b>`** — `okf::bundle_diff()` gives a semantic bundle diff
  with content-hash rename detection and trust-tier transitions. The blocker is
  not the API but that okq has no notion of a second snapshot; two directory
  paths is easy and low-value, two git revisions is where the value is and needs
  a subprocess okq has never had.
- **Should `search` accept trust filters?** The index would need the fields in
  its schema and a rebuild. Deferred until someone wants it.

## Related

- [ADR-0014](../adrs/0014-surfacing-okf-v02-semantics.md) — why trust lives in
  the envelope and is omitted at its default
- [ADR-0013](../adrs/0013-back-to-upstream-okf.md) — the okf 0.2 move that
  brought the trust module
- [find.md](find.md) — the filters this extends
- [get.md](get.md) — where the derivation's evidence is shown
- [stats.md](stats.md) — the distributions
- [lint.md](lint.md) — the gating counterpart (L4 unverified, L11 stale, L12
  draft)
- [design-overview.md](../guides/design-overview.md) — token-frugality, which
  drove the omit-at-default choice
