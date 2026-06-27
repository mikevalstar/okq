---
name: okq-explore
description: Search and navigate an OKF doc bundle with okq instead of grep. Use to find related docs, read a section, or see what links to what before starting work, or when the user mentions okq/OKF.
allowed-tools: Bash
---

Map the territory before you touch it. Use `okq` (not `grep`/`rg`) to find the
relevant concepts, read only what you need, and follow the link graph. See the
`okq-reference` skill for the full command/flag contract.

## The loop

1. **Orient** (new or unfamiliar bundle):
   `okq --bundle <dir> stats` — counts, types, tags, hub concepts.

2. **Find candidates** — two ways, often both:
   - Ranked, fuzzy: `okq search "<topic>"` (BM25; `"quote"` for a phrase, `--limit N`).
   - Exact predicate: `okq find --type adr --tag security --where status=active`.
   Both return `path:line` + a snippet. Read those first.

3. **Read on demand** — never the whole file:
   - One section: `okq get <id> --section "<heading>"`
   - Just metadata: `okq get <id> --frontmatter`

4. **Follow the graph** to find what `search` missed by vocabulary:
   - `okq neighbors <id>` (add `--depth 2`, `--direction in|out`, `--edge related`)
   - `okq backlinks <id>` — who depends on / references this
   - `okq path <a> <b>` — how two concepts connect

## Rules

- Token-frugal: collect `path:line` + snippet, expand sections only as needed.
  Don't paste full documents into context.
- `search` for vocabulary you can guess; `find` for exact frontmatter; the graph
  for relationships. Use all three rather than over-searching one.
- Zero results is a valid answer (exit 0), not a failure — report it and adjust
  terms or widen the predicate.
- Scripting: add `--json` and pipe to `jq`
  (`okq find --type feature --json | jq -r '.concepts[].id'`).

## Hand-off

When the goal is to *write* a doc, switch to `okq-write-okf`; to *fix* link or
status health, switch to `okq-maintain`.
