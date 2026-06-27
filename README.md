# okq

A fast, local command-line tool for searching and navigating [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf) (OKF) bundles: directories of Markdown files with YAML frontmatter, cross-linked into a knowledge graph.

> Beta. Every command is implemented and tested. The `--json` shapes and exit codes are stable enough to script against. See [PLAN.md](PLAN.md) for what's next.

okq runs full-text search, frontmatter queries, and graph navigation over a tree of docs. It is deterministic and local: no embeddings, no network, no API key, same answer every time. The same command works whether a person runs it or a program does, because every command has a `--json` mode and documented exit codes.

## What's an OKF bundle?

OKF is a small, vendor-neutral convention: one concept per Markdown file, a handful of well-known YAML frontmatter keys (`type`, `title`, `tags`, …), and links between files. ADRs, design docs, runbooks, and wikis all fit it. okq works on any OKF bundle, and degrades gracefully on any Markdown-with-frontmatter tree.

## Install

```sh
cargo install okq
```

Or with [mise](https://mise.jdx.dev):

```sh
mise use -g cargo:okq
```

## Usage

Point okq at a bundle with `--bundle` (default: the current directory).

```sh
# full-text search, ranked by relevance
okq search "retrieval latency"

# filter by frontmatter
okq find --type adr --tag security

# read one concept, or a single section of it
okq get adrs/0002-library-stack --section Decision

# follow the links
okq neighbors adrs/0002-library-stack
okq backlinks features/search
okq path features/search features/get

# health and overview
okq deadlinks
okq orphans
okq stats
```

You don't need the full path. A unique suffix is enough: `okq get 0002-library-stack`.

## Scripting and agents

Every command takes `--json` and writes one JSON document to stdout; messages and progress go to stderr. Exit codes are documented (below), so a script can branch on `$?` without parsing output. That makes okq a clean tool call for an LLM assembling context: it returns the relevant locations instead of dumping whole files.

```sh
okq search auth --json | jq -r '.results[].path'
okq find --tag security --json | jq '.count'
okq deadlinks --check        # exit 3 if any dead links, for CI
```

`okq schema <command>` prints the JSON Schema for that command's output, generated from the code, so you can validate against it.

## Agent skills

The [`skills/`](skills/) directory ships [Agent Skills](https://agentskills.io) that teach an AI agent to use okq well — so adopting okq also onboards the agents that work in your bundles. They follow the open `SKILL.md` standard and work in Claude Code and other compatible agents.

| Skill | What it does |
|-------|--------------|
| `okq-explore` | Search and navigate a bundle to assemble context before work. |
| `okq-write-okf` | Author OKF docs from okq's templates, with cross-links and verification. |
| `okq-maintain` | Find and fix bundle rot; audit a doc against the code. |
| `okq-reference` | The okq CLI contract — loaded automatically as background knowledge. |

Install all four with [skills.sh](https://www.skills.sh):

```sh
npx skills add mikevalstar/okq
```

Or install them by hand — copy (or symlink) the skill folders into your agent's skills directory:

```sh
# Claude Code: personal (all projects) or project-local
cp -r skills/okq-* ~/.claude/skills/        # personal
cp -r skills/okq-* .claude/skills/          # this project only
```

Then invoke one with `/okq-explore`, `/okq-write-okf`, or `/okq-maintain`; `okq-reference` loads on its own when okq or OKF comes up.

## Commands

| Command | What it does |
|---------|--------------|
| `okq search <query>` | Ranked full-text search over section text (BM25). |
| `okq find` | Filter concepts by `--tag`, `--type`, `--where field=value`, `--match` (`--regex`). |
| `okq get <concept>` | Print a concept's frontmatter and/or body, or one `--section`. |
| `okq neighbors <concept>` | Adjacent concepts via the link graph (`--depth`, `--direction`, `--edge`). |
| `okq backlinks <concept>` | Concepts that link to this one. |
| `okq path <a> <b>` | Shortest link path between two concepts (`--undirected`). |
| `okq orphans` | Concepts with no inbound links (`--check` for CI). |
| `okq deadlinks` | Links pointing at missing concepts (`--check` for CI). |
| `okq stats` | Counts by type and tag, link density, edge types, hubs. |
| `okq schema [<cmd>]` | JSON Schema for a command's `--json` output. |
| `okq init` | Scaffold a new bundle: `adrs/` + `features/`, a seed ADR, a README. |
| `okq new <type> [title]` | Add one concept from a template (`adr` numbers itself, `feature` slugifies). |

Run `okq <command> --help` for flags and examples.

## Starting a bundle

```sh
okq init                       # scaffold an OKF bundle in the current directory
okq new adr "Adopt Tantivy"    # add a numbered ADR from a template
```

`init` is non-destructive: it creates only the files that are missing, and adds its section to an existing README between markers rather than overwriting it.

## Ignoring files

Not every Markdown file in the tree is a concept. Drop a `.okqignore` file in the bundle (full `.gitignore` syntax — comments, `!` negation, anchoring, `**`, and per-directory nesting) and matching files drop out of every command:

```gitignore
# fixtures and scratch notes aren't real concepts
tests/
drafts/
!drafts/keep.md
```

Ignored files are treated as if they weren't in the bundle: they don't show up in `search`, `find`, `stats`, or `orphans`; `get` on one reports "not found"; and a link pointing at one becomes a dead link. Pass `--no-ignore` on any command to query the full tree.

## How it works

- Search uses a BM25 index (Tantivy), cached per-bundle under your XDG cache directory and rebuilt when files change. okq never writes into the bundle itself.
- The graph is built from inline Markdown links and from frontmatter relations (`related`, `supersedes`, `depends-on`, …).
- Results are locations, not document dumps: ranked `path:line` plus a short snippet. You expand what you want with `get`.
- Parsing and the data model come from the [`okf`](https://crates.io/crates/okf) crate; okq adds the query and navigation layer on top.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success, including an empty result (a query that found nothing still ran). |
| 1 | Internal error: bad bundle, I/O, or index failure. |
| 2 | Usage error: bad flags, a malformed `--where`, or an invalid query. |
| 3 | A `--check` run found issues (`orphans`/`deadlinks`). |
| 4 | Concept not found. |
| 5 | Section not found or ambiguous. |

## License

[Apache-2.0](LICENSE), matching OKF and the `okf` crate.
