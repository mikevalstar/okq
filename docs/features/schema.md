---
type: feature
title: okq schema — emit the JSON Schema for command output
status: active # draft | accepted | active | deprecated
created: 2026-06-26
updated: 2026-06-26
tags: [cli, schema, json, contract, agents]
milestone: M3
command: "okq schema"
related: ["stats.md", "../adrs/0004-exit-code-taxonomy.md", "../guides/design-overview.md"]
---

# okq schema — emit the JSON Schema for command output

## Summary

`okq schema [<command>]` prints the **JSON Schema** for a command's `--json`
output envelope (or all of them), generated from the `schemars` derives every
output type already carries. It makes the agent-facing output contract explicit
and machine-checkable: an agent can fetch the schema and validate okq's output;
we can snapshot it to catch accidental drift.

## Motivation

PLAN.md §7's M3 calls for **"stable JSON schemas documented."** Agents depend on
okq's `--json` shape (the `okq.<command>/vN` envelopes); that dependency should be
a *published contract*, not folklore read off examples. Every output struct
already derives `schemars::JsonSchema` — this command surfaces those as real JSON
Schema documents, closing the loop between "we carry the derives" and "the
contract is usable."

## Scope

### In scope

- Emit the JSON Schema for any one command's `--json` envelope, or all at once.
- Generated from the live `schemars` derives, so the schema can't drift from the code.

### Out of scope

- **Validation** — okq emits schemas; validating arbitrary documents against them
  is the caller's job (any JSON Schema validator).
- **Bundle access** — `schema` is static; it doesn't read a bundle.

## Behavior

```sh
okq schema stats          # JSON Schema for the okq.stats/v1 envelope
okq schema neighbors      # ...for okq.neighbors/v1
okq schema                # all schemas, as one JSON object keyed by command
okq schema > schemas.json # the whole contract as a committable artifact
```

- `<command>` is one of the JSON-producing commands: `get`, `find`, `search`,
  `neighbors`, `backlinks`, `path`, `orphans`, `deadlinks`, `stats`. (`neighbors`
  and `backlinks` share an output type, hence an identical schema.)
- With **no argument**, emits a JSON object `{ "<command>": <schema>, … }`.
- Output is always a JSON Schema document on stdout (it *is* JSON); the global
  `--json` flag is redundant here.

### Versioning (the contract rule)

The `schema` field of each envelope carries a version tag (`okq.get/v1`, …). The
schema for `vN` is **stable**: a breaking change to an envelope bumps the version
(`v2`) rather than mutating `v1` in place (PLAN.md §8). `okq schema` output is
snapshot-tested, so an unintended shape change fails CI — turning the contract
into something enforced, not just asserted.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Schema emitted |
| 2 | Unknown command argument (lists the known ones) |

## Acceptance criteria

- [ ] `okq schema <command>` prints a valid JSON Schema for that command's envelope.
- [ ] `okq schema` prints all schemas keyed by command name.
- [ ] An unknown command argument exits 2 and names the known commands.
- [ ] Schemas are generated from the `schemars` derives (no hand-written schema).
- [ ] Output is snapshot-tested so envelope-shape drift is caught.

## Open questions

- **Committed artifact** — also generate a `docs/reference/schemas/` tree in CI so
  the contract is browsable without running okq? (Deferred; the command is the
  source of truth.)
- **Schema dialect** — track whatever `schemars` emits (currently a recent draft);
  pin/annotate the dialect if agents need a specific one.

## Related

- [stats](stats.md) — where this deliverable was first specced (the other half of M3)
- [ADR-0004](../adrs/0004-exit-code-taxonomy.md) — the sibling "contract" decision (exit codes)
- [PLAN.md](../guides/design-overview.md) — §7 M3 (documented JSON schemas), §8 schema versioning as a day-one contract
