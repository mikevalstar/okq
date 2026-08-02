---
type: fixture
title: Trust frontmatter — malformed values degrade to defaults
status: active
tags: [tests, robustness, fixtures, trust]
generated: sometime last spring
verified: yesterday
stale_after: whenever
related: ["../features/trust.md", "trust-shapes.md"]
---

# Malformed trust frontmatter

Every trust key here holds a value of the wrong shape: `generated` and
`verified` are scalars where a mapping and a list belong, and `stale_after` is
not a date.

None of this is an error. §11 requires a consumer to keep reading, so the
concept loads, stays queryable, and reports the **defaults** — trust tier
`unverified`, not stale. What it must never do is panic, drop the concept, or
invent a value.

The counterpart, [trust-shapes.md](trust-shapes.md), covers the valid shapes.
