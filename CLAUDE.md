# CLAUDE.md — working notes for okq

`okq` is a fast, deterministic, local-first CLI for querying **Open Knowledge
Format (OKF)** bundles (Markdown + YAML frontmatter), built to serve humans and
AI agents with the same tool. Full design lives in [PLAN.md](PLAN.md); decisions
and feature specs live in [docs/](docs/). Read those before large changes.

## Layout

Library-first: all logic is in the `okq` lib; the binary is a thin shell.

```
src/
  main.rs              — entry point: std::process::exit(okq::run())
  lib.rs               — run() + dispatch (one arm per subcommand)
  cli.rs               — clap parser; help text, examples, styles
  error.rs             — AppError + exit-code mapping (exit:: constants)
  model.rs             — ConceptRecord (shared envelope) + resolve_concept (partial ids)
  sections.rs          — heading-delimited section chunking (pulldown-cmark)
  yaml_json.rs         — okf YAML → serde_json bridge
  index.rs             — Tantivy search index: schema, build, XDG cache, staleness
  graph.rs             — typed-edge graph: inline links + frontmatter relations, BFS
  templates.rs         — embedded init/new templates + date helper
  commands/{get,find,search,graph,stats,schema,scaffold}.rs
docs/                  — documentation-first OKF bundle: adrs/, features/, guides/, workflows/
docs/tests/            — deliberately malformed fixtures for robustness tests
tests/                 — assert_cmd integration tests + insta snapshots
```

okf (the upstream crate) owns the **data layer** — parse, frontmatter, concepts,
the link graph. okq owns the **query layer**. Don't reimplement what okf provides.

## Commands

```sh
cargo build
cargo test                       # unit + integration; 95+ tests
cargo clippy --all-targets       # must be warning-free
cargo fmt                        # must be clean (cargo fmt --check)
cargo run -- --bundle docs search "tantivy"   # dogfood against our own docs/
INSTA_UPDATE=always cargo test   # accept new/changed insta snapshots
```

## Adding a feature / command — the process

We are **documentation-first** and ship a small, polished increment each time.
For any new command or user-visible feature:

1. **Spec first.** Write `docs/features/<name>.md` from the template
   (`status: draft`). Get it accepted (`status: accepted`) before building. Use
   an **ADR** (`docs/adrs/NNNN-*.md`) for any decision that's expensive to
   reverse (a dependency, a format, a location). See the doc conventions in
   [docs/README.md](docs/README.md). **Use okq to find related specs/ADRs first**
   (e.g. `okq --bundle docs search "<topic>"`, `okq find --tag …`) so you cross-link
   them — don't grep (see Dogfooding below).
2. **Implement**, reusing okf for data and okq's shared pieces: the concept
   envelope (`model.rs` / `get`), the collection envelope (`find`), the section
   model (`sections.rs`), and the exit-code taxonomy (`error.rs`).
3. **Honor output discipline** (see below): stdout = data, `--json` everywhere,
   exit codes from the shared taxonomy, token-frugal (locations, not bodies).
4. **Test**: unit tests for logic + `assert_cmd` integration tests against a
   temp fixture, with `insta` snapshots for JSON/human output. Add a robustness
   case (malformed input must degrade gracefully, never panic).
5. **Bump the version** in `Cargo.toml` (patch bump per change — we keep one
   version per commit; `0.0.1` is the reserved crates.io placeholder).
6. **Check the CLI help is good.** Every command needs: a clear one-line `about`,
   an `after_help` **Examples** block with runnable commands, well-described
   flags, and value names. Keep it `gh`-grade. Then **update and review the help
   snapshots** (`cargo test --test help`, `INSTA_UPDATE=always` to re-accept) and
   eyeball `okq <cmd> --help` and `okq --help`. Help example headers stay **plain
   text** so `--no-color`/`NO_COLOR` are honest (clap colors its own sections).
7. **clippy + fmt clean.**
8. **Update the README** if the user-facing surface changed (it's the crates.io
   front page; keep the commands table + examples current).
9. **Flip the feature spec to `status: active`.**
10. **Check doc health** before committing docs: once `deadlinks`/`orphans` exist,
    run `okq --bundle docs deadlinks` and `okq --bundle docs orphans` so new
    cross-links resolve and no spec is left dangling.
