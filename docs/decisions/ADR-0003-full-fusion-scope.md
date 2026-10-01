# ADR-0003: Phase 1 scope — full fusion, no time-boxed split

Status: **accepted** (2026-09-02) · Owner: maintainers · Affects: stats ingest..parity

## Context

An earlier plan split the MVP into a "WBA core first, fusion later" cut. The
owner chose the wider scope: Phase 1 ships the whole loop — stats ingest,
source-map ingest, fusion, ghost/hidden detection, CLI, self-built report —
because a tool that parses fast but cannot explain *why* a chunk is big is not
worth switching to, and because the engine splits cleanly into areas that
can proceed in parallel against frozen contracts.

## Decision

Phase 1 = full fusion, organised as parallel areas behind contracts
(`docs/contracts/`) rather than as a time-phased schedule. The
project is expressed as **milestones and dependencies**, never as dates: a
schedule would imply commitments we cannot make honestly before the SME
baseline (the source-map-explorer baseline) exists.

## Consequences

- The **first** parallel batch is contracts (contracts), benchmarks (bench harness),
  the source-map-explorer baseline (SME baseline) and docs and translations (docs/i18n). Nothing downstream starts until the
  contracts are frozen, which is deliberate: with N areas writing against
  an unfrozen schema, the merge cost exceeds the parallelism gain.
- Two stop-the-line conditions are attached to this decision because the scope
  is wide:
  1. If `source-map-explorer` turns out to take **< 1 s** on realistic maps,
     the "fusion is fast" claim is dead and the product must be repositioned
     around cross-bundler insight (the source-map-explorer baseline decides this, and it is the first
     thing that gets measured).
  2. If the 400 MB full pipeline cannot get under **5 s / 200 MB**, we drop
     the performance claim and ship the memory + fusion story instead of
     pretending.
- WASM bindings stay a packaging-only placeholder. `omnibundlescope-core` performs
  no IO so Phase 2 is additive, but no `wasm-bindgen` dependency enters the tree
  until Phase 2 starts.
