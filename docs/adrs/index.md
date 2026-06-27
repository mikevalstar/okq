# Architecture Decision Records

Numbered, immutable-once-committed records of decisions that are expensive to
reverse. List or read them with okq:

    okq --bundle docs find --type adr
    okq --bundle docs get adrs/0002-library-stack

Copy [`_template.md`](_template.md) to start a new one, or run
`okq new adr "<title>"` (once scaffolding lands).
