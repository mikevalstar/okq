---
name: okq-reference
description: okq CLI contract — commands, --json output, and exit codes for querying OKF bundles, Obsidian vaults, or any Markdown-with-frontmatter collection. Background reference, loaded whenever okq or OKF is in use.
user-invocable: false
allowed-tools: Bash
---

`okq` is a fast, deterministic, local-first CLI for querying OKF bundles, Obsidian
vaults, or any Markdown-with-frontmatter collection. No network, no embeddings, no
API key — same bundle, same answer, every time. Use it instead of `grep`/`rg`/`yq`
when working in a Markdown-with-frontmatter knowledge base.

## Bundle basics

- A **bundle** is a directory of Markdown files, each with YAML frontmatter; one
  **concept** per file. `--bundle <dir>` selects it (default: current directory).
- A **concept id** is the path without `.md`: `adrs/0004-exit-code-taxonomy`.
  Partial ids resolve if unambiguous. `get` also accepts the `.md` path.
- `index.md` / `log.md` are reserved and are not concepts.
- Malformed files are skipped, not fatal — they never crash a query.

## Commands

| Command | What it does |
|---|---|
| `search <query>` | Rank sections by relevance (BM25); returns the most authoritative hit. `"quoted"` = phrase — quote a multi-word query or a keyword-dense note can outrank the real match. `--limit N`. |
| `find` | Filter concepts by exact predicate: `--tag`, `--type`, `--where field=value`, `--match <text>` (literal substring, every match unranked; `--regex` to treat as regex), `--status`, `--trust`, `--stale`. Repeatable flags AND (tags/where) or OR (type/status/trust). |
| `get <concept>` | Expand one concept. `--section <heading>`, `--field <key>`, `--frontmatter`, `--body`. |
| `neighbors <concept>` | Adjacent concepts via the link graph. `--depth N`, `--direction in\|out\|both`, `--edge <type>`. |
| `backlinks <concept>` | Concepts that link *to* this one (graph edges only — use `find --match` for plain-text mentions). |
| `path <from> <to>` | Shortest link path between two concepts. `--undirected`. |
| `orphans` | Concepts with no inbound links (stale-doc candidates). `--check`. |
| `deadlinks` | Links pointing at missing/renamed concepts. `--check`. |
| `stats` | Bundle overview: counts, distributions (type, tag, trust, status), link density, hubs. `--top N`. |
| `validate` | OKF conformance: unparseable, untyped, or malformed docs. `--check`, `--severity`. |
| `lint` | Bundle hygiene beyond conformance, 16 coded rules (L1–L16). `--check`, `--rule`, `--ignore`, `--today`. Never an error, never affects conformance. |
| `schema <command>` | JSON Schema for a command's `--json` output (the agent contract). |
| `spec` | Print the OKF specification this build implements (v0.2, verbatim, embedded — works offline, no bundle needed). `--section <heading>`. |
| `new <type> [title]` | Create one concept from a template (`adr` \| `feature`). `--list`. |
| `init` | Scaffold a new OKF bundle (idempotent). |

## Trust & lifecycle

Concept records carry `status` (draft/stable/deprecated), `trust`
(unverified/machine-confirmed/human-reviewed, **derived** from `verified`), and
`stale`. Each is **omitted when at its default** — absent `status` means
`stable`, absent `trust` means `unverified`, absent `stale` means not stale. Do
not read absence as unknown.

Prefer verified, non-stale sources when assembling context:
`okq find --trust human-reviewed`. `get` shows the `generated`/`verified` events
behind the tier.

## Output discipline

- **stdout = data.** With `--json`, exactly one JSON document on stdout. Human
  notes, warnings, and "no results" go to stderr.
- **Token-frugal.** Results are ranked `path:line` + frontmatter + a short
  snippet, never full bodies. Read those first; expand on demand with
  `get <id> --section <heading>`. Do not dump whole files into context.
- Every command has a non-interactive path; nothing prompts.
- `--json` is for programs. When *you* are the reader, use the human `path:line`
  output — it's several times smaller. Reserve `--json` for piping to a consumer:
  `okq search "auth" --json | jq -r '.results[].path'`.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success (zero results is success, not an error) |
| 1 | other (bad bundle, I/O, index) |
| 2 | usage (bad flag, invalid regex/query) |
| 3 | `--check` gate tripped (orphans/deadlinks/lint/validate) — for CI |
| 4 | concept not found / not resolvable |
| 5 | section not found / ambiguous |

Branch on `$?` rather than parsing text.
