//! Placeholder for the Phase 2 WASM surface.
//!
//! We deliberately do **not** depend on `wasm-bindgen` yet (ADR-0003): the goal
//! for Phase 1 is only to keep `omnibundle-core` free of IO so that adding the
//! bindings later is a packaging change. When Phase 2 starts, this crate grows
//! the real functions:
//!
//! - `analyse_stats(bytes) -> graph json`
//! - `analyse_source_map(bytes) -> attributed sizes`
//! - `analyse_tree(stats, maps) -> fused graph` (browser-side drag & drop)
//!
//! The JSON strings returned to JS are the same `report-schema.json` payload the
//! CLI writes, so the web UI and the HTML report never drift.

/// Schema version of the payload this crate will expose, kept in sync with
/// `omnibundle_core::model::UnifiedBundleGraph::SCHEMA_VERSION`.
pub const SCHEMA_VERSION: u32 = omnibundle_core::model::UnifiedBundleGraph::SCHEMA_VERSION;

/// Placeholder entry point so the crate is buildable and testable on every
/// target, including non-wasm hosts.
pub fn schema_version() -> u32 {
    omnibundle_core::model::UnifiedBundleGraph::new().schema_version
}

#[cfg(test)]
mod tests {
    #[test]
    fn core_is_reachable_from_the_wasm_surface() {
        assert_eq!(super::schema_version(), super::SCHEMA_VERSION);
    }
}
