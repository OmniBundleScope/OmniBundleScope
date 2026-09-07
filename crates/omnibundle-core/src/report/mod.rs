use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A node of the treemap the report draws: **sizes only, no module dump**.
///
/// This is the fix for the measured bottleneck: serialising 154,379 modules
/// into the HTML produced a 125.6 MB file and ~24 s of report generation
/// (`docs/en/04-benchmark-plan.md` §6). The picture a human reads needs a
/// handful of levels and nothing else; per-module facts belong in the detail
/// payload, which is emitted separately and loaded only when a node is opened.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupNode {
    pub name: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<GroupNode>,
    /// Modules behind this node, and the number of nodes dropped by the child
    /// cap. `dropped > 0` must be visible in the report, not silent.
    pub module_count: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub dropped: u64,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// Children kept per node before the remainder is folded into `other`.
pub const MAX_CHILDREN: usize = 256;
/// Assets kept as the treemap's top level before the remainder is folded into
/// one node. A build with 1,500 assets and 400 packages each would otherwise
/// produce a 384,000-node tree (measured: a 4.5 MB report); a real build has
/// tens of assets, so this cap only bites pathological inputs — and when it
/// does, the fold is visible rather than silent.
pub const MAX_ASSETS: usize = 64;
const OTHER: &str = "other";

/// Grouping dimensions available in the MVP. "Module dependency" (edge view) is
/// deliberately Phase 2 — see `docs/decisions/ADR-0002`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    /// Group by original source file (post-fusion).
    SourceFile,
    /// Group by npm package.
    Package,
    /// Group by chunk / async boundary.
    Chunk,
}

impl Dimension {
    pub fn as_str(self) -> &'static str {
        match self {
            Dimension::SourceFile => "source_file",
            Dimension::Package => "package",
            Dimension::Chunk => "chunk",
        }
    }
}

impl Dimension {
    /// The group key a module belongs to under this dimension.
    fn key_for(&self, m: &crate::model::Module) -> String {
        match self {
            Dimension::SourceFile => m
                .sources
                .first()
                .map_or_else(|| m.name.clone(), |s| s.file.clone()),
            Dimension::Package => m
                .package
                .as_ref()
                .map_or_else(|| "<app>".to_string(), |p| p.name.clone()),
            Dimension::Chunk => {
                if m.chunks.is_empty() {
                    "no chunk".to_string()
                } else {
                    let ids: Vec<String> = m.chunks.iter().map(u32::to_string).collect();
                    format!("chunk {}", ids.join("+"))
                }
            }
        }
    }
}

/// Build the two-level treemap the report draws: asset → group.
///
/// Deterministic by construction: modules are read from a `BTreeMap`, so the
/// grouping is stable across runs and parity tests can diff it.
pub fn treemap_tree(graph: &crate::model::UnifiedBundleGraph, dim: Dimension) -> GroupNode {
    // modules are attributed to an asset through the chunk ids they share
    let mut per_asset: BTreeMap<String, BTreeMap<String, (u64, u64)>> = BTreeMap::new();
    let mut loose: BTreeMap<String, (u64, u64)> = BTreeMap::new();

    for module in graph.modules.values() {
        let size = module.sizes.effective();
        let group = dim.key_for(module);

        let owner = graph
            .assets
            .iter()
            .find(|a| module.chunks.iter().any(|c| a.chunks.contains(c)))
            .map(|a| a.name.clone());

        let bucket = match owner {
            Some(asset) => per_asset.entry(asset).or_default(),
            None => &mut loose,
        };
        bucket.entry(group).and_modify(|(s, c)| {
            *s += size;
            *c += 1;
        }).or_insert((size, 1u64));
    }

    let mut children: Vec<GroupNode> = graph
        .assets
        .iter()
        .map(|asset| {
            let groups = per_asset.remove(&asset.name).unwrap_or_default();
            let (nodes, dropped) = groups_to_nodes(groups);
            let size: u64 = nodes.iter().map(|n| n.size).sum();
            let module_count: u64 = nodes.iter().map(|n| n.module_count).sum();
            GroupNode {
                name: asset.name.clone(),
                size: size.max(asset.sizes.effective()),
                module_count,
                children: nodes,
                dropped,
            }
        })
        .collect();

    if !loose.is_empty() {
        let (nodes, dropped) = groups_to_nodes(loose);
        let size = nodes.iter().map(|n| n.size).sum();
        let module_count = nodes.iter().map(|n| n.module_count).sum();
        children.push(GroupNode {
            name: "(modules outside any asset)".to_string(),
            size,
            module_count,
            children: nodes,
            dropped,
        });
    }

    let total: u64 = children.iter().map(|c| c.size).sum();
    let (children, folded_assets) = fold_asset_tail(children);

    GroupNode {
        name: "bundle".to_string(),
        size: total.max(graph.totals.total_size),
        module_count: graph.modules.len() as u64,
        dropped: children.iter().map(|c| c.dropped).sum::<u64>() + folded_assets.1,
        children,
    }
}

