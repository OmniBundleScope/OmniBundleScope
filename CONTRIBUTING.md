# Contributing to FastScope

Thanks for looking at this. The bar is deliberately specific, because the
project's whole premise is that its claims are measurements:

> **Every change ships with a measurement, or says why it does not need one.**

If you improve the parser, show the before/after on a fixture. If you change the
report, show the file size and generation time. If you fix a bug, add the test
that would have caught it. A change that cannot be measured either gets a
measurement or gets an argument in the PR for why it obviously cannot.

## Setup

```bash
git clone {{REPO_URL}}
cd fastscope
cargo build --release
cargo test --workspace
```

Rust 1.90+ (see `rust-toolchain.toml`). Node 18+ only for the benchmark and npm
harness; the Rust build needs no Node.

## The gates your PR has to pass

CI runs all of these, on Linux, Windows and macOS:

| gate | command | why it is not negotiable |
|---|---|---|
| tests | `cargo test --workspace` | the parity tests are the accuracy argument |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | the workspace sets `clippy::pedantic`; this is where the `u64 as i64` wrap was found |
| fmt | `cargo fmt --all --check` | diff noise is review noise |
| parity | `node harness/parity.mjs` on real webpack builds | a wrong number is worse than no number |
| i18n | `node harness/check-i18n.mjs` | four languages must not drift apart |

Run them locally before pushing:

```bash
cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cd bench && npm ci && node fixtures/build/build-webpack.mjs marked && node harness/parity.mjs --stats fixtures/artifacts/webpack/marked/stats.json --bundle fixtures/artifacts/webpack/marked
```

## Filling in the URLs

The URLs this project does not control yet - the repository, the npm page, the
crates.io pages, the docs site - are declared once, in [`repo-links.json`](repo-links.json),
and appear in the files as double-brace tokens. Nothing anywhere hardcodes them.

```bash
node bench/harness/links.mjs            # what is still empty, and where each token appears
# edit repo-links.json: fill in `value`
node bench/harness/links.mjs --apply    # substitute into every tracked file
```

To apply without ending up with `https://github.com/github.com`, fill in the
values before applying rather than editing the 16 files by hand.

Two things are checked on top of the substitution:

- `NPM_PACKAGE` and `CRATES_CORE_PACKAGE` must match what the manifests actually
  declare, so the badge cannot point at a package that does not exist under that name.
- While anything is still empty, the checker fails if a URL for the
  `fastscope/fastscope` repository has been hardcoded anywhere. Once the table
  is filled, that check switches off: those strings can legitimately be correct
  once someone owns that org or domain, and a gate that objects to a deliberate
  choice is worse than no gate.

CI prints the outstanding placeholders on every build but does not fail on them.
The release workflow runs the same script with `--deny`, because a release
publishes a README that cannot be fixed afterwards without a new version.

## Where things live

Contracts are frozen first, and every path has one owning lane
([docs/contracts/OWNERS.md](docs/contracts/OWNERS.md)). If your change touches
the unified graph, the report schema, the CLI surface or the benchmark
protocol, it needs an ADR, not just a good commit message.

| workstream | owns | module |
|---|---|---|
| WS-1 | `stats.json` / `metafile.json` ingest | `crates/fastscope-core/src/stats` |
| WS-2 | size attribution (stat / parsed / gzip) | `.../src/sizes` |
| WS-3 | source map v3 parse + attribute | `.../src/sourcemap` |
| WS-4 | the fusion join, ghost/hidden | `.../src/fusion` |
| WS-5 | report payload + HTML shell | `.../src/report`, `assets/report` |
| WS-6 | CLI surface, budget gate | `crates/fastscope-cli` |
| WS-7 | parity harness | `bench/harness` |
| WS-8 | CI, releases, packaging | `.github`, `npm` |

The core crate performs **no IO** and has no async runtime. That is what keeps
the WASM target a packaging change instead of a rewrite, so a PR that adds
`std::fs` to the core will be asked to move it to a `*_from_*` constructor.

## Honest numbers

- A benchmark target with no measurement stays `unverified`. Nobody fills it in
  from a previous number, a different machine, or an impression.
- Real fixtures (`preact`, `marked`, `chalk`, `dayjs`) and synthetic fixtures
  live in separate tables and are never mixed.
- If you cannot reproduce a published number, open an issue before you change
  the number. The docs record the machine, the protocol and the raw results
  files for exactly this reason.

## Commit messages

Conventional-ish, with the reason:

```
perf(fusion): index the path suffixes instead of scanning every source

The join was O(modules x sources): 2.5 billion comparisons on the B8 fixture,
73.8 s. A suffix index built once per asset makes it 4.9 s with identical
matching semantics (a test asserts the index and the scan agree).
```

## Reporting a benchmark discrepancy

Open an issue with the fixture class, the command, and the machine. If you have
the raw output, attach it. Disagreements are treated as bugs in the tool until
proven otherwise — that is the only rule that keeps a benchmark meaningful.

## Code of conduct

[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Security issues:
[SECURITY.md](SECURITY.md) — not the public issue tracker.

## Licence

MIT or Apache-2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).