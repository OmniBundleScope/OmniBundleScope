//! `OmniBundle` core engine.
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
