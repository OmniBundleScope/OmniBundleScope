# 07 — Risk register

Translations: per-document translations are still in progress. ZH: docs/zh/README.md · JA: docs/ja/README.md · DE: docs/de/README.md

Each risk has a trigger, a mitigation and a named owner. A risk without a
measurable trigger is a worry, not a risk, and gets deleted at the next review.

| id | risk | trigger (measurable) | mitigation | owner |
|---|---|---|---|---|
| R1 | **SME has no real pain at realistic scale**, so "fusion is fast" is void | the source-map-explorer baseline measures SME < 1 s on real fixtures | stop the line (ADR-0003 #1); reposition around cross-bundler insight, where the value does not depend on SME's speed | the source-map-explorer baseline |
| R2 | **The attribution tree is the real cost**, not the parse (WBA spent ~60 s there) | sizes profiling shows > 3 s in attribution at 400 MB | parallelise per module; if still short of B3, cut the performance claim rather than the honesty | sizes |
| R3 | **Join keys are wrong** → fusion produces confident nonsense | `check_size_invariant` fails, or hand labels disagree > 5 % | invariants in `model.rs` gate every write; parity harness; no "close enough" tolerance above 0.1 % | fusion |
| R4 | **Ghost/hidden detection misfires** on real projects | agreement < 95 % on hand labels | classifier is a small pure function with a table test; ship it off by default until it is right | fusion |
| R5 | **Wide scope across many areas** — contract drift, merge conflicts | a change touches an area it does not own, or two changes disagree on the schema | the contracts are frozen; one ADR per schema change, reviewed before merge | contracts |
| R6 | **Numeric regressions shipped in a patch release** | a release changes reported numbers without a version bump | §3 of the release doc: number changes are minor/major; parity gate on release candidates | distribution and CI |
| R7 | **Reference tools change or vanish** | WBA/SME publish a new version, or SME is deleted | parity targets pinned by version; the harness keeps working against a pinned copy; their fixes are re-measured, not re-implemented blindly | parity |
| R8 | **Fixture drift** — upstream repos change their build, so "our" fixtures stop being comparable | manifest commit no longer builds, or artifact hashes change | manifests pin commits; the fixture build config is ours where it matters (ADR-0005) | benchmarks |
| R9 | **Path normalisation fails on Windows** (`\`, drive letters, `file://` URLs) | parity diff > 0.1 % on Windows only | normalisation rules are in the contract; Windows is a first-class platform; CI parity runs on all three OSes | fusion, parity |
| R10 | **The report becomes the bottleneck** (10k nodes, Canvas 2D) | B10 missed: first paint > 2 s or interaction < 30 fps | virtualise the treemap, render only the viewport, down-sample; WebGL stays the Phase 2 answer | report |
| R11 | **The name is squatted** before we publish | npm/crates lookup returns something else at publish time | re-check at publish time; fallback `omnibundlescope-cli` / `omnibundlescope-rs` already decided in ADR-0004 | distribution and CI |
| R12 | **Benchmarks become unfalsifiable** (a number is quoted that nobody measured) | a benchmark table entry without a `bench/results/` artefact | "unverified" is an allowed, expected state; review rejects measured claims with no artefact | contracts, benchmarks |
| R13 | **Four-language docs drift** and mislead users | `check-i18n.mjs` fails, or a translation is stale | header stamps with EN revision; stale blocks release; numbers and commands must be verbatim | docs and translations |
| R14 | **Burnout of the "one tool" promise** — we promise every bundler and ship three | Phase 3 slips while the README claims universality | the README states the supported input matrix explicitly and grows with the code, not ahead of it | docs and translations, contracts |

## The two risks we would accept if we had to

- **R1** is the one that could end the project as originally conceived. We would
  rather find out in the first measurement than after the fusion engine.
- **R2** is the one most likely to cost the most time, because it is the part
  nobody has written yet. It is also the part that decides whether the "< 5 s"
  claim survives.
