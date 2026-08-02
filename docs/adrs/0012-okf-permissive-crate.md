---
type: adr
title: ADR-0012 — Depend on the published okf-permissive crate, not a git fork
status: superseded
created: 2026-07-25
updated: 2026-08-02
tags: [okf, dependencies, fork, filenames, publishing, crates-io]
supersedes: "0010-okf-unicode-filenames-fork.md"
superseded-by: "0013-back-to-upstream-okf.md"
related:
  - "0013-back-to-upstream-okf.md"
  - "0010-okf-unicode-filenames-fork.md"
  - "0009-okf-spaces-fork.md"
  - "0002-library-stack.md"
  - "../features/emoji-filenames.md"
  - "../guides/design-overview.md"
---

# ADR-0012: Depend on the published okf-permissive crate, not a git fork

> **Superseded by [ADR-0013](0013-back-to-upstream-okf.md): okq depends on
> upstream `okf` again, and the fork is retired.** Two days after this was
> written, upstream shipped `okf` 0.2 — implementing OKF spec v0.2 and adopting
> a permissive concept-id rule wider than the fork's, plus percent-decoded link
> targets. The premise below ("upstream has gone quiet, so the wait is
> unbounded") no longer holds, and with it the reason to own a data layer. The
> `okf-permissive` crate stays published and unyanked, but okq no longer uses it.

## Context

[ADR-0010](0010-okf-unicode-filenames-fork.md) pinned okq's data layer to a
**git dependency** on a branch of our `okf` fork, on the explicit assumption that
the arrangement was temporary: upstream would ship the permissive filename rule,
and okq would revert to the crates.io release. That assumption has expired.

- Upstream [`W4G1/okf`](https://github.com/W4G1/okf) has published exactly one
  version, `0.1.0-alpha.1` (2026-06-16), and has had **no commits since**. Our
  filename ticket (upstream issue #1) has sat unanswered since 2026-07-07.
- The OKF format itself is alive — the spec home,
  [GoogleCloudPlatform/knowledge-catalog](https://github.com/GoogleCloudPlatform/knowledge-catalog),
  is actively developed — but it ships **Python and TypeScript only**. There is
  no official Rust implementation to migrate to.
- A survey of crates.io found no drop-in replacement. `ryu-knowledge` is the
  closest philosophical match (OKF v0.1, "permissive by contract", and it does
  no filename validation at all), but it is an internal extraction from a
  commercial platform monorepo, it depends on the RUSTSEC-flagged `serde_yml`,
  and it lacks the pieces okq builds on: `ConceptId`, `Frontmatter`/`Value`,
  `validate_bundle`/`Diagnostic`/`Severity`, and typed relations.
  `okf-open-knowledge-format` shares the name but not the format — it has its
  own `okf://` URI scheme, root registry, and file-admission layer, and rejects
  what it does not recognise. `scrinium`'s repository is gone.

Meanwhile the git pin has a standing cost ADR-0010 accepted and named: **crates.io
disallows git dependencies**, so okq could not publish at all. It has been stuck
at 0.3.0 on crates.io since 2026-06-27 while 0.4.0 and 0.5.0 shipped to GitHub
only, behind an install-from-git note in the README.

## Options considered

### Option A — Keep waiting on upstream

Hold the git pin and keep the README workaround. Costs nothing today, but the
evidence says the wait is unbounded, and every release in the meantime is
invisible to `cargo install okq`, `cargo binstall`, and `mise use cargo:okq`.

### Option B — Vendor the data layer into okq

Copy `okf`'s parser and model into `src/`. Unblocks publishing, but it deletes
the data/query split [ADR-0002](0002-library-stack.md) is built on and makes okq
the owner of a YAML parser it does not want to maintain.

### Option C — Publish the fork as its own crate (chosen)

Merge our two fork branches, rename the package, publish it, and depend on it
like any other crates.io dependency.

## Decision

**Option C.** The fork is published as
[`okf-permissive`](https://crates.io/crates/okf-permissive) from
[mikevalstar/okf-permissive](https://github.com/mikevalstar/okf-permissive), and
okq depends on it by version:

```toml
okf = { package = "okf-permissive", version = "0.2.0" }
```

- **The library crate is still named `okf`**, so every `use okf::…` in okq is
  unchanged and the dependency source is a one-line switch in either direction.
  Reverting to upstream means deleting `package = …`, exactly as before.
- **Both fork branches were merged first.** The spaces branch (ADR-0009) and the
  permissive-filenames branch (ADR-0010) were siblings off upstream's initial
  commit; since the denylist rule is a strict superset of the spaces rule, the
  merge resolves to the denylist and nothing is lost.
- **The fork is labelled as one.** Its README leads with a fork banner and a
  table of what diverges, its `NOTICE` lists the modifications as Apache-2.0
  §4(b) requires, and it credits the upstream author. This is a maintained fork,
  not a rename of someone else's work.
- **It has its own release automation**, mirroring okq's: a tag triggers a
  crates.io publish over Trusted Publishing (OIDC) plus prebuilt binaries.
- The exit condition from ADR-0009/0010 is **retired, not met**. okq no longer
  waits on upstream. If upstream ever ships an equivalent rule, switching back is
  the same one line — but that is now an optimisation, not a plan.

## Consequences

- **okq can publish to crates.io again.** The blocker named in ADR-0010's
  consequences is gone; 0.5.1 is the first release to reach crates.io since
  0.3.0, and the README's install-from-git workaround is removed.
- **We own a fork.** Bugs in the data layer are now ours to fix and release —
  the cost of Option C, and the reason it was deferred twice. The surface is
  small (a zero-dependency, std-only crate) and we were already carrying it.
- **Builds are still reproducible**, now by ordinary semver plus the committed
  `Cargo.lock`, rather than a pinned git revision.
- **Filename behaviour is unchanged.** `okf-permissive` 0.2.0 is the merge of
  the two pinned branches, so the emoji/Unicode capability described in
  [emoji-filenames.md](../features/emoji-filenames.md) behaves exactly as it did
  under the git pin.

## Related

- [ADR-0010](0010-okf-unicode-filenames-fork.md) — the git-pinned fork this
  supersedes; the character rule it chose is what `okf-permissive` ships
- [ADR-0009](0009-okf-spaces-fork.md) — the original spaces fork, merged into
  the published crate
- [ADR-0002](0002-library-stack.md) — `okf` owns the data layer; this changes
  *where the build comes from*, not the data/query split
- [emoji-filenames.md](../features/emoji-filenames.md) — the user-visible
  capability the fork exists to provide
- [design-overview.md](../guides/design-overview.md) — the data/query split this
  preserves
