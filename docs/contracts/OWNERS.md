# Ownership map (anti-collision contract)

Several workstreams run in this repository at the same time. The rules below
are what keeps that from turning into merge conflicts and contract drift. They
are binding: a change that edits outside its lane should be blocked in review.

## 1. Path ownership

| lane | owns (exclusive) | must not touch |
|---|---|---|
| **WS-0** contracts | `docs/contracts/**`, `docs/decisions/**`, `crates/omnibundle-core/src/model.rs`, `crates/omnibundle-core/src/error.rs` | anything else |
| **WS-A** bench | `bench/**`, `.github/workflows/bench.yml` | `crates/**` |
| **WS-S** SME baseline | `docs/en/01-evidence.md` (§ SME), `bench/baselines/**` | other evidence sections |
| **WS-1** stats ingest | `crates/omnibundle-core/src/stats/**` | `model.rs`, other modules |
| **WS-2** sizes | `crates/omnibundle-core/src/sizes/**` | ditto |
| **WS-3** source maps | `crates/omnibundle-core/src/sourcemap/**` | ditto |
| **WS-4** fusion | `crates/omnibundle-core/src/fusion/**` | ditto |
| **WS-5** report | `crates/omnibundle-core/src/report/**`, `assets/report/**` | ditto |
| **WS-6** CLI | `crates/omnibundle-cli/**` | core modules (raises a change request to WS-0 instead) |
| **WS-7** parity | `crates/omnibundle-core/tests/parity/**`, `crates/omnibundle-cli/tests/**` | production code (fixtures only) |
| **WS-8** dist & CI | `.github/**` (except `bench.yml`), `crates/omnibundle-wasm/**`, `packaging/**` | core + CLI |
| **WS-9** docs/i18n | `docs/{en,zh,ja,de}/**`, `README*.md` | `crates/**`, `bench/**` |

A change request across lanes is a PR against the owning lane's path plus a
one-line note in the PR body. Nobody edits another lane's files to "just fix a
typo".

## 2. Dependency order (what may start when)

```
WS-0 contracts ──┬─▶ WS-1 stats ──┐
                 ├─▶ WS-3 maps ───┼─▶ WS-4 fusion ─▶ WS-6 CLI ─▶ WS-8 dist/CI
                 └─▶ WS-5 report ─┘        │
                                          └─▶ WS-7 parity
WS-A bench  ─────────────────────────────────▶ (gates B1-B10 for everyone)
WS-9 docs  ─────────────────────────────────▶ (independent from day one)
```

WS-A, WS-0, WS-9 and WS-S can run from the first commit. WS-1/3/5 need the
frozen contracts. WS-4 needs at least one real ingest. WS-6 needs the fusion
engine to be useful.

## 3. Definition of done (every lane, same bar)

1. `cargo test --workspace` green, `cargo clippy --all-targets` no errors.
2. At least one test that would fail if the lane's core claim were false.
3. If the lane touches a measured path: a bench run appended to
   `bench/results/` and the relevant row in `04-benchmark-plan.md` updated with
   a measured number or an explicit "unverified".
4. If the lane changes the payload: `report-schema.json` **and** `model.rs`
   updated in the same PR, with an ADR.
5. Doc comments on non-obvious code, in English.

## 4. Conflict protocol

- Two lanes need the same contract change: WS-0 decides within one round; if
  undecided, the **larger blast radius wins** (payload schema > internal
  signature), and the decision goes into `decisions/`.
- Numbers disagree: re-run the benchmark, do not average. See
  `bench-spec.md` §2.
- A lane discovers its task is impossible (e.g. a target cannot be met): stop,
  record the measurement, and escalate to the stop-the-line conditions in
  `03-implementation-plan.md` §7. Do not quietly redefine the target.
