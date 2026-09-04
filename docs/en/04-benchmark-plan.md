# 04 — Benchmark plan

Owner: WS-A (protocol) · Translations: [ZH](../zh/04-benchmark-plan.md) ·
[JA](../ja/04-benchmark-plan.md) · [DE](../de/04-benchmark-plan.md)

Normative protocol: `docs/contracts/bench-spec.md`. This file is the **status
board**: every target, its current number or an explicit `unverified`, and the
lane that owns it.

Rules that are not negotiable:

- a target without a measured number stays `unverified`; nobody fills it in from
  an upstream issue or from expectation;
- real fixtures and synthetic fixtures never share a table;
- a missed target is recorded as missed, and changing a target needs an ADR.

## 1. Targets

| id | scenario | fixture class | target | measured | owner |
|----|----------|---------------|--------|----------|-------|
| B1 | stats 363 MB / 154,379 modules, ingest | synthetic | ≤ 2 s, ≤ 200 MB | **1,334 ms / 126 MB** (ingest 1,139 ms) | WS-1 ✅ |
| B2 | stats 1,049 MB / 445,602 modules, ingest | synthetic | ≤ 3 s, ≤ 400 MB | **3,628 ms / 346 MB** — memory passes, wall **misses by 0.6 s** | WS-1 ⚠️ |
| B3 | 400 MB full pipeline | synthetic | ≤ 5 s, ≤ 200 MB | unverified | WS-2 |
| B4 | 1 GB full pipeline | synthetic | ≤ 15 s, ≤ 400 MB | unverified | WS-2 |
| B5 | `.map` 50 MB / 10k sources, parse + attribute | real + synthetic | ≤ 1 s | unverified | WS-3 |
| B6 | `source-map-explorer` baseline on the same `.map` | real + synthetic | any number, must exist | **0.22-0.25 s (small real), 18.4 s median (10k sources), 562 s (50k sources); 31-642 MB** | **WS-S ✅** |
| B7 | gzip 25,600 assets | synthetic | — | **2,177 ms → 342 ms (6.4x)** | WS-2 |
| B8 | fusion memory, 1 GB stats + 50 MB map | synthetic | < 500 MB peak | unverified | WS-4 |
| B9 | parity diff vs WBA / SME | real | ≤ 0.1 %, ordering only | unverified | WS-7 |
| B10 | report: 10k modules, first paint / interaction | real | < 2 s / > 30 fps | unverified | WS-5 |

## 2. Reference baselines (measured, reference machine)

| tool | input | wall | peak RSS | fixture |
|---|---|---|---|---|
| **omnibundle 0.1.0 (release)** | **stats 363 MB / 154,379 modules** | **1.33 s** | **126 MB** | synthetic |
| **omnibundle 0.1.0 (release)** | **stats 1,049 MB / 445,602 modules** | **3.63 s** | **346 MB** | synthetic |
| webpack-bundle-analyzer 4.10.2 | stats 363 MB (same file) | 63.6 s | 2,295 MB | synthetic |
| webpack-bundle-analyzer 4.10.2 | stats 1,049 MB | 176.3 s | 1,437 MB | synthetic |
| node `readFileSync` + `JSON.parse` | stats 381 MB | 0.94 s | 919 MB | synthetic |
| source-map-explorer 2.5.3 | preact 97 KB map | 0.25 s | 31 MB | real |
| source-map-explorer 2.5.3 | marked 177 KB map | 0.22 s | 48 MB | real |
| source-map-explorer 2.5.3 | 10k sources, 7.1 MB map | **18.4 s** (median of 3) | 277 MB | synthetic |
| source-map-explorer 2.5.3 | 50k sources, 36.5 MB map | **562.3 s** | 642 MB | synthetic |

On identical input the shipped binary is **47.7x faster and 18.2x lighter** than
`webpack-bundle-analyzer`, and it is the same binary users run — `--bench` exists
so a release claim can be produced by the product rather than by a prototype.

