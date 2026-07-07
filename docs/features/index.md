# Feature specifications

One spec per command or capability — what it does, its scope, behavior, and
acceptance criteria. List or read them with okq:

    okq --bundle docs find --type feature
    okq --bundle docs get features/search

Copy [`_template.md`](_template.md) to start a new one, or run
`okq new feature "<title>"` (once scaffolding lands).

<!-- okq:index:begin -->
### Concepts

| Title | File |
|-------|------|
| Feature name (often a command, e.g. "okq search") | [_template.md](_template.md) |
| Emoji & Unicode in file names | [emoji-filenames.md](emoji-filenames.md) |
| okq find — filter concepts by predicate | [find.md](find.md) |
| Optional frontmatter — infer title from filename | [frontmatter-optional-title.md](frontmatter-optional-title.md) |
| okq get — expand one concept on demand | [get.md](get.md) |
| okq graph navigation — neighbors / backlinks / path / orphans / deadlinks | [graph.md](graph.md) |
| okq index | [index-command.md](index-command.md) |
| .okqignore — exclude files from a bundle | [okqignore.md](okqignore.md) |
| okq init & new — scaffold and author OKF bundles | [scaffold.md](scaffold.md) |
| okq schema — emit the JSON Schema for command output | [schema.md](schema.md) |
| okq search — ranked full-text retrieval | [search.md](search.md) |
| okq skills (install / list) | [skills-install.md](skills-install.md) |
| Agent skills (okq-* suite) | [skills.md](skills.md) |
| okq stats — bundle overview & health metrics | [stats.md](stats.md) |
| okq validate (alias doctor) | [validate.md](validate.md) |
| Wikilinks — Obsidian-style [[links]] as graph edges | [wikilinks.md](wikilinks.md) |
<!-- okq:index:end -->
