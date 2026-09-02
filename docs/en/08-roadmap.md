# 08 — Roadmap

Owner: WS-0 · Translations: [ZH](../zh/08-roadmap.md) ·
[JA](../ja/08-roadmap.md) · [DE](../de/08-roadmap.md)

The PRD's calendar (M1-2, M3-4, …) is kept here for continuity with the vision,
but the operative plan is `03-implementation-plan.md` (milestones, no dates).
Where the two disagree, the milestone plan wins.

## Phase 1 — the whole loop (current)

**Scope:** stats ingest · source-map ingest · size attribution · fusion with
ghost/hidden detection · CLI with budgets · self-built HTML report · parity
tests · four-language documentation · multi-platform binaries.

**Exit criteria (all must hold):**

- B1, B2, B3, B4, B5, B8, B9, B10 measured and within target
- parity ≤ 0.1 % against pinned WBA and SME on the real fixtures
- ghost/hidden ≥ 95 % agreement with hand labels on three real projects
- WS-S baseline recorded, and R1 either retired or the positioning rewritten
- release-grade documentation in EN/ZH/JA/DE

**Deliberately not in Phase 1:** GUI, dev server, WebGL, browser-side analysis,
esbuild metafile ingest, CI budget across multiple projects, trend history.
Those are Phase 2+ and the README must not imply otherwise.

## Phase 2 — Web UI and WASM

- `omnibundle-wasm` gets real bindings; the core is already IO-free so this is
  packaging plus a JS API.
- WebGL treemap renderer behind the existing payload contract, keeping the
  Canvas 2D path as a fallback.
- Browser-side drag-and-drop analysis (privacy story: build metadata never
  leaves the machine).
- The module-dependency (edge) view, which needs its own interaction model.
- `--serve` for live re-analysis; `--watch` reserved and implemented here.

**Exit criteria:** 10k-module treemap > 50 FPS in the WebGL path; the same
payload renders identically in the CLI report and the web app; a 1 GB stats
file analysed in-browser without a crash (memory ceiling becomes the browser's,
which is the honest trade-off to document).

## Phase 3 — ecosystem, CI

- esbuild `metafile.json` ingest; rollup/vite visualizer stats; heuristic
  analysis of bundles with no metadata at all.
- Budget across a whole monorepo (`--baseline` for trend diffing, SARIF for
  code scanning).
- Statistical diagnostics: repeated dependency versions, tree-shaking
  effectiveness per package.

**Exit criteria:** the supported input matrix in the README grows by these
formats, with benchmarks for each; the budget gate is used unchanged in three
real CI setups.

## Phase 4 — optional commercialisation

- Historical size trends, PR-level size comments, hosted mode.
- Explicitly optional: if the open-source tool is working, this is a
  distribution decision, not an engineering one, and it does not gate anything
  above.

## Metrics, restated honestly

| PRD metric | how we will actually report it |
|---|---|
| ≥ 5x faster than WBA | per phase, with the fixture class stated. Today only the parse segment is measured (176.3 s → 2.78 s, 63x); the full pipeline is `unverified` and stays that way until B3/B4 are run. |
| 70 % less memory at 500 MB | as a measured pair, per phase (today: 1,985 MB → 59 MB at the parse stage on a 1,049 MB input) |
| 2,000 stars in 6 months | not an engineering target; we do not plan against it and will not restate it as achieved |
| 50,000 weekly npm downloads | same; the honest leading indicators we *do* watch are parity-clean releases and fixture coverage |

The pattern to keep: every public number is either measured and reproducible
from the committed harness, or it is not published.
