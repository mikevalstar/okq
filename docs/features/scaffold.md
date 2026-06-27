---
type: feature
title: okq init & new — scaffold and author OKF bundles
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, init, new, scaffold, authoring, okf, templates]
milestone: M3.5
command: "okq init | okq new"
related: ["get.md", "stats.md", "../adrs/0001-documentation-first-okf-shaped.md", "../adrs/0004-exit-code-taxonomy.md", "../guides/design-overview.md"]
---

# okq init & new — scaffold and author OKF bundles

## Summary

`okq init` scaffolds a minimal, **OKF v0.1-conformant** bundle in a directory;
`okq new <type> [title]` adds one concept from an embedded template with
frontmatter pre-filled. Together they close the loop — okq can now *create* the
bundles it queries — and they lower the adoption barrier by answering "what does
a conformant bundle even look like?" in one command.

## Motivation

Querying is the point, but a bundle has to exist first, and the chicken-and-egg of
"how do I lay one out conformantly?" is a real adoption barrier (PLAN.md §5). `init`
gets a repo from zero to a queryable, [OKF-conformant](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)
skeleton; `new` makes adding the next doc a one-liner instead of a copy-paste of
remembered frontmatter. The same tool that scaffolds also searches and
navigates — that integration is okq's angle (not novelty).

## Design stance

- **Follow the Google OKF spec closely.** Frontmatter uses OKF's well-known keys;
  reserved files (`index.md`, `log.md`) and concept-identity rules are honored;
  the root marks `okf_version`. We don't invent conventions the spec doesn't have.
- **Minimal and unopinionated.** `init` scaffolds **`adrs/` and `features/` only** —
  broadly useful for software projects without pushing a fuller personal taxonomy
  (guides/workflows/etc.) on adopters. More folders are a `new`/manual step, not a
  default.
- **Non-destructive.** `init` never overwrites an existing concept; it creates only
  what's missing and *injects* into an existing README rather than clobbering it.

## Behavior — `okq init`

Scaffolds into `--bundle <dir>` (default: cwd), creating only absent files:

```
<bundle>/
├── README.md          # base README, or okq section injected into an existing one
├── index.md           # root listing + `okf_version: "0.1"` (OKF §6/§11)
├── adrs/
│   ├── index.md        # "Architecture Decision Records" directory listing
│   └── 0001-record-architecture-decisions.md   # canonical seed ADR
└── features/
    └── index.md        # "Features" directory listing
```

Each piece earns its place:
- **`adrs/` and `features/`** — the two folders, plus the README handling below.
- **Root `index.md`** carries `okf_version: "0.1"` (the spec's bundle-version
  marker, §11) and doubles as the root directory listing (§6).
- **Per-folder `index.md`** are real OKF directory listings — and keep the dirs
  non-empty so git tracks them.
- **Seed `adrs/0001-record-architecture-decisions.md`** — Michael Nygard's
  canonical first ADR. It demonstrates the OKF frontmatter, makes the bundle
  immediately queryable, and seeds the ADR practice without being opinionated.

### README handling (treat the base as something you modify)

Because `README.md` isn't a reserved OKF filename, it *is* a concept — so for the
bundle to stay conformant it needs a `type`. `init` ensures the README carries
`type: readme` frontmatter (adding a minimal block if absent), then:

- **No README yet** → write a **base README**: `type: readme` frontmatter, a short
  intro, plus a "Managed with okq" section (what the bundle is, and example `okq`
  commands to query it).
- **README exists** → ensure its `type: readme` frontmatter, then **inject** the
  okq section between markers, idempotently:

  ```markdown
  <!-- okq:begin -->
  ## Knowledge base

  This directory is an Open Knowledge Format (OKF) bundle. Explore it with okq:

      okq search "<topic>"     # ranked full-text
      okq find --tag <tag>     # filter by frontmatter
      okq stats                # overview
      okq new adr "<title>"    # add a decision

  <!-- okq:end -->
  ```

  Re-running `init` replaces the block between the markers (never touches the rest);
  if the markers are absent, the block is appended. This is the "we took the base
  and modified it" model — okq owns only its fenced section.

## Behavior — `okq new <type> [title]`

