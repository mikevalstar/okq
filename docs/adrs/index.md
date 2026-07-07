# Architecture Decision Records

Numbered, immutable-once-committed records of decisions that are expensive to
reverse. List or read them with okq:

    okq --bundle docs find --type adr
    okq --bundle docs get adrs/0002-library-stack

Copy [`_template.md`](_template.md) to start a new one, or run
`okq new adr "<title>"` (once scaffolding lands).

<!-- okq:index:begin -->
### Concepts

| Title | File |
|-------|------|
| ADR-0001 — Documentation-first, in an OKF-shaped docs tree | [0001-documentation-first-okf-shaped.md](0001-documentation-first-okf-shaped.md) |
| ADR-0002 — Library stack (stand on the shoulders of giants) | [0002-library-stack.md](0002-library-stack.md) |
| ADR-0003 — The search index lives in the XDG cache, not the bundle | [0003-search-index-in-xdg-cache.md](0003-search-index-in-xdg-cache.md) |
| ADR-0004 — Exit-code taxonomy | [0004-exit-code-taxonomy.md](0004-exit-code-taxonomy.md) |
| ADR-0005 — Dogfood okq for our own docs, specs, and features | [0005-dogfood-okq-for-docs.md](0005-dogfood-okq-for-docs.md) |
| ADR-0006 — .okqignore for excluding files from a bundle | [0006-okqignore-filtering.md](0006-okqignore-filtering.md) |
| ADR-0007 — Network is opt-in, scoped to skill install | [0007-opt-in-network-for-skill-install.md](0007-opt-in-network-for-skill-install.md) |
| ADR-0008 — Scope & non-goals (no MCP server; vector search not planned) | [0008-scope-non-goals.md](0008-scope-non-goals.md) |
| ADR-0009 — Track a fork of okf until it allows spaces in file names | [0009-okf-spaces-fork.md](0009-okf-spaces-fork.md) |
| ADR-0010 — Widen the okf fork to allow emoji and Unicode in file names | [0010-okf-unicode-filenames-fork.md](0010-okf-unicode-filenames-fork.md) |
| ADR-NNNN — Short title of the decision | [_template.md](_template.md) |
<!-- okq:index:end -->
