---
okf_version: "0.1"
---

# okq documentation bundle

An [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)
(OKF) v0.1 bundle — Markdown + YAML frontmatter, one concept per file,
cross-linked into a knowledge graph. It holds the design of
[okq](https://github.com/mikevalstar/okq): architecture decisions under
`adrs/` and feature specs under `features/` (with contributor templates in
`guides/` and `workflows/`).

This tree is queried with okq itself (we dogfood — see ADR-0005):

    okq --bundle docs find --type adr
    okq --bundle docs search "<topic>"
    okq --bundle docs stats

See [README.md](README.md) for the structure and conventions.
