# 01 — Evidence: why this project exists

Owner: WS-0, with WS-S for the source-map section · Translations:
[ZH](../zh/01-evidence.md) · [JA](../ja/01-evidence.md) · [DE](../de/01-evidence.md)

This document is the honest ledger behind OmniBundle. Every number here was
measured on the reference machine unless it is explicitly marked as coming from
an upstream issue. Nothing in this file is aspirational.

Reference machine: Windows, node 24.18.0, python 3.12.10, rustc/cargo 1.97.1,
2026-09-02.

## 1. Where the idea came from

OmniBundle is the survivor of a nine-round direction search for "rewrite a slow,
in-demand developer tool in Rust". The full search record lives outside this
repository and is referenced in §7; the short version:

- 28+ candidate directions were screened through three gates: demand (weekly
  downloads ≥ 1,000), supply (two rounds of searching, including for existing
  Rust/Go rewrites), and a mandatory three-sentence feasibility case.
- Most candidates died on their own numbers. Measured and killed: `zod`
  (322,581 validations/s), `js-yaml` (20.5 MB/s), `papaparse` (56.6 MB/s),
  `chalk` (35.4M ops/s), `jsdiff` (20k lines in 9 ms), `changesets` (1.4 s for a
  500-package workspace), `PyInstaller` onefile start-up (0.58 s at 7.7 MB,
  1.04 s at 19.7 MB — the widely reported 10 s did not reproduce).
