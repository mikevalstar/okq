# Changelog

All notable changes to okq are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and okq aims to follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `--json` shapes
and exit codes are treated as an agent/script contract — breaking either is a
deliberate, documented change.

Releases are cut by pushing a `vX.Y.Z` tag, which publishes to crates.io and
attaches prebuilt binaries to the GitHub Release.

## [Unreleased]

## [0.5.0] — 2026-07-07

An Obsidian-parity release: three features that make okq read an Obsidian vault
the way Obsidian does — resolving aliases, counting inline `#tags`, and telling a
genuinely broken link apart from a not-yet-created one. Built on the wikilink
support from 0.3.2; specs in `docs/features/{aliases,inline-tags,phantom-links}.md`
and the resolution-contract decision in
[ADR-0011](docs/adrs/0011-aliases-in-resolution.md).

### Added

- **Frontmatter aliases resolve concepts.** A note's `aliases:` (Obsidian's
  alternate names — a YAML list or a single scalar) now resolve everywhere a
  concept id does: `okq get Hooman`, `neighbors`, `backlinks`, and `path` accept
  an alias (case-insensitively), and a bare `[[Hooman]]` wikilink forms a real
  `wikilink` edge to the aliased note instead of a dead link. Aliases sit at the
  **lowest** resolver priority, so a real filename is never shadowed by another
  note's alias (ADR-0011); colliding aliases error as ambiguous. Spec:
  `docs/features/aliases.md`.
- **Inline `#tags` are first-class tags.** Obsidian-style `#tag` tokens in a
  concept body now count as tags, unified with the frontmatter `tags:` list, so
  `okq find --tag KGPortal` matches a note tagged only inline and `okq stats`
  counts them. The scanner skips code fences/spans, ignores tag-shaped non-tags
  (`#123`, `# heading`, URL fragments, `foo#bar`), supports nested `#area/work`,
  and lowercases for case-insensitive matching. The `tags` array in
  `get`/`find`/`search` output is the deduped union (frontmatter first, then
  inline, author order preserved). Spec: `docs/features/inline-tags.md`.

### Changed

- **`deadlinks` distinguishes *phantom* from *broken*.** A bare `[[Note]]` to a
  note that doesn't exist yet is a **phantom** — normal in an Obsidian vault,
  where you write links before creating notes — not an error. Each
  `okq.deadlinks/v1` record gains a `kind: "broken" | "phantom"` field, and
  **`deadlinks` now lists broken links only by default** (so it isn't thousands
  of false alarms on a vault); `--phantoms` includes phantoms and
  `--phantoms-only` lists just them. `--check` gates on the listed set (broken by
  default). ⚠️ Behavior change: the default `deadlinks` result set and default
  `--check` are narrower than before. A bundle whose bare links all resolve (e.g.
  okq's own `docs/`) is unaffected.
- **`okq stats` splits the health line.** `dead_links` now counts **broken** links
  only; a new `phantom_links` field (and a `Phantom links:` column in the human
  output) counts phantoms. The `okq.stats/v1` schema gains `phantom_links`.

### Notes

- `deadlinks/v1` and `stats/v1` schemas gain fields additively; the *default
  result set* of `deadlinks` narrows (see above). Everything else is backward
  compatible.

## [0.4.0] — 2026-07-07

The first tagged release since `v0.3.0`, so it ships everything below since then
— spaces (0.3.1), wikilinks (0.3.2), optional frontmatter (0.3.3), and the
emoji/Unicode filenames + dependency bumps of this entry. okq depends on the
temporary `okf` fork (ADR-0010), so this release is the GitHub Release + prebuilt
binaries; the crates.io publish stays paused until the fork lands upstream.

### Added

- **Emoji & Unicode in file names.** A concept may now live in a file named with
  emoji (`🚀 Launch.md`), accented Latin (`café.md`), or CJK (`设计.md`) — a
  leading emoji included. Such concepts load and are surfaced by
  `get`/`find`/`search`/graph like any other, and percent-encoded links to them
  (`Q1%20%F0%9F%9A%80%20Launch.md`) resolve. This widens the temporary `okf`
  fork's concept-id rule from an ASCII allowlist to a denylist (reject only
  control chars, `/`, `\`, and `: * ? " < > |`, plus a leading `.`/`-` or a
  leading/trailing space). See
  [`docs/features/emoji-filenames.md`](docs/features/emoji-filenames.md) and
  [ADR-0010](docs/adrs/0010-okf-unicode-filenames-fork.md).

### Changed

- Re-pin the `okf` dependency to the fork's permissive-filenames branch (a
  superset of the spaces branch). [ADR-0010](docs/adrs/0010-okf-unicode-filenames-fork.md)
  supersedes [ADR-0009](docs/adrs/0009-okf-spaces-fork.md).
- Dependencies: bump `ureq` 2.12.1 → 3.3.0 (the skill-install fetch is ported to
  the 3.x request/body API; still rustls, no system OpenSSL) and `ignore`
  0.4.26 → 0.4.27.

### Fixed

- `deadlinks` now reports **broken percent-encoded links** (e.g. a typo'd
  `Quarterly%20Reprot.md` or `%F0%9F…`-encoded target). The graph resolver
  percent-decodes a link target before classifying it — mirroring `okf` — so an
  encoded in-bundle link is judged by the concept it denotes, closing a gap the
  spaces work left (working encoded links resolved; broken ones slipped through).

## [0.3.3] — 2026-07-07

### Added

