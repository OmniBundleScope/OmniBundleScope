# 07 — Risk register

Owner: WS-0 · Translations: [ZH](../zh/07-risk-register.md) ·
[JA](../ja/07-risk-register.md) · [DE](../de/07-risk-register.md)

Each risk has a trigger, a mitigation and a named lane. A risk without a
measurable trigger is a worry, not a risk, and gets deleted at the next review.

| id | risk | trigger (measurable) | mitigation | owner |
|---|---|---|---|---|
| R1 | **SME has no real pain at realistic scale**, so "fusion is fast" is void | WS-S measures SME < 1 s on real fixtures | stop the line (ADR-0003 #1); reposition around cross-bundler insight, where the value does not depend on SME's speed | WS-S |
| R2 | **The attribution tree is the real cost**, not the parse (WBA spent ~60 s there) | WS-2 profiling shows > 3 s in attribution at 400 MB | parallelise per module; if still short of B3, cut the performance claim rather than the honesty | WS-2 |
| R3 | **Join keys are wrong** → fusion produces confident nonsense | `check_size_invariant` fails, or hand labels disagree > 5 % | invariants in `model.rs` gate every write; parity harness; no "close enough" tolerance above 0.1 % | WS-4 |
| R4 | **Ghost/hidden detection misfires** on real projects | agreement < 95 % on hand labels | classifier is a small pure function with a table test; ship it off by default until it is right | WS-4 |
| R5 | **Wide scope across many workstreams** → contract drift, merge conflicts | a PR edits another lane's path, or two lanes disagree on the schema | `OWNERS.md` lane rules; WS-0 arbitrates; one ADR per schema change | WS-0 |
| R6 | **Numeric regressions shipped in a patch release** | a release changes reported numbers without a version bump | §3 of the release doc: number changes are minor/major; parity gate on release candidates | WS-8 |
| R7 | **Reference tools change or vanish** | WBA/SME publish a new version, or SME is deleted | parity targets pinned by version; the harness keeps working against a pinned copy; their fixes are re-measured, not re-implemented blindly | WS-7 |
| R8 | **Fixture drift** — upstream repos change their build, so "our" fixtures stop being comparable | manifest commit no longer builds, or artifact hashes change | manifests pin commits; the fixture build config is ours where it matters (ADR-0005) | WS-A |
| R9 | **Path normalisation fails on Windows** (`\`, drive letters, `file://` URLs) | parity diff > 0.1 % on Windows only | normalisation rules are in the contract; Windows is a first-class platform; CI parity runs on all three OSes | WS-4, WS-7 |
| R10 | **The report becomes the bottleneck** (10k nodes, Canvas 2D) | B10 missed: first paint > 2 s or interaction < 30 fps | virtualise the treemap, render only the viewport, down-sample; WebGL stays the Phase 2 answer | WS-5 |
| R11 | **The name is squatted** before we publish | npm/crates lookup returns something else at publish time | re-check at publish time; fallback `omnibundle-cli` / `omnibundle-rs` already decided in ADR-0004 | WS-8 |
| R12 | **Benchmarks become unfalsifiable** (a lane quotes a number it did not measure) | a benchmark table entry without a `bench/results/` artefact | "unverified" is an allowed, expected state; review rejects measured claims with no artefact | WS-0, WS-A |
| R13 | **Four-language docs drift** and mislead users | `check-i18n.mjs` fails, or a translation is stale | header stamps with EN revision; stale blocks release; numbers and commands must be verbatim | WS-9 |
| R14 | **Burnout of the "one tool" promise** — we promise every bundler and ship three | Phase 3 slips while the README claims universality | the README states the supported input matrix explicitly and grows with the code, not ahead of it | WS-9, WS-0 |

## The two risks we would accept if we had to

- **R1** is the one that could end the project as originally conceived. We would
  rather find out in the first measurement than after the fusion engine.
- **R2** is the one most likely to cost the most time, because it is the part
  nobody has written yet. It is also the part that decides whether the "< 5 s"
  claim survives.
