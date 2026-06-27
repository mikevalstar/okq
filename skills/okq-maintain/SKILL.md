---
name: okq-maintain
description: Find and fix OKF doc-bundle rot with okq — dead links, orphans, stale status. Use after renames or moves, before committing doc changes, or in CI.
allowed-tools: Bash, Read, Edit
---

Keep the bundle's graph connected and its metadata honest. `okq` reports the
problems; you fix them in the files. See `okq-reference` for the command contract.

## Health checks

```sh
okq --bundle <dir> deadlinks   # links pointing at missing/renamed concepts
okq --bundle <dir> orphans     # concepts with no inbound links (stale candidates)
okq --bundle <dir> stats       # distribution, link density, hubs — spot outliers
```

Clean output means nothing to fix. Run all three after any rename, move, or
delete.

## Audit a document against the code

The checks above catch structural rot. This catches *semantic* rot — a doc that
parses and links fine but no longer matches what the code does. Do this when
auditing a doc, after a feature changes, or when a doc is suspected stale.

1. **Read the doc's claims.** `okq get <id>` (or `--section` for a big doc).
   Pull out the concrete, checkable assertions: commands, flags, file paths, API
   names, types, defaults, behavior, exit codes — not the prose.

2. **Compare each claim to the code.** Find the implementation (grep/read the
   relevant source, run the command, check the actual signature). For every
   claim, mark it: **matches**, **drifted** (code changed), or **wrong** (doc was
   never right). Note exactly what differs — old value → current value.

3. **Confirm discrepancies with the user before changing anything.** Present the
   list: for each, say whether the *doc* looks stale or the *code* looks like the
   regression, and ask which to treat as the source of truth. Don't assume the
   code is always right — a doc may capture intended behavior the code drifted
   from. Decisions in committed ADRs are immutable: if one is now wrong, supersede
   it with a new record rather than rewriting it (follow the bundle's convention).

4. **Apply the agreed fixes** to the doc; bump `updated:` if the bundle tracks
   it. Flag any code-side issues separately for the user.

5. **Check related nodes for the same problem.** A change rarely sits in one doc.
   Walk the graph and re-audit the neighbors:
   ```sh
   okq neighbors <id> --depth 1     # docs this one links to / from
   okq backlinks <id>               # docs that depend on this one
   ```
   For each linked doc, confirm it's still consistent with the fix you just made
   (a corrected flag name, a renamed concept, a reversed decision). Repeat steps
   1–4 on any that drifted. Stop when the neighborhood is consistent.

## Fixing dead links

`deadlinks` reports each broken link with its source `path:line` and the target
it failed to resolve.

1. Decide whether the **target** moved/renamed or the **link** is wrong.
2. If the target moved, update every link to the new id — find them with
   `okq backlinks <old-or-new-id>` and `okq search "<old name>"`.
3. Re-run `okq deadlinks` until it's clean.

## Fixing orphans

An orphan has no inbound links. Either it should be linked from somewhere, or
it's genuinely stale.

1. For each orphan, find where it *should* be referenced (`okq search` for its
   topic) and add a link from the relevant doc — or a `related:` entry.
2. If it's obsolete, mark it (`status: deprecated` if the bundle uses status) or
   remove it. Don't leave it dangling silently.
3. Note: an index/landing doc with no inbound links can be a legitimate root —
   confirm before "fixing" it.

## Status & metadata hygiene

If the bundle uses a status lifecycle (e.g. draft → accepted → active →
deprecated):

```sh
okq find --where status=draft     # drafts that may be stale
okq find --where status=accepted  # accepted specs not yet flipped to active
```

Advance or close out stragglers; update `updated:` dates when the bundle tracks
them. Keep tags consistent — `okq stats` surfaces near-duplicate tags to merge.

## CI gating

`--check` makes a check fail the build (exit 3) when problems exist:

```sh
okq deadlinks --check && okq orphans --check
```

Use it in CI or a pre-commit hook so doc rot can't land. Branch on `$?`, not the
text output.
