# Changelog

All notable changes to okq are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and okq aims to follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The `--json` shapes
and exit codes are treated as an agent/script contract — breaking either is a
deliberate, documented change.

Releases are cut by pushing a `vX.Y.Z` tag, which publishes to crates.io and
attaches prebuilt binaries to the GitHub Release.

## [Unreleased]

## [0.2.0] — 2026-06-27

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

[Unreleased]: https://github.com/mikevalstar/okq/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/mikevalstar/okq/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/mikevalstar/okq/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/mikevalstar/okq/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/mikevalstar/okq/releases/tag/v0.1.0
