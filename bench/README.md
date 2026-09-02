# Bench harness (WS-A)

Everything needed to reproduce every number in `docs/en/01-evidence.md`.

```bash
cd bench
npm install                 # reference tools: webpack-bundle-analyzer, source-map-explorer
./fixtures/fetch.sh         # or .\fixtures\fetch.ps1 — real fixtures, pinned by commit

# synthetic scale inputs
node harness/gen-stats.mjs 381000000 /tmp/stats-400mb.json
node harness/gen-stats.mjs 1100000000 /tmp/stats-1gb.json
node harness/gen-map.mjs 10000 /tmp/map-10k-sources.json 8000000

# reference baselines
node harness/run-tool.mjs --tool wba  --input /tmp/stats-400mb.json --class synthetic
node harness/run-tool.mjs --tool node --input /tmp/stats-400mb.json --class synthetic
node harness/run-tool.mjs --tool sme  --input /tmp/map-10k-sources.json --class synthetic

# docs parity
node harness/check-i18n.mjs
```

Results land in `results/<label>-<timestamp>.json` in the shape defined by
`docs/contracts/bench-spec.md` §5. `phase_ms: null` is a valid result: it
records that a phase breakdown was not measured, which is exactly the honesty
rule the project depends on.

## Protocol

`docs/contracts/bench-spec.md` §2: 3 runs, median, first run discarded on a cold
cache (and said so), same machine, nothing else running. Peak RSS is sampled at
100 ms — on Windows, run the child through `run-bench.ps1` because Node cannot
read another process's RSS cheaply.

## Two fixture classes, never mixed

- **real** — preact, marked, chalk, dayjs (+ six smoke repos). Correctness,
  parity, CI.
- **synthetic** — generated above. Scale, memory ceilings, regression floors.

Every table that reports numbers says which class produced them.

## Layout

```
bench/
  fixtures/manifest.json   pinned repos, licenses, measured baselines
  fixtures/fetch.{sh,ps1}  clone + pin
  fixtures/build/          per-fixture build configs (webpack for preact, …)
  harness/gen-stats.mjs    synthetic stats.json generator
  harness/gen-map.mjs      synthetic source map generator
  harness/run-tool.mjs     reference-tool runner
  harness/check-i18n.mjs   docs parity checker
  results/                 one JSON per run (gitignored)
```
