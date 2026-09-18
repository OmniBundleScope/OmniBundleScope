## What this changes

<!-- One or two sentences. Link the issue it closes. -->

## The measurement

<!-- Required unless the change cannot affect a number. Say which and why. -->

| before | after | how measured |
|---|---|---|
| | | |

Fixtures and commands used:

```
```

## Checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean
- [ ] `cargo fmt --all --check` is clean
- [ ] A test covers the change, and it fails without the fix
- [ ] Parity still passes if this touches ingest, fusion or the report payload
- [ ] Docs updated: the number, and every language that carries it
- [ ] Benchmarks re-run if this touches a hot path (`bench/harness/measure.ps1`)

## Contracts

- [ ] This does not change the unified graph, report schema, CLI surface or benchmark protocol
- [ ] If it does, an ADR is included (see `docs/contracts/OWNERS.md`)