---
name: okq-write-okf
description: Author or edit an OKF document (ADR, feature spec, runbook, wiki page). Use when writing docs in an OKF bundle — covers okq's templates, frontmatter, cross-links, and verifying the result.
allowed-tools: Bash, Read, Write, Edit
---

Write documents that `okq` can parse, link, and rank. Explore first
(`okq-explore`) so you cross-link to what already exists instead of duplicating
it. See `okq-reference` for the command contract.

## Start from a template — don't hand-roll frontmatter

`okq new` stamps a correctly-shaped file (frontmatter + section skeleton + today's
date) and prints its path. Prefer it over writing frontmatter from memory.

```sh
okq new --list                      # show available types
okq new adr "Adopt Tantivy for search"   # auto-numbered ADR in adrs/
okq new feature "Saved searches"         # feature spec in features/
$EDITOR "$(okq new feature 'Saved searches')"   # create then open
```

`okq new` emits the **canonical OKF well-known keys** —
`type`, `title`, `description`, `tags`, `timestamp` — and a body skeleton:
ADRs get Status / Context / Decision / Consequences; features get Summary /
Motivation / Behavior / Acceptance criteria / Open questions. Fill the skeleton;
keep the headings.

**Check for a bundle-local template first.** Many bundles keep a richer
`_template.md` (e.g. `adrs/_template.md`, `features/_template.md`) with extra
frontmatter keys and section conventions of their own — this repo, for instance,
adds `status` (draft → accepted → active → deprecated), `created`, `updated`,
`milestone`, and `related`. If a `_template.md` exists, copy and follow *it*
rather than the bare `okq new` output, so the new doc matches the bundle. When in
doubt, read an existing accepted doc of the same type as your model.

## Frontmatter rules

- `type` is required for a file to count as a concept (drives `find --type`).
- `title` is the display name; `description` is the one-line summary `find`/`search` show.
- `tags` are how docs are grouped — reuse existing tags (`okq stats` lists them)
  instead of inventing near-duplicates.
- Use the bundle's own extension keys consistently if it has them (status,
  related, etc.). Don't impose this repo's extensions on a bundle that uses only
  the canonical keys.

## Body & cross-linking

- One concept per file. Keep headings meaningful — sections are the unit `search`
  ranks and `get --section` retrieves.
- Link related concepts so the graph stays connected: inline Markdown links to
  other docs, and/or a `related:` list in frontmatter. These become the edges
  `neighbors`/`backlinks`/`path` traverse. A doc with no inbound links shows up in
  `orphans`.
- Write for retrieval: put the answer near the heading; don't bury key terms.

## Verify before you're done

```sh
okq get <id>                 # parses? frontmatter + body render correctly?
okq find --type <type>       # the new doc appears in its type
okq deadlinks                # every link you added resolves (no output = clean)
okq search "<a key term>"    # the doc is findable by its own topic
```

Fix anything these surface. If you renamed or moved files, also run
`okq orphans` and re-link. For ongoing bundle health, hand off to `okq-maintain`.
