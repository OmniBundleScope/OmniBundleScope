//! `OmniBundleScope` core: stats ingest, size attribution, source-map parsing, the
//! fusion engine and the report payload.
//!
//! No IO, and no CLI or async runtime. Bytes enter through [`model`] or the
//! `*_from_*` constructors in the ingest modules.

// Lint policy: the workspace sets `clippy::pedantic = warn` and every crate opts in,
// so CI's `clippy --workspace --all-targets -- -D warnings` gates on pedantic too.
// It found the `u64 as i64` in the attribution delta and the unchecked `f64 as u64`
// in size scaling; both are now correct rather than allowed.
//!
// Two groups are allowed on purpose:
// - `cast_precision_loss` is display math (`bytes as f64 / 1024.0`).
// - `too_many_lines` is `fusion::analyse`: splitting it would hide the order its
//   steps depend on.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]
#![forbid(unsafe_code)]

pub mod bom;
pub mod error;
pub mod folder;
pub mod fusion;
pub mod model;
pub mod report;
pub mod sizes;
pub mod sourcemap;
pub mod stats;

pub use error::{Error, Result};
pub use model::UnifiedBundleGraph;