Creates one concept from the embedded template for `<type>`:

```sh
okq new adr "Adopt Tantivy for search"   # -> adrs/0002-adopt-tantivy-for-search.md
okq new feature "Saved searches"         # -> features/saved-searches.md
okq new adr                              # title omitted -> prompts? no — see below
```

- **Types** ship embedded: `adr` and `feature` (matching what `init` scaffolds);
  the set is extensible. `okq new --list` shows available types.
- **Placement & naming:** `adr` → `adrs/NNNN-<slug>.md`, **auto-numbered** from the
  existing highest ADR; `feature` → `features/<slug>.md`. `<slug>` is the slugified
  title.
- **Frontmatter pre-filled** to OKF conventions: `type`, `title`, `description`
  (placeholder), `tags: []`, `timestamp` (today, ISO-8601), plus a body skeleton
  appropriate to the type (ADR: Status/Context/Decision/Consequences; feature:
  Summary/Motivation/Scope/…).
- Prints the path of the created file to stdout (so it's pipeable: `$(okq new …)`).
- **Non-interactive** (agent-runnable): a missing required title is a usage error
  naming the flag, never a prompt.

### Templates

Embedded in the binary (`include_str!`/`rust-embed`, ADR-0002) — no template-dir
bootstrap. They degrade to OKF v0.1 defaults. A bundle-local `.okq/templates/`
override is a deferred open question (PLAN.md §8), as is how `new`'s templates stay
in lockstep with the OKF version `init` targets.

## Exit codes (ADR-0004)

| Code | Meaning |
|------|---------|
| 0 | Scaffolded / created successfully (incl. `init` that only injected a README block) |
| 2 | Usage error: unknown `new` type, missing required title, bad flags |
| 1 | I/O failure, or a target file already exists and would be overwritten |

`init` is idempotent: re-running on an already-scaffolded bundle succeeds (exit 0),
creating only what's missing and refreshing the README block.

## Acceptance criteria

- [ ] `okq init` in an empty dir creates `adrs/` and `features/` and a base README,
  all OKF-conformant; the result is immediately queryable (`okq stats` works).
- [ ] `okq init` in a dir with an existing README injects the okq block between
  markers without altering the rest; re-running is idempotent.
- [ ] `okq init` never overwrites an existing concept.
- [ ] `okq new adr "<title>"` creates an auto-numbered `adrs/NNNN-<slug>.md` with
  OKF frontmatter (`type: adr`, title, timestamp) and a body skeleton; prints the path.
- [ ] `okq new feature "<title>"` creates `features/<slug>.md` similarly.
- [ ] `okq new <unknown-type>` → exit 2 listing known types; missing title → exit 2.
- [ ] Created docs pass okq's own load (no parse errors) and appear in `find`/`search`.
- [ ] Fully non-interactive; templates embedded (no external files needed).

## Open questions

- **index.md generation** — `init` seeds `index.md` listings, but they go stale as
  docs are added. A dedicated `okq index` (generate/synthesize OKF directory
  listings) is the natural companion — in M3.5 or later? (Distinct from `okq schema`.)
- **`timestamp` source** — `new` fills today's date; needs a date source (a small
  `time`/`jiff` dep, or compute from `SystemTime`). Authoring may use the clock —
  the determinism principle binds queries, not writes.
- **Template override** — embedded-only for v1; when/how to honor `.okq/templates/`,
  and keeping `new`'s output in lockstep with the OKF version (PLAN.md §8).
- **Type→folder mapping** — fixed (`adr`→`adrs/`, `feature`→`features/`) vs.
  configurable; what happens when `new feature` runs in a bundle `init` never touched.
- **README markers** — `<!-- okq:begin/end -->` (chosen); bikeshed the exact text.

## Related

- [ADR-0001](../adrs/0001-documentation-first-okf-shaped.md) — our own OKF-shaped docs tree, the dogfood model for what `init` produces
- [get](get.md) / [stats](stats.md) — what a scaffolded bundle is immediately queryable with
- [OKF spec](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) — the conformance target for layout, frontmatter, and reserved files
- [PLAN.md](../guides/design-overview.md) — §5 `init`/`new`, §7 M3.5, §8 template-override & index.md open questions
