---
name: okq-explore
description: Search and navigate an OKF bundle, Obsidian vault, or any Markdown-with-frontmatter collection with okq instead of grep. Use to find related docs, read a section, or see what links to what before starting work, or when the user mentions okq/OKF.
allowed-tools: Bash
---

Map the territory before you touch it. Use `okq` (not `grep`/`rg`) to find the
relevant concepts, read only what you need, and follow the link graph. See the
`okq-reference` skill for the full command/flag contract.

## The loop

1. **Orient** (new or unfamiliar collection — an OKF bundle, an Obsidian vault, or
   any directory of Markdown-with-frontmatter):
   `okq --bundle <dir> stats` — counts, types, tags, hub concepts. On a messy vault
   it also reports phantom links and orphans, so you know how far to trust the graph.

2. **Find candidates** — pick the tool by the question:
   - Ranked, fuzzy: `okq search "<topic>"` (BM25) — the most *authoritative*
     sections. **Quote a multi-word query**: bare `search static site generator` is
     OR-matched and term-frequency-ranked, so a keyword-dense unrelated note can top
     the list — `search "static site"` or one distinctive term fixes it.
   - Exact literal, full recall: `okq find --match "<text>"` — *every* note that
     contains the string, unranked. Use when you know the words and want the whole
     set, not the single top hit.
   - Exact predicate: `okq find --type adr --tag security --where status=active`.
   All return `path:line` — `search` adds a match snippet, `find` lists the concept
   title. Read those first, then `get` the ones worth expanding.

3. **Read on demand** — never the whole file:
   - One section: `okq get <id> --section "<heading>"`
   - Just metadata: `okq get <id> --frontmatter`

4. **Follow the graph** to find what `search` missed by vocabulary:
   - `okq neighbors <id>` (add `--depth 2`, `--direction in|out`, `--edge related`)
   - `okq backlinks <id>` — who depends on / references this (graph edges only;
     use `find --match` to also catch plain-text mentions that aren't links)
   - `okq path <a> <b>` — how two concepts connect

## Rollup a topic across many notes

To assemble what a *set* of notes says about something — a weekly project report,
everything that touched an ADR, every mention of a person — trace the graph, don't
search:

1. `okq backlinks "<X>"` (graph edges) or `okq find --match "<X>"` (literal
   mentions) — get the candidate list, `path:line` only.
2. **Filter that list in-context** by date, path, or predicate. It's just paths, so
   this step is nearly free.
3. `okq get <note> --section "<heading>"` on each survivor — pull only the relevant
   slice, never whole files.
4. Synthesize from those slices.

`search` ranks one authoritative hit; it can't tell you what a *whole set* of notes
says about X. That's a graph slice — start from `backlinks`/`find`, not `search`.

## Rules

- Token-frugal: collect `path:line` + snippet, expand sections only as needed.
  Don't paste full documents into context.
- `search` for vocabulary you can guess; `find` for exact frontmatter or literal
  text; the graph for relationships. Use all three rather than over-searching one.
- Zero results is a valid answer (exit 0), not a failure — report it and adjust
  terms or widen the predicate.
- `--json` is for programs, not for reading. Eyeball the human `path:line` list to
  filter — it's several times smaller than the JSON. Reach for `--json` only when
  you pipe to a consumer: `okq find --type feature --json | jq -r '.concepts[].id'`.

## Hand-off

When the goal is to *write* a doc, switch to `okq-write-okf`; to *fix* link or
status health, switch to `okq-maintain`.
