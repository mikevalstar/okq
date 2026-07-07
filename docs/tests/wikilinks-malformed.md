---
type: fixture
title: Malformed and edge-case wikilinks
status: active
tags: [tests, robustness, fixtures, wikilinks]
related: ["../features/wikilinks.md"]
---

# Malformed and edge-case wikilinks

The wikilink scanner must degrade gracefully on all of these — extract what it
can, never panic. None of these resolve to a real concept, so `deadlinks` may
list the bare-name ones, but the bundle must still load and query.

- Unterminated open: [[dangling with no close
- Empty target: [[]] and whitespace-only [[   ]]
- Same-note anchors (no edge): [[#heading]] and [[#^block-42]]
- Adjacent, no space: [[alpha]][[beta]]
- Alias and heading and block: [[gamma#Section|shown]] and [[delta#^b-1]]
- Embed / transclusion: ![[epsilon]]
- Path-ish and `.md`: [[some/where/note.md]]
- External-looking (skipped): [[https://example.com|site]]
- Nested brackets: [[outer [[inner]] ]]

A wikilink inside inline code `[[NotAScannedLink]]` is content, not a link.

```
[[AlsoIgnoredInFence]]
```
