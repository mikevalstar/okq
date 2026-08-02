---
type: fixture
title: Trust frontmatter — every shape, valid and malformed
status: percolating
tags: [tests, robustness, fixtures, trust]
generated: { by: okq/0.7.0, at: 2026-08-02 }
verified:
  - { by: process:nightly, at: 2026-08-01 }
  - { by: human:mike, at: 2026-08-02 }
  - { by: "", at: 2026-08-02 }
  - not-a-mapping
stale_after: 2099-01-01
related: ["../features/trust.md"]
---

# Trust frontmatter shapes

A deliberately mixed `verified` list, exercising the §5.2/§5.3 rules okq relies
on. Expected reading:

- The tier is **human-reviewed** — one `human:` verifier is enough, and the
  non-human and malformed entries neither block nor downgrade it.
- `by: ""` is an empty id, so it is *not* a human actor (the `human:` prefix
  test requires a non-empty id) and does not by itself lift the tier.
- The bare string `not-a-mapping` is skipped, not fatal.
- `status: percolating` is outside the three values §5.4 names, so it must be
  reported **verbatim** rather than coerced to `stable`.
- `stale_after: 2099-01-01` is far future, so this concept is never stale for
  any realistic `--today` — it keeps the fixture deterministic.

See [trust-malformed.md](trust-malformed.md) for the degrade-to-default cases
and [../features/trust.md](../features/trust.md) for the feature.
