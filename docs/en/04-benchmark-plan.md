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
| B1 | stats 381 MB / 160,728 modules, parse | synthetic | ≤ 2 s, ≤ 200 MB | **0.99 s / 17 MB** (prototype) | WS-1 |
| B2 | stats 1,049 MB / 441,976 modules, parse | synthetic | ≤ 3 s, ≤ 400 MB | **2.78 s / 59 MB** (prototype) | WS-1 |
| B3 | 400 MB full pipeline | synthetic | ≤ 5 s, ≤ 200 MB | unverified | WS-2 |
| B4 | 1 GB full pipeline | synthetic | ≤ 15 s, ≤ 400 MB | unverified | WS-2 |
| B5 | `.map` 50 MB / 10k sources, parse + attribute | real + synthetic | ≤ 1 s | unverified | WS-3 |
| B6 | `source-map-explorer` baseline on the same `.map` | real | any number, must exist | unverified | **WS-S** |
| B7 | gzip 25,600 assets | synthetic | — | **2,177 ms → 342 ms (6.4x)** | WS-2 |
| B8 | fusion memory, 1 GB stats + 50 MB map | synthetic | < 500 MB peak | unverified | WS-4 |
| B9 | parity diff vs WBA / SME | real | ≤ 0.1 %, ordering only | unverified | WS-7 |
| B10 | report: 10k modules, first paint / interaction | real | < 2 s / > 30 fps | unverified | WS-5 |

## 2. Reference baselines (measured, reference machine)

| tool | input | wall | peak RSS | fixture |
|---|---|---|---|---|
| webpack-bundle-analyzer 4.10.2 | stats 381 MB | 61.4 s | 1,985 MB | synthetic |
| webpack-bundle-analyzer 4.10.2 | stats 1,049 MB | 176.3 s | 1,437 MB | synthetic |
| node `readFileSync` + `JSON.parse` | stats 381 MB | 0.94 s | 919 MB | synthetic |
| OmniBundle streaming prototype | stats 381 MB | 0.99 s | 17 MB | synthetic |
| OmniBundle streaming prototype | stats 1,049 MB | 2.78 s | 59 MB | synthetic |

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
