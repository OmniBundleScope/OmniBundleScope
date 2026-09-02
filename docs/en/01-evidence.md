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

**Unverified and load-bearing:** the attribution-tree stage (the ~60 s WBA spends
after parsing) is *not* yet reimplemented. B3/B4 in `bench-spec.md` are the
first real test of the "< 5 s for 400 MB" target, and WS-2 owns it.

## 3. The source-map-explorer question (WS-S, in progress)

This is the load-bearing unknown of the whole product, so it is measured first.

From SME's own issues (upstream, not our measurement):

- `#186`: a combined bundle of 10,000 files, directory depth 11 — 45 s inside
  `getWebTreeMapData` alone, versus 3 s for the entire rest of the pipeline.
- `#158` ("Version 2.2 too slow"): 462.79 s on a 5.2 MB angular bundle — but this
  is a 2020 regression that upstream fixed, so it cannot be used as a baseline.
- `#65`: 64.95 % of bytes unmapped in one report.
- Project state: last release v2.5.3 on **2022-09-26**, 3,930 stars, 57 open
  issues. Abandoned-ish, which is opportunity *and* risk (nobody will fix the
  bugs we find).

Root cause named in the issues is allocation, not parsing: building a
file-length string on every mapping iteration. That matters because a Rust
*parser* is only ~1.6x faster than `@babel/parser` on small files (301 ms vs
480 ms for 20k files) — the win has to come from zero-allocation VLQ decoding
and binary search over sorted mappings, not from "Rust parses faster".

**WS-S will replace the issue numbers above with measurements on our own real
fixtures (preact/marked/chalk/dayjs) before any fusion work starts.** If SME
measures under 1 s on realistic maps, the "fusion is fast" premise is dead and
ADR-0003's stop-the-line condition applies.

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
node harness/run-wba.mjs            # baseline: wall + peak RSS per phase
cargo run --release --bin bench-stats -- ../out/stats-400mb.json
node harness/check-i18n.mjs         # docs parity across four languages
```

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
