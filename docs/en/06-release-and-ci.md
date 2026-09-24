# 06 — Release, distribution and CI

Owner: WS-8 · Translations: Translations: per-document translations are still in progress. [ZH](../zh/README.md) · [JA](../ja/README.md) · [DE](../de/README.md)

## 1. Artefacts

| artefact | channel | notes |
|---|---|---|
| `omnibundle-core` | crates.io | published first; proves the build is real |
| static binaries | GitHub releases | windows x64, macOS x64 + arm64, linux x64 + arm64 |
| `omnibundle` (npm) | npm | thin wrapper: resolves the platform, downloads the binary, execs it |
| `omnibundle-wasm` | npm (Phase 2) | `@omnibundle/wasm`, browser bundle |

No native binaries inside npm tarballs: a 200 MB install is how a CLI loses its
users, and the wrapper costs about 40 KB.

## 2. Publication order and why

1. tag `vX.Y.Z` → release workflow builds and uploads binaries;
2. `cargo publish` the core crate (the crate version is the source of truth);
3. publish the npm wrapper pointing at the release URL.

The npm name is the scarcest resource here (ADR-0004), so it is spent last, on a
package that already has a working binary behind it.

## 3. Versioning

Semantic versioning, with one rule that matters for a build tool: **a change in
the reported numbers is a major or minor change, never a patch.** Users pin
versions in CI; a patch that silently re-attributes 2 % of their bytes would be
a betrayal. `schema_version` in the payload moves with any breaking shape
change, independently of the crate version.

## 4. CI pipelines

| workflow | triggers | gates |
|---|---|---|
| `ci.yml` | every PR | `cargo fmt --check`, `clippy -D warnings` on the measured paths, `cargo test --workspace`, `check-i18n.mjs`, smoke run |
| `bench.yml` | PRs touching `core/src/{stats,sizes,fusion,sourcemap}` | memory ceilings (B1/B2/B8 style) and parity (B9); timings published as an artefact, not gated — runners are noisy |
| `parity.yml` | PRs touching ingest/fusion, and release candidates | full parity across all real fixtures, ≤ 0.1 % |
| `release.yml` | `v*` tags, manual | build matrix, checksums, publish order §2 |
| `docs.yml` | PRs touching `docs/**` | i18n parity, link check, glossary conformance |

`clippy -D warnings` is enforced on `omnibundle-core` only. The CLI and WASM
crates get warnings, because a pedantic lint storm in a wrapper is not where our
attention belongs.

## 5. Platform support policy

| platform | support | why |
|---|---|---|
| linux x64/arm64, macOS x64/arm64 | first class | CI + release matrix |
| windows x64 | first class | the reference machine is Windows; a tool that cannot verify its own benchmarks on Windows is not finished |
| musl, armv7, s390x | best effort | via cargo-dist, not gated |
| wasm32-unknown-unknown | Phase 2 | depends on `omnibundle-wasm` |

## 6. Reproducible releases

`-C strip=symbols`, `--locked`, pinned toolchain from `rust-toolchain.toml`, and
SHA-256 checksums published with every binary. The npm wrapper verifies the
checksum before exec'ing; a corrupted download is an error, not a mystery.

## 7. Support policy for the parity targets

`webpack-bundle-analyzer` and `source-map-explorer` are moving or frozen
upstream (SME has not shipped since 2022-09-26). Parity therefore is defined
against **pinned versions** recorded in `bench/fixtures/manifest.json`, not
against "whatever is latest". When a reference tool changes, the bump is a
deliberate PR that shows the numeric diff — which is also how we learn whether
they fixed something we were going to reimplement.
