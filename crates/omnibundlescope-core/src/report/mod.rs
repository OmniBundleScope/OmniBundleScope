use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A node of the treemap: **sizes only, no module dump**. Per-module facts
/// belong in the detail payload.
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

/// `skip_serializing_if` hands the field by reference, so this signature is not
/// a choice: `fn is_zero(&u64)` is what serde's attribute requires.
#[allow(clippy::trivially_copy_pass_by_ref)]
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
    fn key_for(self, m: &crate::model::Module) -> String {
        match self {
            Dimension::SourceFile => {
                m.sources.first().map_or_else(|| m.name.clone(), |s| s.file.clone())
            }
            Dimension::Package => {
                m.package.as_ref().map_or_else(|| "<app>".to_string(), |p| p.name.clone())
            }
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
/// Two levels: asset -> group, with the tail folded (see `MAX_CHILDREN`).
///
/// # Panics
/// Never. Every input is treated as data, and a module with no owning asset
/// becomes the explicit `(modules outside any asset)` node rather than being
/// dropped or panicking on an empty search.
pub fn treemap_tree(graph: &crate::model::UnifiedBundleGraph, dim: Dimension) -> GroupNode {
    // modules are attributed to an asset through the chunk ids they share
    let mut per_asset: BTreeMap<String, BTreeMap<String, (u64, u64)>> = BTreeMap::new();
    let mut loose: BTreeMap<String, (u64, u64)> = BTreeMap::new();

    // Index chunk id -> asset name once. The obvious `assets.iter().find(|a|
    // module.chunks.iter().any(|c| a.chunks.contains(c)))` is O(modules x
    // assets x chunks): 154,379 x 1,500 for the 400 MB fixture, and this
    // function runs once per dimension. A BTreeMap lookup turns that into
    // O(modules x chunks) — the difference between seconds and minutes there.
    let mut chunk_owner: BTreeMap<u32, &str> = BTreeMap::new();
    for asset in &graph.assets {
        for chunk in &asset.chunks {
            // First asset wins, matching the previous `find` semantics.
            chunk_owner.entry(*chunk).or_insert(asset.name.as_str());
        }
    }

    for module in graph.modules.values() {
        let size = module.sizes.effective();
        let group = dim.key_for(module);

        let owner = module.chunks.iter().find_map(|c| chunk_owner.get(c).copied());

        let bucket = match owner {
            Some(asset) => per_asset.entry(asset.to_string()).or_default(),
            None => &mut loose,
        };
        bucket
            .entry(group)
            .and_modify(|(s, c)| {
                *s += size;
                *c += 1;
            })
            .or_insert((size, 1u64));
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

fn fold_asset_tail(mut nodes: Vec<GroupNode>) -> (Vec<GroupNode>, (u64, u64)) {
    if nodes.len() <= MAX_ASSETS {
        return (nodes, (0, 0));
    }
    nodes.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    let folded: Vec<GroupNode> = nodes.split_off(MAX_ASSETS);
    let (size, module_count) =
        folded.iter().fold((0u64, 0u64), |(s, m), n| (s + n.size, m + n.module_count));
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
    let mut ordered: Vec<(String, u64, u64)> =
        groups.into_iter().map(|(k, (size, count))| (k, size, count)).collect();
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
/// Stream the per-module detail payload straight to `out`.
///
/// This used to build a `serde_json::Value` first and serialise that. For
/// 154,379 modules the intermediate `Value` cost ~550 MB — a `Value` is a tree
/// of boxed maps and owned strings, roughly an order of magnitude more memory
/// than the JSON text it produces — and it pushed the full pipeline from 126 MB
/// exactly this. Serialising from a borrowed view of the graph costs one buffer.
///
/// It is a `Detail<'a>` view, not an owned copy: the module id is the map key
/// and nothing needs to be cloned to write it.
/// # Errors
/// Propagates a write or serialisation failure from out; nothing is
/// swallowed, because a truncated detail file that looks complete is worse than
/// a failed command.
pub fn write_detail<W: std::io::Write>(
    graph: &crate::model::UnifiedBundleGraph,
    out: &mut W,
) -> std::io::Result<u64> {
    serde_json::to_writer(
        &mut *out,
        &Detail {
            schema_version: graph.schema_version,
            modules: DetailModules(&graph.modules),
            fusion: &graph.fusion,
        },
    )
    .map_err(std::io::Error::other)?;
    // The trailing newline keeps the companion file a well-formed JS assignment.
    out.write_all(b"\n")?;
    Ok(0)
}

#[derive(serde::Serialize)]
struct Detail<'a> {
    schema_version: u32,
    modules: DetailModules<'a>,
    fusion: &'a Option<crate::model::FusionSummary>,
}

/// Serialises the module table as `{ "<module id>": { …facts } }` without
/// cloning an `id` per entry: the id *is* the key, and borrowing it is what
/// keeps this off the heap.
struct DetailModules<'a>(&'a BTreeMap<String, crate::model::Module>);

impl serde::Serialize for DetailModules<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (id, module) in self.0 {
            map.serialize_entry(
                id,
                &DetailModule {
                    name: &module.name,
                    package: module.package.as_ref().map(|p| p.name.as_str()),
                    chunks: &module.chunks,
                    reasons: &module.reasons,
                    sources: module.sources.iter().map(|s| s.file.as_str()).collect(),
                    sizes: module.sizes,
                    attribution_delta: module.attribution_delta,
                },
            )?;
        }
        map.end()
    }
}

/// One module as the shell's drill-down needs it.
#[derive(serde::Serialize)]
struct DetailModule<'a> {
    name: &'a str,
    package: Option<&'a str>,
    chunks: &'a [u32],
    reasons: &'a [String],
    sources: Vec<&'a str>,
    sizes: crate::model::SizeSet,
    attribution_delta: i64,
}