**A regression the harness caught, recorded because it is instructive:** the
first CLI did `std::fs::read` before parsing, which put the *file size* on the
memory floor — 1 GB input became 6.4 GB resident and 480 s. The fix was to
stream from a 1 MiB `BufReader` (`core::stats::ingest_file`) and to use a 512 KiB
head read only for sniffing the artifact type. The memory story is a property of
the IO path, not of the parser; a benchmark that measured only parse time would
never have seen it. Details: `bench/results/ws1-stats-ingest-2026-09-03.json`.

The 5x-data / 30.6x-time ratio between the two source-map rows is the other
signal worth watching: SME's attribution is superlinear, while small projects
pay only process start-up.

The "Cannot create a string longer than 0x1fffffe8" crash from WBA issue #492
did **not** reproduce on node 24 — V8's string ceiling moved. We claim the
memory ceiling, not the crash.

## 3. Cross-language calibration (measured while choosing the architecture)

These numbers are not targets; they exist so nobody re-derives them. They are
also the reason the project's claims are modest: real Rust wins are 4-19x, not
100x.

| comparison | JS | Rust | ratio |
|---|---|---|---|
| template render, 20k rows (identical output bytes) | nunjucks 203,510 rows/s | minijinja 840,282 rows/s | 4.1x |
| XML DOM build (identical element count) | xml2js 8.73 MB/s | roxmltree 166.1 MB/s | 19x |
| copy 30,000 small files | fs-extra 19.09 s | walkdir + rayon 4.82 s | 4.0x |
| HTML parse to DOM | jsdom 4.4 MB/s | kuchiki (html5ever) 30.8 MB/s | 7x |
| source map parse, 20k small files | @babel/parser 480 ms | oxc-parser 301 ms | 1.6x |
| shrink-loop traversal, 364 nodes | CPython 3.12 33,694 visits/s | Rust 2,570,694 visits/s | 76x |

The last two rows are the honest counterweight: a Rust *parser* is barely faster
than a JS one. Our wins come from allocation strategy (source maps), from
parallelism (gzip, copies, attribution) and from the absence of a heap ceiling
(stats).

## 4. Fixture matrix

Real (fetched, pinned, permissively licensed, see ADR-0005):

| fixture | build | feeds |
|---|---|---|
| preact 10.29.8 (MIT) | our webpack config → `stats.json` + `.map` | WBA path, fusion, parity |
| marked 18.0.14 (MIT) | upstream esbuild config → `metafile.json` + `.map` | esbuild path |
| chalk 6.0.1 (MIT) | `tsc --sourceMap --declaration` | tiny-scale sanity, tsc path |
| dayjs 1.11.23 (MIT) | upstream babel/rollup build → `.map` | rollup path |
| p-limit, nanoid, ms, mitt, ufo, h3 (MIT) | upstream | CI smoke tests |

Synthetic (generated, for scale only): stats 381 MB and 1,049 MB; `.map` with
10k+ sources; 25,600 tiny assets for the gzip stage.

## 5. What we publish

Every release ships a `benchmarks` table generated from `bench/results/`, with
the fixture class, the tool version and the machine. If a number is not
reproducible from the committed harness, it does not go in the table — including
the ones we would like to be true.

## 6. New finding: the payload is now the bottleneck (WS-2 / WS-5 input)

Running the shipped binary end to end on the 363 MB fixture:

| phase | time |
|---|---|
| ingest (parse + graph) | 1,051 ms |
| report generation (serialise payload, inline shell, write) | ~24 s |
| output size | **125.6 MB** single HTML file, 154,379 modules |

The ingest goal is met and the report goal is not: a report nobody can open is
not a report. This is now the leading constraint on B3/B4/B10, and the fix is
architectural rather than a micro-optimisation:

- the treemap payload must be a **tree of group nodes with sizes only**, not a
  dump of every module (a package-level treemap needs the top-N groups, and a
  drill-down needs the child list of the *selected* node only);
- module-level detail (reasons, sources, per-dimension sizes) should be fetched
  on demand from a companion JSON, with the HTML holding the tree;
- the "single self-contained file" promise needs revisiting for very large
  graphs, or the promise needs a documented size ceiling above which we emit
  `report.html` + `report.data.json` and say so in the summary line.

WS-2 owns the measurement of the corrected approach; WS-5 owns the payload
shape. Until then `--bench` is the honest path for large inputs: it measures the
part we have actually optimised.