- **Optional frontmatter.** A Markdown file with no YAML frontmatter is a
  first-class concept: its `title` is inferred from the filename (the concept
  id's last segment, verbatim — no humanizing), so plain note folders are
  titled, searchable (the inferred title is indexed and boosted), and navigable
  like any other bundle. `get --frontmatter` still shows the file's true
  (empty) frontmatter — the inferred title is a display value, not a rewrite.
  Resolves [#6](https://github.com/mikevalstar/okq/issues/6). See
  [`docs/features/frontmatter-optional-title.md`](docs/features/frontmatter-optional-title.md).

### Changed

- The `title` field in the `get`, `find`, `search`, and `stats` `--json`
  envelopes is now **always present** (a string, no longer nullable/omitted),
  since every concept has an inferrable title. Consumers that special-cased a
  missing `title` no longer need to.

## [0.3.2] — 2026-07-07

### Added

- Obsidian-style **wikilinks** are now a graph edge source. `[[Note]]`,
  `[[Note|alias]]`, `[[Note#heading]]`, `[[Note#^block]]`, `[[folder/Note]]`, and
  `![[embeds]]` in a concept body become `wikilink` edges, so Obsidian vaults (and
  any `[[…]]`-linked bundle) are navigable with `neighbors`/`backlinks`/`path`/
  `orphans`/`deadlinks`. Resolution is lenient: a bare name matches a concept's
  filename anywhere in the bundle, case-insensitively; unresolved in-bundle
  targets are reported by `deadlinks`. Filter with `--edge wikilink`. Resolves
  [#5](https://github.com/mikevalstar/okq/issues/5). See
  [`docs/features/wikilinks.md`](docs/features/wikilinks.md).

## [0.3.1] — 2026-07-07

### Changed

- Depend on a temporary fork of `okf` ([mikevalstar/okf#1](https://github.com/mikevalstar/okf/pull/1),
  pinned by commit) that permits optional spaces in concept file names, so
  documents like `Quarterly Report.md` load and their links resolve. Reverts to
  the crates.io release once upstream okf ships this. See
  [`docs/adrs/0009-okf-spaces-fork.md`](docs/adrs/0009-okf-spaces-fork.md).

## [0.3.0] — 2026-06-27

### Added

- `okq validate` (alias `okq doctor`) — report OKF conformance issues: docs okq
  can't parse, concepts missing the required `type`, malformed reserved files, and
  unresolved links, each with severity, path, and reason. `--check` gates CI (exit
  3 on any error), `--severity <error|warning|info>` sets the display floor, and
  `--json` emits the `okq.validate/v1` contract. Surfaces the docs okq otherwise
  loads-but-skips silently. See [`docs/features/validate.md`](docs/features/validate.md).
- `okq index` — regenerate the `index.md` directory listings from the bundle's
  concepts: per-directory folder links and a concept table, written into a fenced
  block so surrounding prose (and the root's `okf_version`) is preserved.
  Idempotent; `--check` fails CI (exit 3) on a stale listing; `--json` emits
  `okq.index/v1`. Completes the authoring loop (`init` → `new` → `index`). See
  [`docs/features/index-command.md`](docs/features/index-command.md).

### Added

- `okq skills install` / `okq skills list` — install and update the bundled
  `okq-*` agent skills (okq-explore, okq-write-okf, okq-maintain, okq-reference).
  Skills are embedded in the binary and written to a canonical `.agents/skills/`,
  then symlinked into `.claude/skills/`. Flags: `--global`, `--from-repo`,
  `--via-skills-sh`. See [`docs/features/skills-install.md`](docs/features/skills-install.md).
- The four `okq-*` agent skills themselves, under [`skills/`](skills/), teaching an
  agent to use okq ([`docs/features/skills.md`](docs/features/skills.md)).

### Notes

- `okq skills install --from-repo` is the only command that uses the network, an
  opt-in exception to the local-first contract ([ADR-0007](docs/adrs/0007-opt-in-network-for-skill-install.md));
  every query command remains offline and deterministic.
- Project scope settled: no MCP server, and vector search remains evidence-gated
  and unplanned ([ADR-0008](docs/adrs/0008-scope-non-goals.md)).

## [0.1.2] — 2026-06-27

### Added

- `.okqignore` support — exclude files from a bundle with full `.gitignore`
  syntax (nested, negation, anchoring), plus a `--no-ignore` escape hatch on every
  command ([ADR-0006](docs/adrs/0006-okqignore-filtering.md),
  [`docs/features/okqignore.md`](docs/features/okqignore.md)).

## [0.1.1] — 2026-06-26

### Added

- Tag-triggered release workflow: publish to crates.io via Trusted Publishing
  (OIDC) and attach prebuilt binaries to the GitHub Release.

## [0.1.0] — 2026-06-26

Initial public beta. The full command surface is implemented, tested, and
dogfooded against this repo's own `docs/` bundle.

### Added

- **Read & search:** `get` (with `--section`/`--frontmatter`/`--body`), `find`
  (`--tag`/`--type`/`--where`/`--match`/`--regex`), and ranked section-level
  `search` (Tantivy BM25, cached in the XDG cache dir).
- **Graph:** `neighbors`, `backlinks`, `path`, `orphans`, `deadlinks` over typed
  edges (inline links + frontmatter relations).
- **Health & contract:** `stats`, `--check` CI gating (exit 3) on
  `orphans`/`deadlinks`, and `schema` (JSON Schema for every `--json` envelope).
- **Scaffold & author:** `okq init` (OKF bundle starter) and `okq new <type>`
  (one concept from an embedded template).
- Agent-runnable across the board: `--json` everywhere, a stable exit-code
  taxonomy ([ADR-0004](docs/adrs/0004-exit-code-taxonomy.md)), and token-frugal
  `path:line` output.

[Unreleased]: https://github.com/mikevalstar/okq/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/mikevalstar/okq/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/mikevalstar/okq/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/mikevalstar/okq/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/mikevalstar/okq/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mikevalstar/okq/releases/tag/v0.1.0
