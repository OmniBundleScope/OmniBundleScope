//! The unified bundle graph — the single contract every areas codes against.
//!
//! This mirrors `docs/contracts/report-schema.json` and `docs/contracts/unified-graph.md`.
//! Field renames or semantic changes require a new ADR (contracts owns this file).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Root of the report. One instance per analysed directory/bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedBundleGraph {
    /// Schema version of this payload. Bumped on any breaking change.
    pub schema_version: u32,
    /// Ingest artifacts, in the order they were merged.
    pub inputs: Vec<InputArtifact>,
    pub assets: Vec<Asset>,
    pub chunks: Vec<Chunk>,
    pub modules: BTreeMap<String, Module>,
    /// Top-level rollups used by the report shell and the budget gate.
    pub totals: Totals,
    /// Present once a source map has been fused in.
    pub fusion: Option<FusionSummary>,
    /// Diagnostics that are warnings, not errors: ghost/hidden code, unmapped
    /// assets, budget breaches, degraded input.
    pub diagnostics: Vec<Diagnostic>,
}

/// Where a piece of the graph came from. Determines how much we can trust its
/// size numbers and whether the fusion engine may correct them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputArtifact {
    /// webpack / rspack `stats.json`
    Stats { tool: String },
    /// Source Map v3 (`*.map`, or inline `sourceMappingURL`)
    SourceMap { name: String },
    /// esbuild `metafile.json`
    EsbuildMetafile,
    /// rollup-plugin-visualizer style JSON
    VisualizerStats { tool: String },
    /// A build directory with no bundler metadata: sizes are measured from disk,
    /// and the source maps are the only attribution available. This is what
    /// vite, rollup, parcel and tsup hand you unless you configure otherwise.
    DistFolder,
}

impl InputArtifact {
    /// Sizes taken from a stats file are estimates; a source map is ground truth
    /// for per-source byte attribution.
    pub fn trusts_exact_sizes(&self) -> bool {
        matches!(self, InputArtifact::SourceMap { .. })
    }
}

/// A build output file (JS/CSS/asset), the unit the treemap roots at.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    /// Declared size in bytes.
    pub size: u64,
    /// Chunks that contribute to this asset.
    pub chunks: Vec<u32>,
    /// `stat` (declared), `parsed` (measured from the file), `gzip`.
    pub sizes: SizeSet,
}

/// A chunk: the unit of async loading. Mirrors the webpack chunk id space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: u32,
    pub names: Vec<String>,
    /// Initial chunks ship in the first payload; the rest are lazy.
    pub initial: bool,
    pub assets: Vec<String>,
    pub size: SizeSet,
}

/// A module: one entry of the dependency graph.
///
/// `id` is the join key with the source map (stats `identifier` or
/// `name` + chunk). When a map disagrees with the stats size, the corrected
/// value lands in [`ModuleSize::attributed`] and the delta is recorded in
/// [`Module::attribution_delta`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Stable join key: the stats `identifier` when available, else
    /// `chunk-scoped name`. Documented in the fusion contract.
    pub id: String,
    pub name: String,
    pub issuer: Option<String>,
    /// Reasons the module was pulled in (webpack `reasons`).
    pub reasons: Vec<String>,
    /// npm package the module belongs to, when it lives in `node_modules`.
    ///
    /// Shared, not owned: the 1 GB fixture has 445,602 modules and 400 packages,
    /// and three owned strings per module cost ~55 MB — enough to miss the
    /// 400 MB target for no reason at all.
    pub package: Option<std::sync::Arc<PackageRef>>,
    pub chunks: Vec<u32>,
    pub sizes: SizeSet,
    /// `map_attributed - stat` in bytes. Non-zero means the bundler's estimate
    /// was off, which is the whole point of the fusion engine.
    pub attribution_delta: i64,
    /// Sources this module maps back to, filled in by the fusion engine.
    pub sources: Vec<SourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRef {
    pub name: String,
    pub version: Option<String>,
    /// The package root inside `node_modules`, used for treemap grouping.
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceRef {
    pub file: String,
    /// Byte length attributed to this source inside the module's generated code.
    pub bytes: u64,
    /// Original line, when the mapping carries it.
    pub line: Option<u32>,
}

/// The three size dimensions `OmniBundleScope` reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeSet {
    /// What the bundler claimed.
    pub stat: u64,
    /// Measured from the emitted file.
    pub parsed: u64,
    /// gzip of the emitted file.
    pub gzip: u64,
    /// Ground truth from the source map, when fused.
    pub attributed: Option<u64>,
}

