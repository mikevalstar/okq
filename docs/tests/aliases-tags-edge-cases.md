---
type: fixture
title: Edge-case aliases and inline tags
status: active
tags: [tests, robustness, fixtures, aliases]
aliases:
  - Edge Alias
  - ""
related: ["../features/aliases.md", "../features/inline-tags.md"]
---

# Edge-case aliases and inline tags

The alias reader and the inline-tag scanner must degrade gracefully on all of
these — read what they can, never panic. The bundle must still load and query.

Inline tags that should be picked up: #fixture-tag and a nested #area/robustness.

Non-tags that must be ignored: a bare number #123, a URL fragment
https://example.com/#section, a heading marker, and mid-word foo#bar.

A `#not-a-tag-in-code` span is content, not a tag.

```
#also-not-a-tag-in-a-fence
```