/// # Errors
/// Propagates write failures from out.
pub fn write_csv<W: std::io::Write>(
    graph: &crate::model::UnifiedBundleGraph,
    out: &mut W,
) -> std::io::Result<()> {
    writeln!(out, "module_id,name,package,chunks,stat,parsed,gzip,attributed,delta")?;
    for (id, module) in &graph.modules {
        let package = module.package.as_ref().map_or("", |p| p.name.as_str());
        let chunks = module.chunks.iter().map(u32::to_string).collect::<Vec<_>>().join("|");
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{}",
            csv_field(id),
            csv_field(&module.name),
            csv_field(package),
            chunks,
            module.sizes.stat,
            module.sizes.parsed,
            module.sizes.gzip,
            module.sizes.attributed.map(|v| v.to_string()).unwrap_or_default(),
            module.attribution_delta,
        )?;
    }
    Ok(())
}

/// RFC 4180 quoting, only where it is needed.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{Dimension, MAX_ASSETS, MAX_CHILDREN, treemap_tree};
    use crate::model::{
        Asset, Chunk, Module, PackageRef, SizeDimension, SizeSet, UnifiedBundleGraph,
    };
    use std::collections::BTreeMap;

    fn module(name: &str, size: u64, chunks: Vec<u32>, pkg: Option<&str>) -> Module {
        Module {
            id: name.to_string(),
            name: name.to_string(),
            issuer: None,
            reasons: vec!["./src/app.js".to_string()],
            package: pkg.map(|p| {
                std::sync::Arc::new(PackageRef {
                    name: p.to_string(),
                    version: None,
                    path: format!("node_modules/{p}"),
                })
            }),
            chunks,
            sizes: SizeSet { stat: size, parsed: size, ..SizeSet::default() },
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
        // The asset is *measured*, as sizes does from disk. Without a real
        // asset size the totals are 0 and the tree falls back to the module
        // sum, which is a different (and much weaker) test.
        g.assets.push(Asset {
            name: "main.js".into(),
            size: asset_size,
            chunks: vec![0],
            sizes: SizeSet { stat: asset_size, parsed: asset_size, ..SizeSet::default() },
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
        let other = asset.children.iter().find(|n| n.name == "other").expect("other bucket");
        assert_eq!(other.module_count, 40);
        assert_eq!(asset.dropped, 40, "the cap must be reported, not hidden");
    }

    #[test]
    fn asset_level_is_capped_and_the_fold_is_visible() {
        let mut g = fixture(4);
        // add many more assets, each with its own chunk, to blow past MAX_ASSETS
        for i in 0..(MAX_ASSETS + 10) {
            let chunk_id = 100 + u32::try_from(i).unwrap_or(u32::MAX);
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
        let folded =
            tree.children.iter().find(|c| c.name.starts_with("(+")).expect("folded asset node");
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
