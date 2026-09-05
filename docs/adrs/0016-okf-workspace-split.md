---
type: adr
title: ADR-0016 — Depend on okf-core and okf-validator, and absorb okf 0.2.7's validate/lint rebalance
status: accepted
created: 2026-09-05
updated: 2026-09-05
tags: [okf, dependency, lint, validate, trust, compatibility, rule-codes]
generated: { by: human:mike, at: 2026-09-05T00:00:00Z }
supersedes: null
superseded-by: null
related:
  - "0013-back-to-upstream-okf.md"
  - "0014-surfacing-okf-v02-semantics.md"
  - "0002-library-stack.md"
  - "../features/lint.md"
  - "../features/validate.md"
  - "../features/trust.md"
  - "../features/scaffold.md"
---

# ADR-0016: Depend on okf-core and okf-validator, and absorb okf 0.2.7's validate/lint rebalance

## Context

[ADR-0013](0013-back-to-upstream-okf.md) put okq on upstream `okf = "0.2.1"`.
Upstream has since shipped 0.2.7, and three things changed at once.

**1. The crate became a workspace.** `okf` is now a *CLI binary* crate that
re-exports two libraries:

| crate | contents |
|-------|----------|
| `okf-core` | parser, model, bundle, links, trust, markdown, refactor, fix, scaffold |
| `okf-validator` | `validate_bundle`, `lint_bundle_at`, `Diagnostic`, `Severity` |
| `okf` | the `okf` command-line tool, re-exporting both |

Depending on `okf` now means depending on a CLI to get a library, and its
default features pull `clap`, `serde_json`, `okf-studio`, and four language
parsers (`oxc` for JavaScript, `rustpython-parser`, `syn`, `sqlparser`). Those
parsers take okq's dependency tree from 343 to 566 crates and raise the minimum
rustc to 1.96.

**2. validate and lint were rebalanced.** In 0.2.1 lint carried sixteen rules,
most of them frontmatter and link checks. In 0.2.7 those moved into
`validate_bundle` as `V`-coded warnings, and `lint_bundle_at` kept only
authoring and formatting hygiene. The lint codes that survived were
**renumbered**. Every rule okq documents shifted meaning or moved commands:

| okf 0.2.1 lint | okf 0.2.7 | Finding |
|----------------|-----------|---------|
| L1, L2, L3 | validate V3 | missing `title` / `description` / `generated` |
| L4 | lint **L11** | no `verified` events |
| L5, L6 | validate V19, V20 | legacy v0.1 `timestamp` / `# Citations` |
| L7 | validate V4 | body is empty |
| L8 | lint **L1** | body has no top-level `#` heading |
| L9 | validate V8 | latest `verified.at` predates `generated.at` |
| L10 | validate V29 | links to a `status: deprecated` concept |
| L11 | validate V12 | past `stale_after` |
| L12 | lint **L12** | `status: draft` (unchanged) |
| L13 | lint **L10** | self-link |
| L14 | validate V30 | `title` shared with another concept |
| L15 | lint **L9** | orphan concept |
| L16 | validate V35 | `index.md` out of sync with its directory |

Lint also gained rules okf had no equivalent of before: canonical frontmatter
key order (L2), heading hierarchy drift (L3), empty sections (L4), uncited
declared sources (L5), non-standard actor identities (L6), untagged
`# Computation` code blocks (L7), whitespace hygiene (L8), and an unquoted
`okf_version` (L13).

Staleness moved with it. `lint_bundle_at` now ignores the date it is handed;
`validate_bundle_at` is where a `today` argument does anything. `okq lint
--today` had become a flag that changed nothing.

**3. Timestamps got strict.** `Verification::is_valid()` is new, and
`stale_after` changed type from `DateField` to `DateTimeField`. Both now require
a full ISO-8601 datetime *with a time of day and an explicit UTC offset*. Under
0.2.7, a bare `at: 2026-07-14` is not a valid verification event, so
`trust_tier()` silently returns `unverified`; a bare `stale_after: 2026-01-01`
never reports stale.

That last one is the sharp edge. Bare dates are what okq's own templates emit,
what `docs/features/_template.md` documents, and what is already committed in
every bundle scaffolded by okq 0.3–0.8. A silent downgrade to `unverified` is
the worst possible failure mode for a trust signal: `okq find --trust
human-reviewed` would quietly stop matching documents a human really did review.

## Options considered

### Option A — Stay on `okf = "0.2.1"`

Nothing breaks today. But it pins okq to a version upstream has moved off, and
the fixes and modules in 0.2.7 (`markdown`, `refactor`, `diff`) are exactly the
things okq currently hand-rolls. The 0.2.x line is not going to wait.

### Option B — Depend on the `okf` facade at 0.2.7

One dependency line, and `okf::` paths keep working unchanged. But it makes a
CLI crate a library dependency, and its default features add 223 crates for
code-block syntax linting that produced **zero** extra findings on okq's own
corpus. `default-features = false` would drop the validator entirely, which okq
needs.

### Option C — Depend on `okf-core` + `okf-validator` directly (chosen)

