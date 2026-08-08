# Vendored OKF specification

`SPEC.md` is an **exact, unmodified copy** of the Open Knowledge Format
specification, embedded into the okq binary and printed verbatim by `okq spec`.

- **Source:** <https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md>
- **Version:** OKF 0.2
- **Pinned commit:** `3fcbb9f828c2f23d109c855ee403c3a4c81f3a96`
- **Retrieved:** 2026-08-08
- **License:** Apache-2.0 (`LICENSE.md` is the upstream license, retained per
  its redistribution terms; okq itself is also Apache-2.0)

## Updating

When upstream revises the spec:

1. Replace `SPEC.md` with the new raw file, byte-for-byte — no reflowing, no
   edits. `okq spec` promises the *exact* text.
2. Update the pinned commit, version, and retrieval date above.
3. Update `OKF_VERSION` in `src/commands/spec.rs` if the spec's version bumped,
   and re-check okq's behavior against the changed sections (see
   `docs/adrs/0015-embed-okf-spec.md`).
