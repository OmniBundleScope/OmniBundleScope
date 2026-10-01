# Contract: CLI surface

Status: **frozen for Phase 1**. Owner: maintainers. The Rust types live in
`crates/omnibundlescope-cli/src/main.rs`; this file is the normative spec.

Design rule: the flags of the two tools we replace keep working, so migrating
is a one-line change in a CI config.

## 1. Invocation

```
omnibundlescope <PATH> [OPTIONS]

  <PATH>                 a dist folder, a stats.json, or a *.map file.
                         A folder is auto-scanned (see §4).
```

Exit codes:

| code | meaning |
|---|---|
| `0` | success, no budget breach |
| `1` | budget breached (`--budget`), or an `OBS0042` invariant violation |
| `2` | usage error (clap) |
| `3` | input unreadable / unsupported schema |

`1` is the CI gate. `3` is reserved for "we could not read your build" and is
deliberately distinct from `1`, so a broken build never looks like a size
regression.

## 2. Options

| flag | values | default | wba / sme compatibility |
|---|---|---|---|
| `-m, --mode` | `static` \| `json` | `static` | `webpack-bundle-analyzer -m` |
| `-r, --report` | path | `omnibundlescope-report.html` | `webpack-bundle-analyzer -r` |
| `-s, --default-sizes` | `stat` \| `parsed` \| `gzip` \| `attributed` | `parsed` | `webpack-bundle-analyzer -s` |
| `-e, --exclude` | regex, repeatable | – | `webpack-bundle-analyzer -e` |
| `-O, --no-open` | flag | off (static never opens a browser) | `webpack-bundle-analyzer -O` |
| `--budget` | path to `omnibundlescope.config.json` | – | new |
| `--json` | flag (shortcut for `--mode json` to stdout) | off | new |
| `--dims` | `source` \| `package` \| `chunk` (repeatable) | all three | new (Phase 2 adds `deps`) |
| `--include-sources` | flag | off | new: embed `sourcesContent` in the HTML for drill-down |
| `--no-fusion` | flag | off | new: escape hatch, report the raw graph |
| `--cache-dir` | path | `.omnibundlescope-cache` | new |
| `-l, --log-level` | `error` \| `warn` \| `info` \| `debug` | `warn` | `webpack-bundle-analyzer -l` |

Deliberate non-goals for Phase 1: no `--serve` (a static single file is the
whole promise; see ADR-0002), no GUI, no plugin system.

## 3. Budget config

```jsonc
{
  "$schema": "{{DOCS_URL}}/schemas/budget-v1.json",
  "limits": [
    { "scope": "total",    "max": 750000 },              // bytes, gzip dimension
    { "scope": "chunk",    "match": "framework", "max": 300000 },
    { "scope": "package",  "match": "react-dom", "max": 130000 }
  ]
}
```

- `max` is in bytes on the dimension reported by the run (printed in the
  summary line so a breach is never ambiguous).
- a `match` that matches nothing is an **error**, not a no-op: a typo in a
  budget rule must not silently pass CI.
- breaches emit `OBS0040` diagnostics and exit `1`.

## 4. Auto-scan rules (zero-config promise)

Given a directory, OmniBundleScope looks for, in this order:

1. `stats.json` / `*.stats.json` (webpack, rspack) — enables the dependency graph
2. `*.map` next to the emitted assets — enables ground-truth attribution
3. `metafile.json` (esbuild) — enables the module graph without source maps
4. `visualizer.json` / `stats-visualizer.json` (rollup-plugin-visualizer)

Ambiguity is resolved by size (largest wins) and reported as a diagnostic, never
silently. A folder with no recognised artifact exits `3` with a message naming
what it looked for.

## 5. Output determinism

- `--mode json` writes the payload from `report-schema.json` to stdout, sorted
  keys, trailing newline.
- `--mode static` writes one self-contained HTML file: payload inlined, no
  network requests, no external assets.
- both honour `--dims` ordering; the summary line always names the size
  dimension used.

## 6. Budget scope for future flags

`--watch` (Phase 2), `--baseline <file>` (Phase 3, trend diffing) and
`--format sarif` (Phase 3) are explicitly reserved names so plugins and CI
configs written against OmniBundleScope 1.x keep parsing.