- Others died because a Rust rewrite already exists: `djlint` (djangofmt v1.0.0),
  markdown/MDX (markdown-rs, by remark's own author), `clean-css`
  (lightningcss, 5.8x), `git-filter-repo` (filter-repo-rs), `jsonschema` (Rust
  crate at 802 stars), `pre-commit` (prek), `lint-staged` (lefthook), `csvkit`
  (qsv), `openpyxl` (calamine).
- `webpack-bundle-analyzer` was the only candidate that passed all three gates
  with pain **and** headroom both measured. `source-map-explorer` came along
  because it is the other half of the same problem, and because the fusion of
  the two is the actual product.

Two lessons from that search are baked into this project's rules:

1. **High download counts do not mean slow.** Measure first, believe nothing.
2. **Real speedups are 4-19x, not 100x** (measured: nunjucks→minijinja 4.1x,
   xml2js→roxmltree 19x, fs-extra copy→rayon 4.0x, jsdom→kuchiki 7x). A 4x win
   is not a reason to rewrite a whole tool; a memory ceiling or a per-item
   process is.

## 2. The webpack-bundle-analyzer measurements (ours)

Fixture: synthetic `stats.json` in the shape WBA consumes — 1,500 assets,
200 chunks, modules with embedded `source` strings.

| input | tool | wall | peak RSS |
|---|---|---|---|
| 381 MB / 160,728 modules | WBA 4.10.2 | **61.4 s** | **1,985 MB** |
| 381 MB / 160,728 modules | node `readFileSync` + `JSON.parse` only | 0.94 s | 919 MB |
| 381 MB / 160,728 modules | **our streaming prototype** | **0.99 s** | **17 MB** |
| 1,049 MB / 441,976 modules | WBA 4.10.2 | **176.3 s** | **1,437 MB** |
| 1,049 MB / 441,976 modules | **our streaming prototype** | **2.78 s** | **59 MB** |
| 25,600 assets, gzip | serial | 2,177 ms | – |
| 25,600 assets, gzip | **rayon parallel** | **342 ms (6.4x)** | 15 MB |

Three things follow, and they are the technical core of this project:

1. **The ceiling, not the parse.** WBA spends 0.94 s reading and parsing a
   381 MB file and the remaining ~60 s building its analysis structures. We
   match the parse speed and remove the ceiling: 17 MB instead of 1,985 MB.
   Nobody should have to raise `--max-old-space-size` to look at a build.
2. **`#492` did not reproduce.** The reported crash "Cannot create a string
   longer than 0x1fffffe8" (1 GB stats) is a V8 string-length limit that has
   moved since the issue was filed; on node 24 the file parses. We therefore
   do **not** claim to fix a crash we could not reproduce — only a memory wall
   we did measure.
3. **Real wins are structural.** rayon on gzip gave 6.4x. That is the shape of
   this project's claims: no per-item process, no V8 heap, actual parallelism.

**Superseded 2026-09-16:** the attribution-tree stage is now implemented (WS-2,
rayon over assets, sizes from disk) and B3/B4 are measured in §2b below. The
table above is kept as the record of the *reference* tool's numbers, which is
what those numbers are: WBA and the readFileSync floor, not us.

## 2b. Our own full-pipeline numbers (WS-2/WS-4/WS-5, measured 2026-09-16)

Same fixtures, but the whole pipeline: ingest, measure every asset, fuse the
source maps, render the report. Peak memory is sampled every 25 ms from the
child process (`bench/harness/measure.ps1`); both working set and private bytes
are recorded because working set includes file-backed pages of the 1 GB stream
and moved between 411 MB and 1,008 MB for the *same* run.

Raw records, committed so they can be inspected rather than taken on trust:

| row | record |
|---|---|
| B3 | `bench/results/b3-full-pipeline-2026-09-16.json` |
| B4 | `bench/results/b4-full-pipeline-2026-09-16.json` |
| B8 | `bench/results/b8-stats-plus-map-2026-09-16.json` |
| SME baseline | `bench/results/ws-s-sme-baseline-2026-09-03.json` |
| WBA ingest | `bench/results/ws1-stats-ingest-2026-09-03.json` |

| benchmark | input | target | before | **now** |
|---|---|---|---|---|
| B1 ingest | 363 MB / 154,379 modules | ≤ 2,000 ms, ≤ 200 MB | 1,139 ms / 126 MB | 1,392 ms / 157 MB |
| B2 ingest | 1,049 MB / 445,602 modules | ≤ 3,000 ms, ≤ 400 MB | 3,318 ms / 346 MB | 4,402 ms / 350 MB |
| B3 **full pipeline** | 363 MB stats + 1,500 assets (71.6 MB on disk) | ≤ 5,000 ms, ≤ 200 MB | 3,052 ms / **670 MB** | **1,869 ms / 126 MB** |
| B4 **full pipeline** | 1,049 MB stats + 1,500 assets | ≤ 15,000 ms, ≤ 400 MB | 8,080 ms / **962 MB** | **6,140 ms / 376 MB** |
| B5 source map | 10k sources, 7.1 MB map | ≤ 1,000 ms | — | 46 ms |
| B5 source map | 50k sources, 36.5 MB map | ≤ 1,000 ms | — | 209 ms / 67 MB |
| B8 **stats + map** | 1 GB stats + 36.5 MB map, one 17.6 MB asset fully covered | < 500 MB | 73,838 ms / 152 MB | **3,573 ms / 137 MB** |

Reference points for the same inputs: WBA 63.6 s / 2,295 MB on the 363 MB
fixture; SME 562,269 ms / 642 MB on the 50k map.

Three bugs, all found by these targets and none of them visible in the ingest
numbers, which is the argument for measuring the pipeline rather than its parts:

1. **A `serde_json::Value` per module.** The detail payload was built as a
   `Value` tree and then serialised: 550 MB of heap for 154,379 modules, on top
   of the graph itself. B3 went 3,052 ms / 670 MB → 1,753 ms / 156 MB when the
   payload was serialised from a borrowed view and streamed to disk.
2. **A "summary" that scaled with input.** `ghost_modules` held one entry per
   unmapped module — 445,602 entries, 47 MB of report payload — plus a hidden
   byte total summed *after* truncation, so it under-reported. Counts and totals
   are now exact and the lists are bounded samples that say what they dropped.
3. **An O(modules x sources) join.** 2.5 billion comparisons on B8: 73.8 s. A
   suffix index built once per asset made it 4.9 s with the same matching
   semantics (a test asserts index and scan agree on the awkward cases).

Plus one measurement bug worth recording: `PeakWorkingSet64` reads 0 after the
process exits under Windows PowerShell 5.1, so the sampler polls instead, and
reports both working set and private bytes rather than picking the flattering
one.

**Still unverified:** emitted file sizes and gzip are measured on the synthetic
1,500-asset fixture, not on a real 25,600-asset production build; and the B8
fixture has one asset, so "1 GB stats + 50 MB map" is a memory-and-join test,
not a claim about a 1,500-asset build with a map for each one.


## 3. The source-map-explorer measurement (WS-S, **done 2026-09-03**)

This was the load-bearing unknown of the whole product, so it went first. Result:
**the answer is "it depends on scale", and that is more useful than a yes/no.**

All numbers below are ours, on the reference machine, peak RSS sampled at
100 ms (`bench/harness/run-bench.ps1`). Full record:
`bench/results/ws-s-sme-baseline-2026-09-03.json`.

| fixture | class | map | bundle | SME 2.5.3 wall | peak RSS |
|---|---|---|---|---|---|
| chalk 6.0.1 | real | 29 KB | 14 KB | 1,645 ms *(cold run)* | 48 MB |
| preact 10.29.8 | real | 97 KB | 35 KB | **252 ms** | 31 MB |
| preact (minified) | real | 97 KB | 13 KB | **237 ms** | 49 MB |
| marked 18.0.14 | real | 177 KB | 84 KB | **224 ms** | 48 MB |
| marked (minified) | real | 177 KB | 45 KB | **226 ms** | 48 MB |
| synthetic, 10k sources | synthetic | 7.1 MB | 3.5 MB | **18,380 / 18,325 / 23,173 ms → median 18,380 ms** | 277 MB |
| synthetic, 50k sources | synthetic | 36.5 MB | 17.6 MB | **562,269 ms (9.4 minutes)** | 642 MB |

**Two findings, and the second one is the product.**

1. **For small real projects there is no pain.** 0.22-0.25 s, ~50 MB, and almost
   all of that is Node start-up plus module loading. The first run of the day
   (1,645 ms) was Defender scanning a cold module cache — the same artefact we
   keep seeing on Windows, and the reason `bench-spec.md` §2 requires saying so
   rather than quietly discarding outliers.
2. **For large maps the pain is real, reproducible and superlinear.** Going from
   10k to 50k sources (5x the data) costs **30.6x the time** (18.4 s → 9.4 min)
   and 2.3x the memory (277 → 642 MB). That matches the upstream report that
   `getWebTreeMapData` dominates at 10k files (#186), and it is consistent with
   the root cause named there: a file-length string is built on every mapping
   iteration.

**Consequences for the project, stated plainly:**

- Stop-the-line condition R1 is **not** triggered for small projects and **is**
  triggered for large ones. So the product case is the large-project segment,
  and the README/roadmap must not imply that small projects are slow today.
- The win mechanism is still a *hypothesis*: SME does not report per-phase
  timings, so we know the total is superlinear but not which of its stages is
  responsible. B5 (our ingest) plus a stage-level profile is the next
  measurement; until then, "faster attribution" is a target, not a claim.
- A 9.4-minute run cost 9.4 minutes; the 3-run median protocol was affordable at
  10k sources and not at 50k. That deviation is recorded in the results file
  rather than glossed over.

Fixtures are built by `bench/fixtures/build/build-fixtures.mjs` with our own
esbuild config (readable and minified variants), pinned by commit in
`bench/fixtures/manifest.json`.

## 4. What we deliberately did not build, and why

| rejected | measured reason |
|---|---|
| "SME is slow, so rewrite it" | pain not yet measured by us; its stagnation is a supply opportunity, not proof of a pain point |
| "vendor WBA's viewer" | ADR-0002: it cannot show fusion data without a fork; we would own a UI we do not control |
| "simd-json" | ADR-0001: it needs the whole document in memory, i.e. the ceiling we are removing |
| "WebGL treemap in Phase 1" | 10k nodes is fine on Canvas 2D; the renderer swap is Phase 2 work behind a stable payload |
| "generic budget/lockfile tool" | searched in the direction phase and killed on demand: a budget-only product has no verified pain |

## 5. Accuracy is not optional

Speed claims are worthless if the output is wrong. Two of the three reference
tools fail in ways we had to discover ourselves:

- `react-docgen` (measured in the search phase) extracts **0 props** from
  components whose props are an imported interface — silently wrong, fast, and
  widely used.
- A naive Rust extraction pipeline reproduced 140k props where the
  type-checker-based tool reported 5.35M. A 38x difference in "correct" output
  between a fast and a slow implementation is the normal case, not an edge case.

Therefore: parity tests against WBA and SME (WS-7) are a release gate, not a
nicety, and the ghost/hidden classifier needs hand-labelled ground truth
(≥95 % agreement) before it ships.

## 6. How to reproduce every number here

```bash
cd bench
./fixtures/fetch.sh                 # real fixtures, pinned by commit
node harness/gen-stats.mjs 381000000 out/stats-400mb.json
node harness/materialize-assets.mjs --stats out/stats-400mb.json --out out/dist
powershell -File harness/measure.ps1 -Binary ../target/release/omnibundle.exe \
    -Target out/dist -Extra '--mode static --report out/report.html' -Runs 3
node harness/parity.mjs --stats fixtures/artifacts/webpack/marked/stats.json \
    --bundle fixtures/artifacts/webpack/marked
node harness/check-i18n.mjs         # docs parity across four languages
```

The B8 fixture needs a *coherent pair* — a stats file whose modules are the map's
sources, which is what `--sources-from` is for:

```bash
node harness/gen-map.mjs 50000 out/map-50k.json 18500087
node harness/gen-bundle.mjs out/map-50k.json 18500087 out/dist/bundle.js
node harness/gen-stats.mjs 1049000000 out/dist/stats.json \
    --assets 1 --asset-name bundle.js --asset-bytes 18500087 \
    --sources-from out/map-50k.json
```

A stats file full of `node_modules/pkgN/...` modules next to a map of
`src/module-N.ts` has 0% coverage *by construction*, and measuring that would
only prove the tool refuses to invent a join (it does, correctly).

`bench-spec.md` §2 defines the protocol (3 runs, median, first run discarded on
a cold cache, same machine), §5 defines the result format. The point of writing
a protocol instead of ad-hoc commands is that the next person can rerun this in
a year and get numbers worth comparing.

## 7. Provenance of the direction search

The search record is maintained outside this repository:

| document | what it holds |
|---|---|
| `docs/lessons-rust-direction-search.md` (parent workspace) | the hard gates, the mistakes, and the meta-lessons that shaped this project's rules |
| `docs/gate3-benchmarks.md` | the WBA / Pygments / hypothesis measurements |
| `docs/directions-round6.md` | jsdom, changesets, PyInstaller, zod, js-yaml, papaparse, chalk, clean-css, nunjucks, xml2js, fs-extra results |
| `docs/mvp-plan-webpack-analyzer.md` | the first 4-week plan, superseded by `03-implementation-plan.md` |

Two of its conclusions are load-bearing here and worth restating: the pain
reported in an issue is often a CI or network cost rather than tool CPU
(changesets' "20 minutes" was CI, not the tool), and a rewrite only makes sense
when the pain is a memory ceiling, a per-item process, or a superlinear
algorithm. OmniBundle has all three.
