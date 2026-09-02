//! WS-5: the report payload.
//!
//! The HTML shell is **written from scratch** (see ADR-0002): a single
//! self-contained file with a squarified treemap drawn on Canvas 2D, fuzzy
//! search, three grouping dimensions (source file / npm package / chunk) and a
//! dark theme. No vendored viewer, no template engine, no runtime dependency.
//!
//! What lives here is only the payload assembly: the core produces a
//! `serde_json::Value` the shell consumes, plus the CSV projection. The shell
//! itself is TypeScript under `assets/report/`, bundled and inlined by
//! `crates/cli` at build time.

use crate::model::UnifiedBundleGraph;

/// Grouping dimensions available in the MVP. "Module dependency" (edge view)
/// is deliberately Phase 2 — see `docs/decisions/ADR-0002`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    /// Group by original source file (post-fusion).
    SourceFile,
    /// Group by npm package.
    Package,
    /// Group by chunk / async boundary.
    Chunk,
}

/// Build the treemap payload for one dimension. Ordering is stable so the
/// report is byte-reproducible across runs (parity tests depend on it).
pub fn treemap_payload(graph: &UnifiedBundleGraph, dim: Dimension) -> serde_json::Value {
    let mut nodes: Vec<serde_json::Value> = Vec::new();

    for asset in &graph.assets {
        // modules reach an asset through the chunk ids they share with it
        let mut groups: std::collections::BTreeMap<String, u64> =
            std::collections::BTreeMap::new();

        for module in graph.modules.values() {
            if !module.chunks.iter().any(|c| asset.chunks.contains(c)) {
                continue;
            }
            let key = match dim {
                Dimension::SourceFile => module
                    .sources
                    .first()
                    .map_or_else(|| module.name.clone(), |s| s.file.clone()),
                Dimension::Package => module
                    .package
                    .as_ref()
                    .map_or_else(|| "<app>".to_string(), |p| p.name.clone()),
                Dimension::Chunk => module
                    .chunks
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("|"),
            };
            *groups.entry(key).or_insert(0) += module.sizes.effective();
        }

        let children: Vec<serde_json::Value> = groups
            .into_iter()
            .map(|(name, size)| serde_json::json!({ "name": name, "size": size }))
            .collect();

        nodes.push(serde_json::json!({
            "name": asset.name,
            "size": asset.sizes.effective(),
            "children": children,
        }));
    }

    serde_json::json!({
        "schema_version": graph.schema_version,
        "dimension": dim,
        "size_dimension": graph.totals.size_dimension,
        "nodes": nodes,
        "diagnostics": graph.diagnostics,
    })
}

/// CSV projection for BI imports: one row per module, columns follow
/// `docs/contracts/report-schema.json`.
pub fn csv_rows(graph: &UnifiedBundleGraph) -> Vec<Vec<String>> {
    let mut rows = vec![vec![
        "module_id".into(),
        "name".into(),
        "package".into(),
        "chunks".into(),
        "stat".into(),
        "parsed".into(),
        "gzip".into(),
        "attributed".into(),
        "delta".into(),
    ]];
    for module in graph.modules.values() {
        rows.push(vec![
            module.id.clone(),
            module.name.clone(),
            module.package.as_ref().map(|p| p.name.clone()).unwrap_or_default(),
            module
                .chunks
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join("|"),
            module.sizes.stat.to_string(),
            module.sizes.parsed.to_string(),
            module.sizes.gzip.to_string(),
            module.sizes
                .attributed
                .map(|v| v.to_string())
                .unwrap_or_default(),
            module.attribution_delta.to_string(),
        ]);
    }
    rows
}
