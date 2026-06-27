---
type: feature
title: okq get — expand one concept on demand
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, get, retrieval, json, sections, identity]
milestone: M1
command: "okq get"
related: ["../adrs/0002-library-stack.md", "../adrs/0001-documentation-first-okf-shaped.md", "../guides/design-overview.md"]
---

# okq get — expand one concept on demand

## Summary

`okq get <concept>` prints a single concept's frontmatter and/or body — optionally just one section — as human-readable text or `--json`. It is the **expand-on-demand** counterpart to the locations-only shortlists that `search`, `find`, and the graph commands return: those say *where*, `get` produces *what's there*.

## Motivation

The token-frugal contract (PLAN.md §3) says discovery commands return ranked `path:line` + a snippet, never full bodies — the caller (human or agent) then expands exactly what it chose. That expansion step needs a command, and that command is `get`. Without it, the only way to read a hit is to dump the whole file (the very thing the design fights) or hand-roll `sed`/`awk` line slicing.

`get` is also the **first feature built** ([per the M1 plan](../guides/design-overview.md)) because it's the smallest end-to-end slice — load → resolve → output — and so it's where four cross-cutting contracts every other command inherits get pinned down (see [Cross-cutting contracts](#cross-cutting-contracts-this-feature-ratifies)).

## Scope

### In scope

- Resolve **one** concept by its identity and print it.
- Selectors: whole concept (default), `--frontmatter` only, `--body` only, `--section <heading>` only.
- Human output (default) and `--json` (one structured document on stdout).
- `path:line` reporting for the concept and for the selected section.
- Works on a conformant OKF bundle **and** on an OKF-*shaped* tree (any Markdown-with-frontmatter file), per the format-tolerance principle.

### Out of scope

- **Ranking or search** — `get` takes an exact identity, it does not find (that's `search`/`find`).
- **Multiple concepts in one call** — single concept for v1 (see Open questions).
- **Graph traversal** — `get` reads one node; following links is `neighbors`/`backlinks`.
- **Mutation** — `get` is read-only; authoring is `new`/`init`.
- **Rendering** — emits source Markdown, not rendered HTML/ANSI prose.

## Behavior

### Concept resolution (identity)

`<concept>` is resolved, in order:

1. As a **concept ID** — the OKF canonical identity: the file path within the bundle with `.md` removed (`tables/users` → `tables/users.md`). This is the primary form.
2. As a **literal path** — the same with the `.md` suffix included (`tables/users.md`), and a leading `./` tolerated.
3. *(Open question, see below)* As a **frontmatter `id`** value, if the bundle uses explicit `id:` fields.
4. As a **partial path / bare concept name** — a path-segment-aligned suffix of a concept id that *uniquely* identifies one concept. `okq get 0001-documentation-first-okf-shaped` resolves to `adrs/0001-documentation-first-okf-shaped` when that name is unique in the bundle, so callers needn't remember the full path. *(Implemented; the shared resolver `model::resolve_concept` is used by `get` and the graph commands alike.)*

Resolution prefers the **most exact** match: an exact concept id (1) or `.md` path (2) always wins; only if those miss does okq fall back to a unique segment-aligned suffix (a single segment matches by name). Matching is on `/` boundaries, never arbitrary substrings (`get ser` does **not** match `tables/users`). If a partial matches more than one concept, it is an **ambiguous-resolution error** (exit 4) that lists the candidate ids; the caller disambiguates by adding more of the path.

Resolution is relative to `--bundle <dir>` (default: cwd). The **reserved files `index.md` and `log.md` are not concept-addressable** (OKF reserves them; they're generated/scoped artifacts, not concepts) — `get`-ting one is a not-found unless a future `--raw` opts in.

### Invocation & flags

```sh
okq get tables/users                 # frontmatter + full body (human)
okq get tables/users --json          # same, as one JSON document
okq get tables/users --frontmatter   # frontmatter only
okq get tables/users --body          # body only (no frontmatter)
okq get tables/users --section "Schema"   # just the "Schema" section
okq get tables/users --section schema --json
```

- **Selectors** (`--frontmatter`, `--body`, `--section`) are additive: if none are given, the default is **frontmatter + full body**. If any are given, only the requested parts are emitted. `--section` implies a body subset and may be combined with `--frontmatter`.
- **`--section <heading>`** matches a heading by its text, **case-insensitively**, and also accepts a **slugified** form (`"Open questions"` ↔ `open-questions`). A section spans from its heading to the next heading of the same or higher level. Ambiguous matches (same heading text twice) are an error that lists the candidates with their `path:line`; no match is a distinct not-found (see exit codes).
- **Global flags** (shared across okq): `--bundle <dir>`, `--json`, `--no-color`. `get` honors the agent-runnable contract — it is fully non-interactive and never prompts.

### Output

**Human (default):** the concept's source Markdown for the selected parts, preceded by a one-line `path:line` header so the location is always visible. Frontmatter is shown as-is (YAML). Color (heading/path emphasis) honors `--no-color`/`NO_COLOR`/non-TTY.

**`--json`:** exactly one JSON document on stdout (logs/errors to stderr), carrying a versioned schema tag. Shape (illustrative):

```json
{
  "schema": "okq.get/v1",
  "id": "tables/users",
  "path": "tables/users.md",
  "line": 1,
  "type": "table",
  "title": "Users table",
  "frontmatter": { "type": "table", "tags": ["pii"], "title": "Users table" },
  "sections": [
    { "heading": "Schema", "slug": "schema", "level": 2, "line": 12, "body": "..." }
  ]
}
```

- With `--frontmatter`, `sections`/`body` are omitted; with `--body`/`--section`, `frontmatter` is omitted; `--section` returns a single-element `sections` array.
- `id`/`type`/`title`/`path`/`line` form the **shared concept envelope** reused by every other command's shortlist records — `get` is where its schema is fixed (and `schemars`-derived).

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success — concept (and section, if requested) found and emitted |
| 2 | Usage error (missing/invalid args) — clap-level |
| 4 | Concept not found / not resolvable |
| 5 | `--section` heading not found (or ambiguous) within a resolved concept |

Codes 4/5 are distinct so a script/agent can tell "no such doc" from "doc exists, no such section." (This taxonomy is shared across commands — see Open questions / it likely graduates to an ADR.)

## Cross-cutting contracts this feature ratifies

These are settled here once and inherited by `find`, `search`, and the graph commands:

1. **Concept identity** — path-minus-`.md` is canonical; `.md` path form also accepted; frontmatter `id` is an open question.
2. **The `--json` concept envelope** — `id` / `type` / `title` / `path` / `line`, schema-tagged and `schemars`-locked.
3. **Section model & `path:line`** — heading-delimited sections via `pulldown-cmark` (ADR-0002), addressable by text or slug; this is the same chunking `search` indexes against.
4. **Exit-code taxonomy & non-interactive contract** — documented codes, `--json` on stdout / logs on stderr, never prompts.

## Acceptance criteria

- [ ] `okq get <id>` resolves by concept ID and by `.md` path form, relative to `--bundle`.
- [ ] Default output is frontmatter + full body with a `path:line` header.
- [ ] `--frontmatter`, `--body`, `--section` each emit exactly their subset; combos behave as specified.
- [ ] `--section` matches by case-insensitive heading text and by slug; ambiguous → exit 5 listing candidates; missing → exit 5.
- [ ] `--json` emits exactly one document on stdout with `schema: "okq.get/v1"` and the shared envelope; logs/errors go to stderr.
- [ ] Reserved `index.md`/`log.md` are not concept-addressable (exit 4).
- [ ] Concept not found → exit 4; section issues → exit 5; usage error → exit 2.
- [ ] Works on a non-OKF, OKF-shaped Markdown+frontmatter tree (this repo's own `docs/`), proving format-tolerance.
- [ ] Fully non-interactive; identical behavior on and off a TTY.
- [ ] Output contracts (human + JSON) are snapshot-tested (`insta`) so the schema is locked from day one.
- [x] A unique partial path / bare name resolves to its concept; a non-unique partial errors (exit 4) and lists candidates; exact id/path always takes precedence over a partial.

## Open questions

- **Frontmatter `id` resolution** — do we resolve `<concept>` against an explicit frontmatter `id:` in addition to the path? Ties to PLAN.md §8 "concept identity: file path vs. frontmatter id — support both?". Decide before `find`/`search` reuse the envelope.
- **Multiple concepts** — should `get a b c` (or stdin-piped ids) be supported so an agent expands a whole shortlist in one call? Strong ergonomic for the `search → get` loop; deferred for v1 but shape the schema (array-friendly) so it's additive later.
- **Exit-code taxonomy** — codes 0/2/4/5 here are shared across all commands; promote the taxonomy to its own ADR so each new command maps onto it rather than inventing codes.
- **Section addressing edge cases** — duplicate headings, headings inside fenced code blocks, frontmatter-only docs (no headings). Define precisely during build.
- **`--raw` escape hatch** — a future flag to `get` reserved/non-concept files (`index.md`, `log.md`) verbatim?
- ~~**Partial-resolution ambiguity code**~~ — **decided: exit `4`** with candidates listed (`AppError::ConceptAmbiguous`), per [ADR-0004](../adrs/0004-exit-code-taxonomy.md).

## Related

- [ADR-0002](../adrs/0002-library-stack.md) — `pulldown-cmark` sections, `okf` resolution, `schemars` JSON contract, `clap` surface that this feature builds on
- [ADR-0001](../adrs/0001-documentation-first-okf-shaped.md) — the OKF-shaped docs tree `get` is first dogfooded against
- [PLAN.md](../guides/design-overview.md) — §3 token-frugal output, §5 command surface (`get`, chunking), §7 M1, §8 identity & schema-versioning open questions
- Future: `find`, `search` feature specs — reuse the concept envelope and section model ratified here
