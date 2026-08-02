---
type: adr
title: ADR-0013 — Back to upstream okf; retire the okf-permissive fork
status: accepted
created: 2026-08-02
updated: 2026-08-02
tags: [okf, dependencies, fork, filenames, publishing, crates-io]
supersedes: "0012-okf-permissive-crate.md"
superseded-by: null
related:
  - "0012-okf-permissive-crate.md"
  - "0010-okf-unicode-filenames-fork.md"
  - "0009-okf-spaces-fork.md"
  - "0002-library-stack.md"
  - "../features/emoji-filenames.md"
  - "../features/validate.md"
  - "../guides/design-overview.md"
---

# ADR-0013: Back to upstream okf; retire the okf-permissive fork

## Context

[ADR-0012](0012-okf-permissive-crate.md) published our `okf` fork as
[`okf-permissive`](https://crates.io/crates/okf-permissive) and pointed okq at
it by version. The reasoning was that upstream had gone quiet — one release
(`0.1.0-alpha.1`, 2026-06-16), no commits since, our filename issue unanswered
since 2026-07-07 — so waiting was unbounded and the git pin was blocking
crates.io publishing. Owning the fork was accepted as the price.

That reasoning has expired in the other direction. **Upstream shipped `okf`
0.2.0 and 0.2.1 on 2026-07-27**, two days after ADR-0012 was written. The
release implements OKF spec **v0.2** and adds whole modules okq's fork does not
have: `provenance`, `trust`, `actor`, `computation`, `diff`, `lint`, `log`,
`footnotes`.

More to the point, it contains the fork's entire reason for existing. Upstream's
`validate_segment` now rejects only path separators (`/`, `\`), control
characters, and the empty/`.`/`..` cases — spaces, emoji, accented Latin, and
CJK all pass. It also independently added percent-decoding of link targets, the
other half of what [ADR-0009](0009-okf-spaces-fork.md) and
[ADR-0010](0010-okf-unicode-filenames-fork.md) carried.

Upstream is in fact *more* permissive than our fork, which still denies the
Windows-reserved set (`: * ? " < > |`), a leading `.`/`-`, and edge spaces.
Upstream moved that judgment out of validation and into a new
`is_portable_segment()` predicate reported as a **lint warning** — the bundle
stays conformant, the author gets told the name may not link or travel cleanly.
That is a better split than ours: the spec does not forbid those characters, so
neither should the loader.

The cost side of ADR-0012 has not gone away. `okf-permissive` is a crate we
publish, release, and answer for, and it now sits two spec versions behind a
data layer that is being actively developed again.

## Options considered

### Option A — Keep the fork

No work today. But the fork's only functional delta is now upstream, so we would
be maintaining a rename. It also freezes okq's data layer at OKF v0.1 while the
format moves, and every upstream fix becomes a manual port.

### Option B — Fork upstream 0.2 again

Rebase the permissive rule onto 0.2 to keep the stricter Windows-reserved
denylist. Rejected: the stricter rule is the part we would be defending, and
upstream's lint-not-error treatment of it is the better answer. There is nothing
left to fork *for*.

### Option C — Return to upstream okf (chosen)

Depend on `okf = "0.2.1"` from crates.io. Drop the `package = …` rename.

## Decision

**Option C.** okq depends on upstream `okf` 0.2.1 directly:

```toml
okf = "0.2.1"
```

The switch was measured before being decided. It is **six mechanical edits**,
all from 0.2 API changes unrelated to filenames:

- `Frontmatter::title()` / `type_()` return `Cow<'_, str>` rather than `String`
  (five call sites in `model.rs`, `commands/{get,search,stats}.rs`).
- `BundleError::Io` became a struct variant, `Io { kind, message }` (one call
  site in `error.rs`).

With those applied the full suite passes unchanged — including the emoji and
spaced-filename robustness fixtures — clippy is clean, and `okq --bundle docs
stats` reports identical numbers.

`okf-permissive` is **archived, not deleted**. It stays on crates.io so existing
`Cargo.lock` files keep resolving; its README gains a banner pointing at upstream
0.2 as the successor. We do not yank it.

## Consequences

- **We no longer maintain a data layer.** The cost ADR-0012 accepted is retired.
  The data/query split from [ADR-0002](0002-library-stack.md) goes back to being
  a dependency boundary rather than a boundary between two crates we own.
- **okq tracks OKF v0.2.** The new upstream modules (provenance, trust,
  attestation, log) are surface okq can build on rather than reimplement. What,
  if anything, to expose is a separate feature question — this ADR only moves
  the dependency.
- **`okq validate` gains a portability warning.** Upstream's lint reports a
  concept-id segment outside `[A-Za-z0-9_][A-Za-z0-9_.-]*` as a warning
  (`🚀 launch.md` in `docs/tests` now trips it). Bundles stay conformant and
  exit codes are unchanged; the message is new. See
  [validate.md](../features/validate.md).
- **Filename behaviour is unchanged where it matters and wider where it does
  not.** Emoji, spaces, accents, and CJK all still load, as
  [emoji-filenames.md](../features/emoji-filenames.md) describes. Names we used
  to reject (`report:2026.md`, `-draft.md`) now load with a warning.
- **We are exposed to upstream again.** If it goes quiet a second time, the
  fork is one `git revert` away — and this time we would fork from a maintained
  0.2, not from an alpha.

## Related

- [ADR-0012](0012-okf-permissive-crate.md) — published the fork; this supersedes
  it and retires it
- [ADR-0010](0010-okf-unicode-filenames-fork.md) — the character rule upstream
  has now adopted (and widened)
- [ADR-0009](0009-okf-spaces-fork.md) — the original spaces fork
- [ADR-0002](0002-library-stack.md) — `okf` owns the data layer; this restores
  that as an ordinary dependency
- [emoji-filenames.md](../features/emoji-filenames.md) — the capability, now
  provided upstream
- [validate.md](../features/validate.md) — where the new portability warning
  shows up
- [design-overview.md](../guides/design-overview.md) — the data/query split this
  preserves