impl SizeSet {
    /// The size to display. Prefers ground truth, then measurement, then the
    /// bundler's claim. Never silently mixes dimensions: a report states which
    /// dimension it used (see `totals.size_dimension`).
    pub fn effective(&self) -> u64 {
        if let Some(attributed) = self.attributed.filter(|v| *v > 0) {
            return attributed;
        }
        if self.parsed > 0 { self.parsed } else { self.stat }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Totals {
    pub total_size: u64,
    pub module_count: u64,
    pub asset_count: u64,
    pub package_count: u64,
    /// Which dimension `total_size` is expressed in.
    pub size_dimension: SizeDimension,
    /// Sum of every module's effective size, used to check the
    /// `sum(modules) == total` invariant.
    pub module_size_sum: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeDimension {
    Stat,
    #[default]
    Parsed,
    Gzip,
    Attributed,
}

/// Result of cross-checking stats against source maps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FusionSummary {
    /// Modules the stats file declares but which no mapping accounts for.
    /// Typically tree-shaking misses or assets built without a map.
    pub ghost_modules: Vec<GhostModule>,
    /// Ghost modules not listed above, because the list is a bounded sample of the
    /// largest. `ghost_count` and `ghost_bytes` remain exact.
    pub ghost_modules_truncated: u64,
    /// Generated bytes with no module in the stats graph: inlined snippets,
    /// runtime/eval blobs, polyfills injected by the bundler.
    pub hidden_sources: Vec<HiddenSource>,
    /// Hidden sources not listed above, because the list is a bounded sample.
    pub hidden_sources_truncated: u64,
    /// `|attributed - stat|` summed over all modules.
    pub total_attribution_delta: i64,
    /// Modules whose corrected size changed the picture by more than 1%.
    pub significant_corrections: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostModule {
    pub module: String,
    pub declared_size: u64,
    pub chunks: Vec<u32>,
    /// Why we believe it is a ghost.
    pub reason: GhostReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GhostReason {
    /// Declared in the graph, absent from every mapping of the chunk.
    Unmapped,
    /// Mapped, but the attributed size is zero.
    EmptyAttribution,
    /// Present in the stats file but the asset that should contain it does not.
    MissingAsset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenSource {
    pub file: String,
    pub bytes: u64,
    /// Asset or chunk the bytes landed in, when we can tell.
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    /// Module/asset/file the diagnostic is about, for UI linking.
    pub subject: Option<String>,
    /// Extra structured payload; the report shell renders known codes richly.
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl UnifiedBundleGraph {
    /// Bumped on any breaking change to the payload; mirrored in
    /// `docs/contracts/report-schema.json` and `omnibundlescope-wasm`.
    pub const SCHEMA_VERSION: u32 = 1;

    /// An empty graph at the current schema version.
    pub fn new() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            inputs: Vec::new(),
            assets: Vec::new(),
            chunks: Vec::new(),
            modules: BTreeMap::new(),
            totals: Totals::default(),
            fusion: None,
            diagnostics: Vec::new(),
        }
    }

    /// Contract invariant: the sum of module effective sizes matches the total
    /// within `tolerance` (a ratio, e.g. 0.001 for 0.1%).
    ///
    /// The fusion areas must call this before writing a report; a
    /// violation is a bug in the join keys, not a rounding error. The check is
    /// integer-only: `drift * 1_000_000 <= total * tolerance_ppm`, so even a
    /// multi-terabyte total cannot make the check itself drift.
    ///
    /// # Errors
    /// Returns an error when the module sizes and the asset total disagree by
    /// more than `tolerance_ppm` parts per million. A caller that ignores this
    /// is publishing a number nobody checked.
    pub fn check_size_invariant(&self, tolerance_ppm: u64) -> Result<(), super::Error> {
        if self.totals.size_dimension != SizeDimension::Attributed {
            return Ok(());
        }
        let total = self.totals.total_size;
        if total == 0 {
            return Ok(());
        }
        let drift = self.totals.module_size_sum.abs_diff(total);
        let allowed = u128::from(total) * u128::from(tolerance_ppm);
        if u128::from(drift) * 1_000_000 > allowed {
            return Err(super::Error::Invariant(format!(
                "module sizes sum to {} but total is {total} (drift {drift} > allowed {allowed} ppm-scale)",
                self.totals.module_size_sum
            )));
        }
        Ok(())
    }
}

impl Default for UnifiedBundleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{SizeDimension, SizeSet, UnifiedBundleGraph};

    #[test]
    fn new_graph_is_at_the_current_schema_version() {
        assert_eq!(UnifiedBundleGraph::new().schema_version, UnifiedBundleGraph::SCHEMA_VERSION);
    }

    #[test]
    fn size_preference_is_ground_truth_then_measurement_then_claim() {
        let mut sizes = SizeSet { stat: 10, parsed: 20, gzip: 5, attributed: None };
        assert_eq!(sizes.effective(), 20, "measurement wins over the claim");
        sizes.attributed = Some(30);
        assert_eq!(sizes.effective(), 30, "ground truth wins over everything");
        sizes.parsed = 0;
        assert_eq!(sizes.effective(), 30);
        sizes.attributed = None;
        assert_eq!(sizes.effective(), 10, "falls back to the claim");
    }

    #[test]
    fn invariant_only_applies_to_attributed_totals() {
        let mut graph = UnifiedBundleGraph::new();
        graph.totals.total_size = 100_000;
        graph.totals.module_size_sum = 50; // wildly wrong, but not asserted yet
        graph.totals.size_dimension = SizeDimension::Parsed;
        assert!(graph.check_size_invariant(1_000).is_ok());

        graph.totals.size_dimension = SizeDimension::Attributed;
        graph.totals.module_size_sum = 50;
        assert!(graph.check_size_invariant(1_000).is_err());

        // 0.1% of 100,000 bytes is 100 bytes of slack
        graph.totals.module_size_sum = 100_050;
        assert!(graph.check_size_invariant(1_000).is_ok());
        graph.totals.module_size_sum = 100_200;
        assert!(graph.check_size_invariant(1_000).is_err());
    }
}
