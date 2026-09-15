//! `OmniBundle` core: stats ingest, size attribution, source-map parsing, the
//! fusion engine and the report payload.
//!
//! Hard constraint: this crate performs **no IO** and pulls in no CLI/async
//! runtime. Every byte enters through [`model`] or through the `*_from_*`
//! constructors of the ingest modules. That is what makes the WASM target in
//! Phase 2 a packaging change instead of a rewrite.
//!
//! Module ownership (see `docs/contracts/OWNERS.md`):
//! - `stats`     WS-1 webpack/rspack `stats.json` streaming ingest
//! - `sourcemap` WS-3 source map v3 ingest (zero-allocation VLQ)
//! - `sizes`     WS-2 size attribution (stat / parsed / gzip) + attribution tree
//! - `fusion`    WS-4 cross-artifact fusion and ghost/hidden code detection
//! - `report`    WS-5 report payload (the HTML shell lives in `assets/report`)
//! - `model`     WS-0 contract-owned: the unified graph. Changes need an ADR.

// Lint policy: the CI gate is `clippy --workspace --all-targets -D warnings` on
// the default (correctness) groups. `pedantic` is deliberately *not* the gate —
// it is a wish list, and a gate nobody can keep green is a gate that gets
// switched off. Running it is still useful, and it earned its keep here: it is
// what found the `u64 as i64` in the attribution delta and the unchecked
// `f64 as u64` in size scaling. Both are now correct rather than allowed.
//
// Two groups are allowed on purpose:
// - `cast_precision_loss` is display math (`bytes as f64 / 1024.0`); a byte
//   count above 2^53 is 9 petabytes.
// - `too_many_lines` is `fusion::analyse`: splitting the join would hide the
//   order its steps depend on (attribute, ghost, hidden, coverage, invariant).
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]
#![forbid(unsafe_code)]

pub mod bom;
pub mod error;
pub mod fusion;
pub mod model;
pub mod report;
pub mod sizes;
pub mod sourcemap;
pub mod stats;

pub use error::{Error, Result};
pub use model::UnifiedBundleGraph;