/// Keep the largest `MAX_ASSETS` assets; fold the rest into one node whose size
/// is their sum, so the picture stays arithmetically honest.
fn fold_asset_tail(mut nodes: Vec<GroupNode>) -> (Vec<GroupNode>, (u64, u64)) {
    if nodes.len() <= MAX_ASSETS {
        return (nodes, (0, 0));
    }
    nodes.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    let folded: Vec<GroupNode> = nodes.split_off(MAX_ASSETS);
    let (size, module_count) = folded.iter().fold((0u64, 0u64), |(s, m), n| {
        (s + n.size, m + n.module_count)
    });
    nodes.push(GroupNode {
        name: format!("(+{} more assets)", folded.len()),
        size,
        module_count,
        children: Vec::new(),
        dropped: 0,
    });
    (nodes, (folded.len() as u64, module_count))
}

/// Turn a group map into capped child nodes, folding the tail into `other`.
fn groups_to_nodes(groups: BTreeMap<String, (u64, u64)>) -> (Vec<GroupNode>, u64) {
    let mut ordered: Vec<(String, u64, u64)> = groups
        .into_iter()
        .map(|(k, (size, count))| (k, size, count))
        .collect();
    // size desc, then name asc: stable and reproducible
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut dropped = 0u64;
    let mut nodes: Vec<GroupNode> = Vec::new();
    for (name, size, count) in ordered.iter().take(MAX_CHILDREN) {
        nodes.push(GroupNode {
            name: name.clone(),
            size: *size,
            children: Vec::new(),
            module_count: *count,
            dropped: 0,
        });
    }
    for (_, size, count) in ordered.iter().skip(MAX_CHILDREN) {
        dropped += *count;
        match nodes.iter_mut().find(|n| n.name == OTHER) {
            Some(other) => {
                other.size += size;
                other.module_count += count;
            }
            None => nodes.push(GroupNode {
                name: OTHER.to_string(),
                size: *size,
                children: Vec::new(),
                module_count: *count,
                dropped: 0,
            }),
        }
    }
    (nodes, dropped)
}

/// Per-module facts for the inspector: sizes, who pulled it in, what it maps
/// back to.
///
/// This is the payload that must **not** be inlined into the HTML for large
/// graphs; it is emitted as a companion script (loaded via a `<script>` tag,
/// which works from `file://` where `fetch` does not).
pub fn detail_payload(graph: &crate::model::UnifiedBundleGraph) -> serde_json::Value {
    let mut modules = serde_json::Map::new();
    for module in graph.modules.values() {
        modules.insert(
            module.id.clone(),
            serde_json::json!({
                "name": module.name,
                "package": module.package.as_ref().map(|p| p.name.clone()),
                "chunks": module.chunks,
                "reasons": module.reasons,
                "sources": module.sources.iter().map(|s| s.file.clone()).collect::<Vec<_>>(),
                "sizes": module.sizes,
                "attribution_delta": module.attribution_delta,
            }),
        );
    }
    serde_json::json!({
        "schema_version": graph.schema_version,
        "modules": modules,
        "fusion": graph.fusion,
    })
}