Take the two libraries and skip the CLI. `okf-validator` with
`default-features = false` keeps `validate_bundle` and `lint_bundle_at` without
the language parsers.

Keep the `package = "okf-core"` rename in `Cargo.toml` so `okf::` paths across
the twenty-odd files that use the data layer stay as they are — the rename is
one line and the alternative is a mechanical `okf_core::` diff that buys
nothing. Only the four validator symbols (`Severity`, `Diagnostic`,
`validate_bundle`, `lint_bundle_at`) move to an `okf_validator::` path, and only
in the two command modules that use them.

## Decision

**Option C.**

```toml
okf = { package = "okf-core", version = "0.2.7" }
okf-validator = { version = "0.2.7", default-features = false }
```

The language parsers stay off. They are a real capability — syntax-checking
`# Computation` blocks — but okq is a query tool, not an Attested Computation
authoring environment, and 223 crates is not the price to pay before anyone asks
for it. Turning the feature back on is a one-line change if that changes.

**okq keeps reading bare dates.** okq derives the trust tier and staleness
itself in [`src/trust.rs`](../../src/trust.rs). The tier counts any event that
**names a verifier**, because it answers *who* reviewed this rather than *when*;
staleness accepts a `stale_after` that parses as *either* a date or a datetime.
Both are exactly what okf 0.2.1 did. okf's own `trust_tier()` and
`is_stale_on()` are no longer called for these two answers.

This is a deliberate exception to "don't reimplement what okf provides"
([ADR-0002](0002-library-stack.md)). The justification is narrow and specific:
okq is a **reader** of bundles it did not write, including ones its own older
releases scaffolded. A reader that silently reclassifies existing, valid-when-
written frontmatter as untrusted is worse than a reader that is lenient about a
timestamp's precision. okf is right to be strict — it is the format's reference
implementation, and `okq validate` still surfaces that strictness as V6, V7 and
V11 warnings so an author is told to tighten the value. okq is simply not
willing to change the *answer* to "has a human reviewed this?" based on whether
someone wrote a timezone.

**New writes are strict.** `okq init` and `okq new` emit
`2026-09-05T00:00:00Z` — a valid OKF v0.2 timestamp — so scaffolded bundles are
clean under the current validator. Old bundles keep working; new ones are
correct.

**okq adopts okf's rule numbering as-is.** `--rule` / `--ignore` accept L1–L13,
and the codes mean what okf 0.2.7 says they mean. okq does not maintain a
compatibility shim mapping old codes to new ones: the rule set is okf's
([ADR-0014](0014-surfacing-okf-v02-semantics.md)), a shim would be a second
source of truth for it, and pinned codes that silently changed meaning would be
more dangerous than an unknown-code usage error. `okq lint --ignore L15` now
exits 2 with the known codes listed, which is the loud failure a CI config
deserves.

**`--today` moves from `lint` to `validate`,** following the staleness rule it
exists for. `okq validate --today` calls `validate_bundle_at` and reports stale
concepts as V12.

## Consequences

- Findings that used to come from `okq lint` now come from `okq validate` as
  warnings. On okq's own `docs/`, lint drops from 123 findings to 82 and
  validate rises from 118 warnings to 123. A CI job gating on `okq lint --check`
  will get quieter; one gating on `okq validate --check` is unaffected, since
  `--check` keys on *errors* and every moved rule is a warning.
- `okq lint --rule`/`--ignore` values from before 0.9.0 are wrong. L14, L15 and
  L16 no longer exist, and L1, L9, L10, L11 mean different things. This is a
  minor-version break in a documented contract, called out in the changelog.
- `okq lint --today` is gone. It was already a no-op under 0.2.7.
- okq owns a small piece of trust semantics it did not own before. The
  divergence is bounded — two predicates, both documented in
  [features/trust.md](../features/trust.md) — and it should be revisited if okf
  ever offers a lenient parsing mode.
- okq's minimum rustc is unchanged by this, because the language parsers that
  needed 1.96 are off.
- `okf-core` 0.2.7 brings `markdown` (`extract_headings`, `heading_slug`,
  `rewrite_markdown_links`), `refactor`, `diff`, and `scaffold` modules that
  overlap with code okq hand-rolls in `sections.rs` and `templates.rs`. Not
  adopted here — this ADR is about the migration, not a rewrite — but they are
  the obvious next place to delete okq code.
- `Diagnostic` gained a `fixable` flag marking what `okf fix` could repair. Not
  surfaced in the `okq.lint/v1` envelope yet; it is additive when it is.

## Related

- [ADR-0013](0013-back-to-upstream-okf.md) — the move to upstream okf 0.2 this
  continues
- [ADR-0014](0014-surfacing-okf-v02-semantics.md) — the envelope and rule-code
  contract whose rule table this replaces
- [ADR-0002](0002-library-stack.md) — "okf owns the data layer", and the narrow
  exception taken here
- [features/lint.md](../features/lint.md) — the rebuilt rule table
- [features/validate.md](../features/validate.md) — where `--today` went
- [features/trust.md](../features/trust.md) — the timestamp formats okq accepts
