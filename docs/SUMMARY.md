# Summary

[OmniBundleScope](index.md)
- [Product requirements](en/00-prd.md)
- [Evidence log](en/01-evidence.md)

# Understand it
- [Architecture](en/02-architecture.md)
- [Implementation plan](en/03-implementation-plan.md)
- [Glossary](en/GLOSSARY.md)

# Prove it
- [Benchmarks and targets](en/04-benchmark-plan.md)
- [Parity and testing](en/05-parity-and-testing.md)
- [Releases and CI](en/06-release-and-ci.md)

# Steer it
- [Risk register](en/07-risk-register.md)
- [Roadmap](en/08-roadmap.md)

# Contracts
- [Unified graph](contracts/unified-graph.md)
- [CLI surface](contracts/cli-surface.md)
- [Report schema](contracts/report-schema.json)
- [Benchmark protocol](contracts/bench-spec.md)
- [i18n parity](contracts/i18n-parity.md)

# Decisions
- [ADR-0001: streaming over simd-json](decisions/ADR-0001-streaming-over-simd-json.md)
- [ADR-0002: self-built report](decisions/ADR-0002-self-built-report.md)
- [ADR-0003: full fusion scope](decisions/ADR-0003-full-fusion-scope.md)
- [ADR-0004: naming](decisions/ADR-0004-naming.md)
- [ADR-0005: fixture matrix](decisions/ADR-0005-fixture-matrix.md)
- [ADR-0006: renaming to omnibundlescope](decisions/ADR-0006-renaming.md)

# Translations

English is the normative source. The translations live in the repository rather
than in this book, because a translated copy of a document that also exists in
English is a second thing to keep correct:

- [中文](zh/README.md) · [日本語](ja/README.md) · [Deutsch](de/README.md)

`node bench/harness/check-i18n.mjs` reports which of them are `pending` and
which have fallen `stale`.