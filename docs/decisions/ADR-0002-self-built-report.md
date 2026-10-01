# ADR-0002: Write our own HTML report, vendor nothing

Status: **accepted** (2026-09-02) · Owner: maintainers · Affects: report, the CLI

## Context

`webpack-bundle-analyzer`'s report is a single HTML file that embeds a
JavaScript treemap viewer. It is MIT licensed, so vendoring it is legally fine.
The alternatives were:

1. vendor the WBA viewer → instant parity with WBA, zero new UI work, but the
   report stays a *treemap viewer*, and we inherit a UI we do not control and
   cannot extend with fusion-specific views (per-source drill-down, ghost/hidden
   panels, budget overlays).
2. write our own shell → a week or two of work, no license entanglement, full
   control over the payload contract, and the ability to drop fusion data into
   the UI without fighting someone else's data model.
3. a framework (React/Vue) plus a treemap library → fastest to build, biggest
   output, worst fit for "one self-contained file you can email someone".

## Decision

Write our own: a self-contained HTML file, Canvas 2D squarified treemap
(algorithm per Bruls, Huizing & van Wijk, as popularised by d3-squarify),
fuzzy search, three grouping dimensions, dark theme. No runtime dependency, no
network requests, no vendored third-party viewer code. The payload is inlined
JSON from `report-schema.json`.

Canvas 2D now, WebGL later: 10k nodes is comfortably inside Canvas 2D's budget,
and it keeps the Phase 1 output small and dependency-free. Phase 2 can swap the
renderer behind the same payload contract without touching the CLI.

## Consequences

- Report parity with WBA is **visual only by intent**; numeric parity is what
  the parity tests check (see `05-parity-and-testing.md`).
- Phase 1 does not get a "module dependency" (edge) view — it needs its own
  interaction model and lands in Phase 2.
- No `--serve` in Phase 1: a static file is the whole promise, and a dev server
  would put a network surface next to a tool that is often pointed at private
  build metadata. The flag is reserved, not implemented.
- Licensing stays clean: the only code in the repo is ours plus dependencies
  under MIT/Apache-2.0.
