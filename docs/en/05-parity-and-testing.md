# 05 — Parity and testing

Owner: WS-7 · Translations: [ZH](../zh/05-parity-and-testing.md) ·
[JA](../ja/05-parity-and-testing.md) · [DE](../de/05-parity-and-testing.md)

OmniBundle replaces tools people already trust. "Trust" here means one thing:
**the numbers must match**, or the tool is lying with a nicer UI.

## 1. The two gates

| gate | question | tolerance | blocks |
|---|---|---|---|
| **numeric parity** | do we report the same sizes as WBA / SME on the same input? | ≤ 0.1 %, ordering only | npm release, and any public speed claim |
| **semantic parity** | do we classify the same modules as the bundler intended? | ≥ 95 % agreement with hand labels | ghost / hidden features |

Everything else (layout, colours, interaction) is ours by design (ADR-0002).

## 2. How numeric parity is measured

1. Build a fixture (real repos only — synthetic inputs are for scale).
2. Run `webpack-bundle-analyzer` (or `source-map-explorer`) in `--mode json` /
   `json` output; collect the per-module and per-asset sizes.
3. Run OmniBundle with `--json` on the same input; collect the same fields.
4. Diff on the **join key**, not on row order. Report:
   - `missing`: keys the reference has and we do not;
   - `extra`: keys we have and the reference does not;
   - `delta`: relative difference per key, and the aggregate.
5. Tolerance is applied to the aggregate and to each key, with the two reported
   separately: a report that is 0.05 % off in total but 40 % off on one module is
   a bug that a total-only check would hide.

Known and accepted differences, each declared in the test:

| difference | why it is accepted |
|---|---|
| `gzip` level must be 6 | any other level changes every number; the test pins the level |
| `attributed` sizes are ours alone | no reference tool computes them; parity applies to `stat`/`parsed`/`gzip` |
| module keys for chunks without `identifier` | we use `"{chunk}:{name}"`; the reference uses its own internal key — we compare through a name+chunk projection |
| `initial` flag on chunks | absent in some stats versions; we default to `true` and log `OB0002` when the field is missing |

## 3. Correctness tests that are not parity

| test | what it protects |
|---|---|
| VLQ decode vs a JS reference, same `.map` | mapping count and attributed bytes identical; no silent decode drift |
| join key stability | the same source tree, rebuilt, produces the same keys (the anti-fluff property) |
| size invariant (`check_size_invariant`) | catches join bugs before they reach a user |
| ghost classifier table | `MissingAsset` > `Unmapped` > `EmptyAttribution` precedence, with hand labels |
| determinism | two runs on the same input produce byte-identical JSON |
| fixture replay | the harness re-derives every number in `01-evidence.md` §2 |

## 4. Golden files

`crates/omnibundle-core/tests/golden/` holds one JSON per fixture. They are
regenerated with `--update-goldens` and reviewed like code: a diff in a golden
file is either a deliberate, explained change or a bug. No blanket re-record.

Golden files are *never* taken from a real user's private build. Fixtures are
public repos (ADR-0005) and synthetic generators.

## 5. Test layering

| layer | what runs | when |
|---|---|---|
| unit | pure functions: join keys, VLQ, gzip, invariant, classifier | every PR |
| property | fuzz-ish: random stats shapes, random maps, invariants must hold | every PR, cheap seeds |
| parity | real fixtures vs the reference tools | every PR that touches ingest/fusion |
| benchmark | synthetic scale, memory ceilings | every PR touching a measured path; gates on memory, publishes timings |
| smoke | `npx omnibundle` on a tiny real repo end-to-end | every PR, and on release tags |

## 6. Fuzzing (Phase 2, but the hooks are already there)

Malformed build metadata is the norm in the wild, and a build tool must never
panic on it. `cargo fuzz` targets are planned for the VLQ decoder and the stats
seed; until then the CLI maps every parse error to a diagnostic and exit code 3,
never a panic (`core::Error`, no `unwrap` on user input).