11. **Commit** (see commit conventions) and push when asked.

## Conventions

**Dogfood okq for our own docs** (ADR-0005). For doc/spec/feature work in this
repo, reach for okq before `grep`/`rg`/`fd`:
- Find/read: `okq --bundle docs search "<topic>"`, `okq --bundle docs find --tag …`,
  `okq --bundle docs get <id> --section <heading>`.
- Navigate (once M2 lands): `okq --bundle docs neighbors <id>` / `backlinks <id>` /
  `path <a> <b>`; check health with `deadlinks` / `orphans`.
- Keep `docs/` OKF-shaped and cross-linked so these keep working. If okq can't yet
  do what you need, that gap is the next feature — fix the tool, don't route around
  it. (okq is read/query only; editing stays a normal editor job until `init`/`new`.)

**Output discipline (agent-runnable contract).**
- stdout carries the data; with `--json`, exactly one JSON document on stdout.
- Human messages, warnings, "no results" notes, and logs go to **stderr**.
- Token-frugal: return ranked `path:line` + frontmatter + a short snippet —
  never full document bodies. `get` is the expand-on-demand counterpart.
- Every command has a fully non-interactive path; never prompt.

**Exit-code taxonomy** (shared across commands; `error.rs`):

| Code | Meaning |
|------|---------|
| 0 | success (incl. zero results — empty is not an error) |
| 2 | usage (clap; or runtime `AppError::Usage`: bad `--where`, invalid regex/query) |
| 4 | concept not found / not resolvable |
| 5 | section not found / ambiguous |
| 1 | other (bad bundle, I/O, `AppError::Index`) |

**Determinism & local-first.** Same bundle → same answer. No network, no
embeddings (vectors are deferred + evidence-gated). Ranked results have a stable
tie-break (`score desc, then path, then line`). The Tantivy index is a derived
cache in the **XDG cache dir** (`~/.cache/okq/<key>/`, override `OKQ_CACHE_DIR`),
**never written into the bundle** (ADR-0003).

**Docs are immutable at commit, not at `accepted`.** While uncommitted, edit a
spec/ADR freely. Once committed, don't rewrite a decision — write a new ADR that
supersedes it and add a banner to the old one (see ADR-0002 ↔ ADR-0003).

**Commits.** Conventional prefixes (`feat:`, `fix:`, `docs:`, `test:`). End the
commit message with the `Co-Authored-By: Claude …` trailer. Branch off main
only if asked; this repo commits to `main` directly. Commit/push only when the
user asks.

**Releasing.** Publishing is automated via `.github/workflows/release.yml`, which
fires on a `v*` tag. To cut a release: bump `version` in Cargo.toml, run a build
so `Cargo.lock` updates, commit, then `git tag vX.Y.Z && git push origin vX.Y.Z`.
CI publishes to crates.io via Trusted Publishing (OIDC — no stored token; the
`release` environment must be allowed in the crate's crates.io settings) and
attaches prebuilt binaries to the GitHub Release. The tag must match the
Cargo.toml version (the workflow checks). Don't `cargo publish` by hand unless CI
is unavailable.

## Gotchas

- **okf loads permissively**: malformed docs go to `Bundle::parse_errors` and are
  skipped, not fatal. `index.md`/`log.md` are reserved and not concepts.
- **Section line numbers** depend on `sections::body_start_line` (accounts for
  the frontmatter block + the blank line okf strips). Byte offsets from
  pulldown-cmark are char-safe; keep slicing on those, not byte guesses.
- **serde_yaml is deprecated** and `serde_yml` is RUSTSEC-flagged — don't add
  them; lean on okf's frontmatter parser (or `serde_yaml_ng`/`serde_norway` only
  if truly needed).
- **Tantivy `as_str`/`as_u64`** are trait methods — `use tantivy::schema::document::Value as _;`.
- **The published crate is curated** via `exclude` in Cargo.toml (no `docs/tests`,
  no `tests/`). Re-check `cargo package --list` if you add top-level dirs.
- **Cargo.lock is committed** (okq ships a binary → reproducible builds). cargo
  excludes it from the *published* package automatically.
