# 03 — Implementation plan

Owner: WS-0 · Translations: Translations: per-document translations are still in progress. [ZH](../zh/README.md) · [JA](../ja/README.md) · [DE](../de/README.md)

This plan is expressed as **milestones and dependencies, not dates**. Several
workstreams run in parallel here; a calendar would imply commitments nobody
can make before the source-map baseline exists. The single scheduling
primitive is the dependency edge.

## 1. Milestones

```
M0  contracts + bench + SME baseline + docs      (parallel, no dependencies)
     │  gates: contracts frozen · harness green · SME baseline recorded
     ▼
M1  ingest            WS-1 stats   ·   WS-3 source maps   ·   WS-5 report payload
     │  gates: B1, B2, B5 meet their targets; parity harness exists
     ▼
M2  attribute + fuse  WS-2 sizes   ·   WS-4 fusion
     │  gates: B3, B4, B8 meet targets; ghost/hidden agreement ≥ 95 %
     ▼
M3  ship the loop     WS-6 CLI     ·   WS-5 report shell  ·   WS-7 parity
     │  gates: B9 parity ≤ 0.1 %; B10 first paint / fps; budget exit codes
     ▼
M4  distribute        WS-8 binaries + npm + CI     ·   WS-9 four-language docs
        gates: release-grade docs in 4 languages; CI green on all platforms
```

WS-9 (docs/i18n) has no upstream dependency at all: it starts on day one and
never blocks, because the EN documents *are* the specification other lanes read.

## 2. Workstreams

Full lane/path rules: `docs/contracts/OWNERS.md`. Each lane lists its contract
prerequisites, so you can check whether it may start.

### WS-0 — contracts (gate for everything)

- Freeze `report-schema.json`, `unified-graph.md`, `cli-surface.md`,
  `bench-spec.md`, `i18n-parity.md`; keep them in sync with `model.rs`.
- Own ADR-0001..0005 and approve cross-lane contract changes.
- Done when: schema validates, core types are generated from it, and every
  payload field has a documented meaning including who may not change it.

### WS-A — bench harness

- `bench/fixtures/{fetch.sh,fetch.ps1,manifest.json}`, generators, runners for
  WBA / node / ours, result JSON writer, `check-i18n.mjs`.
- Done when: the reference numbers in `01-evidence.md` §2 reproduce from a clean
  clone, and CI publishes a result artifact per PR that touches a measured path.

### WS-S — source-map baseline (the project's load-bearing unknown)

- Measure `source-map-explorer` on our real fixtures (preact/marked/chalk/dayjs
  builds) with wall time, peak RSS and, where possible, per-phase.
- Measure a Rust prototype (zero-allocation VLQ + binary search) on the same
  inputs.
- **Report: does SME have a real pain point at realistic scale?**
- Gate: if SME < 1 s on realistic maps, escalate immediately — ADR-0003
  stop-the-line #1 applies and the product is repositioned before M1 closes.

### WS-1 — stats ingest

- Streaming seeds for `assets`, `chunks`, `modules`; unknown fields skipped
  without allocation; stats v4 and v5 shapes.
- Targets B1/B2; parity: module keys and sizes identical to WBA.
- Risk to watch: WBA tolerates odd files; we must not be stricter than the tool
  we replace. Malformed input is a diagnostic, not a crash.

### WS-2 — size attribution

- `stat` from the stats file, `parsed` from the emitted files, `gzip` at level 6
  (byte-identical to the reference), `attributed` from WS-3.
- rayon over assets and modules; the attribution tree (squarified layout
  computed in core so the payload is deterministic).
- Targets B3/B4. This is the first real test of the full-pipeline claim; profile
  before optimising and publish the phase breakdown either way.

### WS-3 — source map ingest

- Base64 VLQ decoding in place; flat `Vec<Mapping>`; `sourcesContent`;
  `sections` (index maps) support; reject non-v3 with `FS0010`.
- Target B5. Equivalence test: identical mapping count and attributed byte
  totals against a JS reference on the same `.map`.

### WS-4 — fusion engine

- Join modules to sources, compute `attributed` sizes and deltas, classify
  ghost modules and hidden sources, enforce the size invariant.
- Gate: three hand-labelled real projects, ≥95 % agreement.
- Also: `significant_corrections` (modules whose corrected size moves the
  picture by >1 %) — this is the headline number the report leads with.

### WS-5 — report

- Payload builders (three dimensions, CSV projection) in core.
- Shell: single self-contained HTML, Canvas 2D squarified treemap, fuzzy search,
  dark theme, `sourcesContent` drill-down when `--include-sources` is on.
- Target B10. The edge/module-dependency view is Phase 2 by decision.

### WS-6 — CLI

- Zero-config scan, flag alignment with the reference tools, budget config with
  non-zero exit, `--json`, `--dims`, deterministic output, Windows included
  (the reference machine is Windows; a tool that only works on Linux cannot
  verify its own benchmarks).
- Also owns the `.fastscope-cache/` implementation (content-addressed).

### WS-7 — parity tests

- Differential harness: run WBA and SME on the same fixture, diff the *numbers*
  (not the HTML), tolerate ordering only.
- Gate B9. Blocks the npm release, per ADR-0003.

### WS-8 — distribution & CI

- Static binaries for windows/macos(x64, arm64)/linux(x64, arm64), npm wrapper
  that downloads the platform binary, GitHub Actions: test, clippy, bench,
  release.
- crates.io first, then the GitHub release, then npm (ADR-0004 ordering).

### WS-9 — docs & i18n

- EN is normative; ZH/JA/DE follow the parity contract; glossary per language;
  `check-i18n.mjs` in CI; release-grade set is README + PRD + glossary.

## 3. Definition of done (identical for every lane)

1. `cargo test --workspace` green; `cargo clippy --all-targets` without errors.
2. At least one test that fails if the lane's core claim is false.
3. A benchmark run appended to `bench/results/`, and the relevant row in
   `04-benchmark-plan.md` marked measured or explicitly `unverified`.
4. Payload changes touch `report-schema.json` and `model.rs` together, with an
   ADR.
5. Doc comments on non-obvious code, in English.

## 4. Stop-the-line conditions

| condition | consequence |
|---|---|
| SME < 1 s on realistic maps (WS-S) | positioning rewritten: cross-bundler insight, not speed (ADR-0003) |
| 400 MB full pipeline > 15 s or > 500 MB (WS-2) | drop the performance claim, lead with memory + fusion; re-profile before any optimisation |
| Fusion size drift > 1 % (WS-4) | stop, fix join correctness; ghost/hidden work does not start |
| Parity diff > 0.1 % (WS-7) | no npm release |
| Any lane ships without a benchmark number | treated as off-track, reverted in review |

## 5. Explicitly out of scope for Phase 1

GUI, dev server, plugin system, historical trend storage, SARIF output,
monorepo-wide multi-project correlation, source-map *generation* (we analyse
maps, we do not emit them), and any code that would duplicate a bundler.

## 6. What "done" looks like from the outside

```bash
npx fastscope ./dist
# → fastscope-report.html (one file, no network)
# → exit 0, or exit 1 with FS0040 when a budget is breached
# and, for the same input, the numbers match WBA and SME to within 0.1 %
```