/// CSV projection for BI imports: one row per module, columns follow
/// `docs/contracts/report-schema.json`.
pub fn csv_rows(graph: &crate::model::UnifiedBundleGraph) -> Vec<Vec<String>> {
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
            module
                .package
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::{Dimension, MAX_ASSETS, MAX_CHILDREN, treemap_tree};
    use crate::model::{
        Asset, Chunk, Module, PackageRef, SizeSet, SizeDimension, UnifiedBundleGraph,
    };
    use std::collections::BTreeMap;

    fn module(name: &str, size: u64, chunks: Vec<u32>, pkg: Option<&str>) -> Module {
        Module {
            id: name.to_string(),
            name: name.to_string(),
            issuer: None,
            reasons: vec!["./src/app.js".to_string()],
            package: pkg.map(|p| PackageRef {
                name: p.to_string(),
                version: None,
                path: format!("node_modules/{p}"),
            }),
            chunks,
            sizes: SizeSet {
                stat: size,
                parsed: size,
                ..SizeSet::default()
            },
            attribution_delta: 0,
            sources: Vec::new(),
        }
    }

    fn fixture(module_count: usize) -> UnifiedBundleGraph {
        let mut g = UnifiedBundleGraph::new();
        g.chunks.push(Chunk {
            id: 0,
            names: vec!["main".into()],
            initial: true,
            assets: vec!["main.js".into()],
            size: SizeSet::default(),
        });
        let mut modules = BTreeMap::new();
        let mut asset_size = 0u64;
        for i in 0..module_count {
            let m = module(
                &format!("./node_modules/pkg{i}/i.js"),
                100 + i as u64,
                vec![0],
                Some(&format!("pkg{i}")),
            );
            asset_size += m.sizes.parsed;
            modules.insert(m.id.clone(), m);
        }
        // The asset is *measured*, as WS-2 does from disk. Without a real
        // asset size the totals are 0 and the tree falls back to the module
        // sum, which is a different (and much weaker) test.
        g.assets.push(Asset {
            name: "main.js".into(),
            size: asset_size,
            chunks: vec![0],
            sizes: SizeSet {
                stat: asset_size,
                parsed: asset_size,
                ..SizeSet::default()
            },
        });
        g.modules = modules;
        crate::stats::recompute_totals(&mut g);
        g.totals.size_dimension = SizeDimension::Parsed;
        g
    }

    #[test]
    fn tree_is_two_levels_and_totals_match() {
        let g = fixture(30);
        let tree = treemap_tree(&g, Dimension::Package);
        assert_eq!(tree.children.len(), 1, "one asset");
        let asset = &tree.children[0];
        assert_eq!(asset.children.len(), 30, "one node per package");
        assert_eq!(tree.size, g.totals.total_size);
        assert_eq!(tree.module_count, 30);
    }

    #[test]
    fn children_are_capped_and_the_tail_is_visible() {
        let g = fixture(MAX_CHILDREN + 40);
        let tree = treemap_tree(&g, Dimension::Package);
        let asset = &tree.children[0];
        assert_eq!(asset.children.len(), MAX_CHILDREN + 1, "capped, plus the `other` node");
        let other = asset
            .children
            .iter()
            .find(|n| n.name == "other")
            .expect("other bucket");
        assert_eq!(other.module_count, 40);
        assert_eq!(asset.dropped, 40, "the cap must be reported, not hidden");
    }

    #[test]
    fn asset_level_is_capped_and_the_fold_is_visible() {
        let mut g = fixture(4);
        // add many more assets, each with its own chunk, to blow past MAX_ASSETS
        for i in 0..(MAX_ASSETS + 10) {
            let chunk_id = 100 + i as u32;
            g.chunks.push(Chunk {
                id: chunk_id,
                names: vec![format!("c{i}")],
                initial: false,
                assets: vec![format!("extra-{i}.js")],
                size: SizeSet::default(),
            });
            g.assets.push(Asset {
                name: format!("extra-{i}.js"),
                size: 10,
                chunks: vec![chunk_id],
                sizes: SizeSet { stat: 10, parsed: 10, ..SizeSet::default() },
            });
            // one real module per extra asset, so the fold actually hides work
            let m = module(
                &format!("./node_modules/extra{i}/i.js"),
                10,
                vec![chunk_id],
                Some(&format!("extra{i}")),
            );
            g.modules.insert(m.id.clone(), m);
        }
        let tree = treemap_tree(&g, Dimension::Package);
        assert!(tree.children.len() <= MAX_ASSETS + 1, "asset level must be capped");
        let folded = tree
            .children
            .iter()
            .find(|c| c.name.starts_with("(+"))
            .expect("folded asset node");
        assert!(folded.size >= 100, "folded node keeps the bytes: {}", folded.size);
        assert!(tree.dropped >= 10, "the fold is reported, not hidden");
    }

    #[test]
    fn chunk_dimension_groups_by_chunk_id() {
        let g = fixture(3);
        let tree = treemap_tree(&g, Dimension::Chunk);
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].name, "chunk 0");
    }

    #[test]
    fn package_dimension_falls_back_to_app() {
        let mut g = fixture(1);
        g.modules.clear();
        let m = module("./src/app.js", 500, vec![0], None);
        g.modules.insert(m.id.clone(), m);
        crate::stats::recompute_totals(&mut g);
        let tree = treemap_tree(&g, Dimension::Package);
        assert_eq!(tree.children[0].children[0].name, "<app>");
    }
}
