---
type: feature
title: Phantom vs broken links — deadlinks that understands Obsidian's unresolved notes
status: active # draft | accepted | active | deprecated
created: 2026-07-07
updated: 2026-07-07
tags: [deadlinks, obsidian, graph, wikilinks, health]
milestone: null
command: "okq deadlinks"
related: ["graph.md", "wikilinks.md", "aliases.md", "validate.md", "../adrs/0004-exit-code-taxonomy.md"]
---

# Phantom vs broken links — `deadlinks` that understands Obsidian's unresolved notes

## Summary

`okq deadlinks` distinguishes a **broken** link (a reference that *should*
resolve but doesn't — a rename left dangling, a bad path) from a **phantom** link
(a bare `[[Note]]` to a note that simply doesn't exist yet — normal and
intentional in Obsidian). By default `deadlinks` reports only **broken** links;
phantoms are available behind a flag. This keeps `deadlinks --check` a trustworthy
CI gate both on okq's own docs (where every link must resolve) and on Obsidian
vaults (where thousands of forward-referencing phantoms are healthy, not rot).

## Motivation

In okq's own docs bundle every link should resolve, so any dead link is a real
error — that is the model `deadlinks` was built for. Point the same command at a
personal Obsidian vault and it reports **thousands** (a real vault: 2264 of
which 942 unique targets have no file at all), because Obsidian authors routinely
write `[[Michael Clarke]]` *before* the note exists — a grey "unresolved" link
you click to create later. That is the norm, not decay.

Reporting every phantom as an error makes `deadlinks` unusable on a vault and
turns `--check` into a permanent false alarm. Obsidian itself keeps "unresolved
links" in a pane of their own, well away from anything error-shaped. okq should
draw the same line: surface the handful of genuinely broken references, and keep
phantoms as an opt-in, informational list.

## Scope

### In scope

- **Classify** each unresolved edge into a `kind`:
  - **broken** — a concrete in-bundle reference that fails: a `/`-bearing or
    explicit `.md`/relative wikilink target that forms a valid id but matches no
    concept (a rename/move left it dangling); an inline CommonMark link to a
    missing in-bundle file (okf `broken_links`); an unresolvable frontmatter
    relation. These are almost always real mistakes.
  - **phantom** — a bare wikilink name (`[[Michael Clarke]]`) that matches no
    concept **and no alias** (see [[aliases]]). In Obsidian this is a
    not-yet-created note. Normal.
- Add a `kind: "broken" | "phantom"` field to each `okq.deadlinks/v1` record.
- **Default `deadlinks` lists broken only.** `--phantoms` (or `--all`) includes
  phantoms; `--phantoms-only` lists just the phantoms (a useful "notes I've
  referenced but not written yet" report).
- **`--check` trips on broken only** by default; phantoms never fail CI unless
  explicitly requested.

### Out of scope

- **Creating** the phantom notes. Read-only; scaffolding a note from a phantom is
  a separate idea (`new`-from-phantom), noted below.
- **Changing wikilink edge creation.** Phantoms are still not edges (they point
  at nothing); this only classifies and reports the unresolved set.
- **Attachment embeds** (`![[image.png]]`). A missing image/PDF embed is arguably
  its own category, not "broken markdown" — see open questions.

## Behavior

- `okq deadlinks` — **broken** only; `okq.deadlinks/v1` records gain `kind`.
- `okq deadlinks --phantoms` — broken **and** phantom.
- `okq deadlinks --phantoms-only` — phantom only.
- `okq deadlinks --check` — exit **3** if any **broken** (ADR-0004); combined
  with `--phantoms`, the gate covers the requested set (documented explicitly).
- Ordering unchanged — `source_id` then `raw`, deterministic.
- **Backward-compatibility.** This *narrows* the default `deadlinks` result set
  and the default `--check` behavior — a behavior change to a shipped command,
  called out here per the "docs immutable at commit" convention. The schema gains
  an **additive** `kind` field (still `deadlinks/v1`); whether the default-set
  narrowing is breaking enough to warrant `deadlinks/v2` is an open question. On
  okq's own docs bundle nothing is a phantom (all bare names resolve), so its
  `deadlinks` / `--check` output is unchanged.

## Acceptance criteria

- [x] Every unresolved record carries `kind ∈ {broken, phantom}`; a `/`-path or
  `.md` miss is `broken`, a bare-name miss with no alias is `phantom`.
- [x] Default `deadlinks` and `--check` consider **broken** only;
  `--phantoms` / `--phantoms-only` toggle the reported set.
- [x] On okq's own docs (all links resolve) output is unchanged; on a
  vault-shaped fixture holding both kinds, each is classified correctly.
- [x] An alias-resolved target is **neither** broken nor phantom (it resolves) —
  contingent on [[aliases]]; until that lands, an alias target is a phantom.
- [x] Deterministic ordering; malformed input never panics; `insta` snapshots +
  a vault-shaped fixture.

## Open questions

- **Vocabulary.** `broken` / `phantom` vs okf's existing `broken_links`. Pick
  terms and use them consistently in output and docs.
- **Flag surface. → Resolved:** `--phantoms` (broken + phantom) and
  `--phantoms-only` (just phantoms); no `--all`.
- **Attachment embeds** as a third `kind: "attachment"`, and whether to resolve
  them against the vault's attachment folder (Obsidian records the attachment
  path in `.obsidian/`). Likely its own small feature.
- **`stats` surface. → Resolved:** `stats.dead_links` now counts **broken** only;
  a new `phantom_links` field and a `Phantom links:` column carry the phantoms
  (touches [stats](stats.md)).
- **Schema version. → Resolved:** `kind` is additive, so `deadlinks/v1` and
  `stats/v1` stand; only the *default result set* of `deadlinks` narrows (a
  documented behavior change, not a schema break).
- **Sequencing.** Land [[aliases]] first (or together) so alias targets aren't
  mislabeled phantom then reclassified.

## Related

- [graph](graph.md) — `deadlinks` is defined there; this spec refines its
  classification and default behavior.
- [wikilinks](wikilinks.md) — phantoms originate from bare wikilinks that resolve
  to nothing; this spec reinterprets that outcome for Obsidian vaults.
- [aliases](aliases.md) — alias resolution removes false phantoms; sequence it
  first.
- [validate](validate.md) — reports *unparseable* docs; phantom/broken is a
  different health axis (link targets, not frontmatter conformance).
- [ADR-0004 — exit-code taxonomy](../adrs/0004-exit-code-taxonomy.md) — the
  `--check` → exit 3 contract this preserves for broken links.
