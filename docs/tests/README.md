---
type: fixture
title: Robustness fixtures — deliberately malformed docs
status: active
tags: [tests, robustness, fixtures]
related: ["../adrs/0002-library-stack.md"]
---

# Robustness fixtures

This folder holds **deliberately malformed and edge-case documents**. It exists
so the test suite can prove that okq (and the upstream `okf` loader) degrade
gracefully on real-world junk: a bad doc is *skipped* (collected into okf's
`parse_errors`), never a panic and never a failure of the whole bundle.

The Rust suite that drives these lives in [`tests/robustness.rs`](../../tests/robustness.rs).

## What's here

| File | What's wrong / edge case | Expected handling |
|------|--------------------------|-------------------|
| `unterminated-frontmatter.md` | `---` opened, never closed | parse error → skipped |
| `invalid-yaml-flow.md` | unterminated flow sequence `[a, b` | parse error → skipped |
| `tab-indentation.md` | tab-indented YAML (rejected) | parse error → skipped |
| `frontmatter-is-list.md` | frontmatter is a sequence, not a mapping | parse error → skipped |
| `frontmatter-is-scalar.md` | frontmatter is a bare scalar | parse error → skipped |
| `bad name!.md` | filename is not a valid concept-id segment | parse error → skipped |
| `empty.md` | zero bytes | valid concept, empty body |
| `no-frontmatter.md` | OKF-shaped: body only, no frontmatter | valid concept, no `type` |
| `only-frontmatter.md` | frontmatter, no body | valid concept, empty body |
| `tags-not-a-list.md` | `tags` is a scalar, not a sequence | valid; `tags` reads as empty |
| `duplicate-headings.md` | two identical headings | `get --section` → ambiguous (exit 5) |
| `unicode-emoji.md` | multibyte/emoji/RTL in headings & body | valid; section slicing stays char-safe |
| `headings-in-code-fence.md` | `#` lines inside a code fence | those are not sections |
| `deeply/nested/concept.md` | multi-segment concept id | valid, nested |

## Note on bundle pollution

Because these live *inside* `docs/`, they currently show up when you query the
whole tree (`okq --bundle docs find`). That's intentional for now — it proves
okq copes with a bundle that contains junk — but once okq grows ignore support
(`.okqignore` / the `ignore` crate, ADR-0002), this folder should be excluded
from ordinary queries.
