---
type: feature
title: okq index
status: active # draft | accepted | active | deprecated
created: 2026-06-27
updated: 2026-06-27
tags: [cli, authoring, scaffold, index]
milestone: null # milestones retired — see CHANGELOG.md
command: okq index
related:
  - ./scaffold.md
  - ./stats.md
  - ../adrs/0001-documentation-first-okf-shaped.md
  - ../guides/design-overview.md
---

# okq index

## Summary

`okq index` regenerates the directory-listing `index.md` files in a bundle — the
human-and-agent-readable "what's in this folder" tables — from the concepts okq
already knows about, keeping them current as docs are added, renamed, or removed.

## Motivation

`okq init` seeds `index.md` files and `okq new` adds concepts, but nothing keeps
the listings in sync: the moment you add a second ADR, the `adrs/index.md` table
is stale. Today that means hand-editing a listing every time you author a doc —
exactly the repeatable, mechanical chore okq should own. This completes the
authoring loop: **`init` creates → `new` adds → `index` maintains.** OKF treats
`index.md` as *generated, not source* ([ADR-0001](../adrs/0001-documentation-first-okf-shaped.md)),
so okq, which already has the full concept list, is the natural generator.

## Scope

### In scope

- Regenerate the listing in each directory's `index.md` (root and subdirectories)
  from the bundle's concepts: a table/list of `id` + `title` (+ `type` where it
  helps), in deterministic order.
- Preserve human-authored content: manage only a **fenced listing block**
  (`<!-- okq:index:begin -->` / `<!-- okq:index:end -->`, an index-specific
  variant of the marker approach `init` uses for the README), leaving surrounding
  prose and the root `index.md`'s `okf_version` frontmatter untouched.
- Create an `index.md` where one is missing (so a new subdirectory of concepts
  gets a listing).
- Idempotent: re-running rewrites only when the generated block actually changed;
  report per-file `created` / `updated` / `unchanged`.

### Out of scope

- Rewriting concept documents themselves — `index` only touches `index.md`
  listing files.
- Inventing folder structure or moving docs; it lists what exists.
- A bundle-wide table of contents in one file (the per-directory `index.md` is the
  OKF convention; revisit if asked).
- `log.md` (reserved, but not a directory listing).

## Behavior

### Invocation & flags

```sh
okq index                       # regenerate all index.md listings in the bundle
okq --bundle docs index         # ...in a specific bundle
okq index --check               # don't write; exit 3 if any listing is out of date (CI)
```

- Honors `--bundle`, `--no-ignore` (ignored files don't appear in listings),
  `--json`.
- `--check` makes it a read-only verifier (for CI / pre-commit): it writes
  nothing and exits non-zero when a listing would change — so a stale `index.md`
  can't land.

### Output

- Human: a per-file report to **stderr** (`created` / `updated` / `unchanged`),
  like `okq init`.
- `--json`: one `okq.index/v1` document on stdout — the files touched, each with
  its verb and the concept count it now lists.
- The generated listing is a Markdown table or list, deterministic
  (concept-id order), inside the marker block; the rest of the file is preserved
  byte-for-byte.

### Exit codes

Shared taxonomy ([ADR-0004](../adrs/0004-exit-code-taxonomy.md)):

- `0` — listings regenerated (or already current).
- `1` — I/O failure (unwritable bundle, etc.).
- `2` — usage error.
- `3` — `--check` found an out-of-date listing (nothing written).

## Acceptance criteria

- [ ] `okq index` writes a deterministic listing block into each directory's
      `index.md`, preserving surrounding prose and root `okf_version` frontmatter.
- [ ] Re-running with no doc changes reports `unchanged` and rewrites nothing.
- [ ] Adding a concept then running `index` adds it to the right listing.
- [ ] `--check` writes nothing and exits 3 when a listing is stale, 0 when current.
- [ ] Ignored files (`.okqignore`) are absent from listings unless `--no-ignore`.
- [ ] `--json` emits the documented `okq.index/v1` envelope.
- [ ] Generated `index.md` files stay OKF-conformant (reserved; no spurious
      `type`) and don't introduce dead links or parse errors.

## Resolved during implementation

- **Listing shape:** a `| Title | File |` table of relative links for the
  directory's concepts, preceded by a `### Folders` bullet list of immediate
  subdirectory links. (Was: table vs. bullet list.)
- **Nesting depth:** each `index.md` lists its **direct** concept children plus
  links to immediate subdirectories that contain concepts — a navigable tree, not
  a recursive flatten. (Was: direct children vs. recurse.)
- **Marker adoption:** a marker-less `index.md` gets the block **appended** below
  its existing content on first run (no `--force` needed); subsequent runs rewrite
  only between the markers. (Was: wrap vs. require a flag.)

## Open questions

- None outstanding.

## Related

- [scaffold.md](./scaffold.md) — `init`/`new`; `index` is the third authoring command that completes the loop
- [stats.md](./stats.md) — the read-only overview; `index` writes the per-directory listings
- [ADR-0001](../adrs/0001-documentation-first-okf-shaped.md) — `index.md` is generated, not source
- [design-overview.md](../guides/design-overview.md) — where this sits in the command surface
