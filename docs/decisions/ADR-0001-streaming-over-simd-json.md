# ADR-0001: Streaming JSON ingest instead of simd-json

Status: **accepted** (2026-09-02) · Owner: maintainers · Affects: stats ingest, source maps

## Context

The original PRD specified `simd-json` for fast JSON parsing. A prototype was
built first, with two candidate approaches:

1. `simd-json` — SIMD-accelerated parsing, ~2-3x faster than `serde_json` on
   large documents, but it **mutates the input buffer in place and requires the
   whole document in memory** as a `&mut [u8]`.
2. `serde_json` with a custom `DeserializeSeed` — deserialises the huge
   `modules` array element by element, never materialising the document.

## Decision

Use approach 2.

## Evidence

Measured on the reference machine against `webpack-bundle-analyzer` 4.10.2
inputs (`docs/en/01-evidence.md`):

| input | approach 2 (streaming seed) | peak RSS |
|---|---|---|
| stats 381 MB / 160,728 modules | 0.99 s | **17 MB** |
| stats 1,049 MB / 441,976 modules | 2.78 s | **59 MB** |

A `simd-json` implementation would have to hold the entire 1 GB document
(plus its in-place copy) before it could start, which puts the floor at
~2 GB — precisely the failure mode OmniBundleScope exists to remove. Node's own
`JSON.parse` floor on the same file was 0.94 s at **919 MB RSS**; we match its
speed at 1/50th the memory, and the memory is the thing that scales badly.

## Consequences

- The document is never a `serde_json::Value`; every module is deserialised
  straight into the graph.
- No SIMD speedup on the parse itself. We are not claiming one: the win is
  memory ceiling and streaming, not raw parse throughput.
- `simd-json` stays on the table for a *future* different problem (small files
  where the document genuinely fits in memory and SIMD matters).
