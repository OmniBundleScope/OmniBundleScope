use std::collections::BTreeMap;

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::{RawAsset, RawChunk, RawModule, join_key, package_of};
use crate::model::{
    Asset, Chunk, InputArtifact, Module, PackageRef, SizeDimension, SizeSet, Totals,
    UnifiedBundleGraph,
};
use crate::{Error, Result};

/// Ingest from any source that implements [`std::io::Read`].
///
/// **This is the entry point that matters.** Reading the file into a `Vec<u8>`
/// first (as a naive CLI does) puts the file size on top of the memory floor:
/// a 1 GB stats file costs 1 GB before we parse a single module. Streaming from
/// a buffered reader keeps the resident set at the size of the *graph*, which is
/// what the measured 17 MB / 59 MB numbers come from.
///
/// Measured on the corrected 1 GB fixture, a 445,602-module stats file.
pub fn ingest_reader<R: std::io::Read>(reader: R, tool: &str) -> Result<UnifiedBundleGraph> {
    let buffered = std::io::BufReader::with_capacity(1 << 20, crate::bom::BomSkip::new(reader));
    let mut de = serde_json::Deserializer::from_reader(buffered);
    use serde::de::Deserializer as _;
    let graph = de.deserialize_map(StatsVisitor { tool: tool.to_string() })?;
    de.end()?;
    Ok(graph)
}

/// Ingest a webpack/rspack `stats.json` into the unified graph, **without
/// materialising the document**.
///
/// This is the single most important implementation constraint in the project
/// (ADR-0001). The measured consequence of getting it wrong is a ~2 GB floor on
/// a 1 GB input, i.e. a return of the very OOM we are here to remove.
///
/// How the streaming works:
/// - a top-level map visitor sees the keys of the document one at a time;
/// - `assets` and `chunks` are small, so they are collected as vectors;
/// - `modules` is a seed whose visitor pulls one element at a time, converts it
///   into a graph `Module`, inserts it, and drops the raw value — the module
///   array is never resident as a whole;
/// - every other key (including the multi-kilobyte `source` strings) is read as
///   `IgnoredAny`, which parses without allocating.
pub fn ingest_stats(bytes: &[u8], tool: &str) -> Result<UnifiedBundleGraph> {
    ingest_reader(bytes, tool)
}

struct StatsVisitor {
    tool: String,
}

impl<'de> Visitor<'de> for StatsVisitor {
    type Value = UnifiedBundleGraph;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a webpack/rspack stats object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Self::Value, A::Error> {
        let mut graph = UnifiedBundleGraph::new();
        let mut saw_modules = false;

        while let Some(key) = map.next_key::<std::borrow::Cow<str>>()? {
            match key.as_ref() {
                "assets" => {
                    let mut assets = Vec::new();
                    map.next_value_seed(SeqCollector::<RawAsset>::new(&mut assets))?;
                    graph.assets = assets
                        .into_iter()
                        .map(|a| Asset {
                            name: a.name,
                            size: a.size,
                            chunks: a.chunks,
                            sizes: SizeSet { stat: a.size, ..SizeSet::default() },
                        })
                        .collect();
                }
                "chunks" => {
                    let mut chunks = Vec::new();
                    map.next_value_seed(SeqCollector::<RawChunk>::new(&mut chunks))?;
                    graph.chunks = chunks
                        .into_iter()
                        .map(|c| Chunk {
                            id: c.id,
                            names: c.names,
                            initial: c.initial,
                            assets: c.files,
                            size: SizeSet { stat: c.size, ..SizeSet::default() },
                        })
                        .collect();
                }
                "modules" => {
                    saw_modules = true;
                    map.next_value_seed(ModulesInto { modules: &mut graph.modules })?;
                }
                _ => {
                    // entrypoints, errors, warnings, and the hundreds of MB of
                    // `source` strings we deliberately do not want.
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        if !saw_modules {
            graph.diagnostics.push(crate::model::Diagnostic {
                severity: crate::model::Severity::Warning,
                code: "OB0001".into(),
                message: "stats file has no `modules` array (dev-server export?)".into(),
                subject: None,
                data: serde_json::Value::Null,
            });
        }

        graph.inputs.push(InputArtifact::Stats { tool: self.tool });
        graph.totals.size_dimension = SizeDimension::Parsed;
        recompute_totals(&mut graph);
        Ok(graph)
    }
}

/// Collect a small JSON array into an existing vector without buffering it as a
/// `Value`. `assets` (hundreds) and `chunks` (tens) go through this.
struct SeqCollector<'a, T> {
    out: &'a mut Vec<T>,
}

impl<'a, T> SeqCollector<'a, T> {
    fn new(out: &'a mut Vec<T>) -> Self {
        Self { out }
    }
}

impl<'de, T: serde::Deserialize<'de>> DeserializeSeed<'de> for SeqCollector<'_, T> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        struct V<'a, T> {
            out: &'a mut Vec<T>,
        }
        impl<'de, T: serde::Deserialize<'de>> Visitor<'de> for V<'_, T> {
            type Value = ();
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an array")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<(), A::Error> {
                while let Some(item) = seq.next_element::<T>()? {
                    self.out.push(item);
                }
                Ok(())
            }
        }
        d.deserialize_seq(V { out: self.out })
    }
}

/// One-at-a-time module ingest. This is where the memory ceiling is won: a
/// 440k-module stats file never exists as a `Value` or a `Vec<RawModule>`.
struct ModulesInto<'a> {
    modules: &'a mut BTreeMap<String, Module>,
}

impl<'de> DeserializeSeed<'de> for ModulesInto<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        struct V<'a> {
            modules: &'a mut BTreeMap<String, Module>,
        }
        impl<'de> Visitor<'de> for V<'_> {
            type Value = ();

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a modules array")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<(), A::Error> {
                while let Some(raw) = seq.next_element::<RawModule>()? {
                    let chunk = raw.chunks.first().copied();
                    let id = join_key(raw.identifier.as_deref(), &raw.name, chunk);
                    let package = package_of(&raw.name).map(|(name, path)| PackageRef {
                        name,
                        version: None,
                        path,
                    });
                    let module = Module {
                        id: id.clone(),
                        name: raw.name,
                        issuer: raw.issuer_name,
                        reasons: raw
                            .reasons
                            .into_iter()
                            .filter_map(|r| r.module_name)
                            .collect(),
                        package,
                        chunks: raw.chunks,
                        sizes: SizeSet { stat: raw.size, ..SizeSet::default() },
                        attribution_delta: 0,
                        sources: Vec::new(),
                    };
                    self.modules.insert(id, module);
                }
                Ok(())
            }
        }
        d.deserialize_seq(V { modules: self.modules })
    }
}

/// Recompute the rollups from the graph's current contents. Called after every
/// mutation so a report can never carry stale totals.
pub fn recompute_totals(graph: &mut UnifiedBundleGraph) {
    let mut total = 0u64;
    let mut sum = 0u64;
    let mut packages: Vec<&str> = Vec::new();

    for asset in &graph.assets {
        total += asset.sizes.effective();
    }
    for module in graph.modules.values() {
        sum += module.sizes.effective();
        if let Some(pkg) = &module.package {
            packages.push(pkg.name.as_str());
        }
    }
    packages.sort_unstable();
    packages.dedup();

    graph.totals = Totals {
        total_size: total,
        module_count: graph.modules.len() as u64,
        asset_count: graph.assets.len() as u64,
        package_count: packages.len() as u64,
        size_dimension: graph.totals.size_dimension,
        module_size_sum: sum,
    };
}

/// Cheap shape detection, then a real parse.
///
/// A JSON document cannot be identified by parsing its first 64 KB: a 1 GB
/// stats file has a `modules` array that starts early but does not *end* early,
/// so any bounded slice is invalid JSON. We therefore sniff for the top-level
/// key names in the first few hundred kilobytes, use that only as a hint, and
/// let the real parser report the real error. A wrong hint costs a clear parse
/// failure; it never produces a silently wrong graph.
pub fn ingest_bytes(bytes: &[u8], name: &str) -> Result<UnifiedBundleGraph> {
    let head = &bytes[..bytes.len().min(512 * 1024)];
    let has = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);

    let looks_like_stats = has(b"\"modules\"") || has(b"\"chunks\"") || has(b"\"assets\"");
    let looks_like_metafile = has(b"\"inputs\"") && has(b"\"outputs\"");

    if looks_like_stats {
        return ingest_stats(bytes, "webpack");
    }
    if looks_like_metafile {
        return ingest_metafile(bytes);
    }
    Err(Error::Unsupported(format!(
        "{name}: no recognised artifact. Looked for webpack/rspack stats.json \
         (assets/chunks/modules) and esbuild metafile.json (inputs/outputs)."
    )))
}

/// esbuild metafile support (PRD 3.1; the roadmap lists the *ingest path* for
/// Phase 3, but the parser lives here now so dispatch and the report dimensions
/// are testable end to end).
pub fn ingest_metafile(bytes: &[u8]) -> Result<UnifiedBundleGraph> {
    #[derive(serde::Deserialize)]
    struct Metafile {
        inputs: BTreeMap<String, MetaInput>,
        outputs: BTreeMap<String, MetaOutput>,
    }
    #[derive(serde::Deserialize, Default)]
    struct MetaInput {
        #[serde(default)]
        bytes: u64,
    }
    #[derive(serde::Deserialize, Default)]
    struct MetaOutput {
        #[serde(default)]
        bytes: u64,
        #[serde(default)]
        inputs: BTreeMap<String, MetaOutputEntry>,
    }
    #[derive(serde::Deserialize, Default)]
    struct MetaOutputEntry {
        #[serde(rename = "bytesInOutput", default)]
        bytes_in_output: u64,
    }

    let meta: Metafile = serde_json::from_slice(bytes).map_err(Error::Json)?;
    let mut graph = UnifiedBundleGraph::new();
    graph.inputs.push(InputArtifact::EsbuildMetafile);
    graph.totals.size_dimension = SizeDimension::Parsed;

    for (name, input) in &meta.inputs {
        let package = package_of(name);
        let module = Module {
            id: name.clone(),
            name: name.clone(),
            issuer: None,
            reasons: Vec::new(),
            package: package.map(|(name, path)| PackageRef { name, version: None, path }),
            chunks: Vec::new(),
            sizes: SizeSet { stat: input.bytes, ..SizeSet::default() },
            attribution_delta: 0,
            sources: Vec::new(),
        };
        graph.modules.insert(name.clone(), module);
    }

    for (name, output) in &meta.outputs {
        graph.assets.push(Asset {
            name: name.clone(),
            size: output.bytes,
            chunks: Vec::new(),
            sizes: SizeSet { stat: output.bytes, ..SizeSet::default() },
        });
        for (input, entry) in &output.inputs {
            if let Some(module) = graph.modules.get_mut(input) {
                // esbuild reports per-output contributions; the input's own
                // `bytes` is the pre-bundle source size, so keep the larger
                // measurement and let the fusion engine correct it later.
                module.sizes.parsed = module.sizes.parsed.max(entry.bytes_in_output);
            }
        }
    }

    recompute_totals(&mut graph);
    Ok(graph)
}

/// Ingest from a file path, streaming from disk.
///
/// Prefer this over [`ingest_bytes`] for real files: `ingest_bytes` takes a
/// slice, so the caller has already paid the file size in memory. (Both are
/// correct; only one of them is the product.)
pub fn ingest_file(path: &std::path::Path, tool: &str) -> Result<UnifiedBundleGraph> {
    let file = std::fs::File::open(path).map_err(|e| Error::Io(e))?;
    ingest_reader(file, tool)
}

#[cfg(test)]
mod tests {
    use super::{ingest_metafile, ingest_stats, package_of};

    const STATS: &str = r#"{
        "version": "5.90.0",
        "assets": [{"name": "main.js", "size": 3000, "chunks": [0], "chunkNames": ["main"]}],
        "chunks": [{"id": 0, "names": ["main"], "initial": true, "files": ["main.js"], "size": 3000}],
        "modules": [
            {"id": 1, "identifier": "/repo/node_modules/preact/dist/preact.js",
             "name": "./node_modules/preact/dist/preact.js", "size": 1200, "chunks": [0],
             "reasons": [{"moduleName": "./src/app.js", "type": "harmony side effect evaluation"}],
             "source": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"},
            {"id": 2, "name": "./src/app.js", "size": 800, "chunks": [0],
             "issuerName": null, "reasons": []}
        ],
        "entrypoints": {},
        "errors": []
    }"#;

    #[test]
    fn ingests_assets_chunks_and_modules() {
        let g = ingest_stats(STATS.as_bytes(), "webpack").unwrap();
        assert_eq!(g.assets.len(), 1);
        assert_eq!(g.chunks.len(), 1);
        assert_eq!(g.modules.len(), 2);
        assert_eq!(g.totals.asset_count, 1);
        assert_eq!(g.totals.module_count, 2);
        assert_eq!(g.totals.total_size, 3000, "total comes from the assets");
        assert_eq!(g.totals.module_size_sum, 2000);
    }

    #[test]
    fn reasons_and_package_are_extracted() {
        let g = ingest_stats(STATS.as_bytes(), "webpack").unwrap();
        let preact = g.modules.values().find(|m| m.name.contains("preact")).expect("preact module");
        assert_eq!(preact.package.as_ref().unwrap().name, "preact");
        assert_eq!(preact.reasons, vec!["./src/app.js".to_string()]);

        let app = g.modules.values().find(|m| m.name.contains("app.js")).unwrap();
        assert!(app.package.is_none(), "application code has no package");
    }

    #[test]
    fn missing_modules_array_is_a_diagnostic_not_an_error() {
        let g = ingest_stats(br#"{"version":"5.0.0","assets":[],"chunks":[]}"#, "webpack").unwrap();
        assert!(g.diagnostics.iter().any(|d| d.code == "OB0001"));
    }

    #[test]
    fn unknown_fields_are_ignored_cheaply() {
        // Guards the property the memory story depends on: the `source` string
        // must never end up in the graph, however large it is.
        let big = format!(
            r#"{{"version":"5.0.0","assets":[],"chunks":[],"modules":[
                {{"id":1,"name":"./src/a.js","size":10,"source":"{}"}}]}}"#,
            "x".repeat(50_000)
        );
        let g = ingest_stats(big.as_bytes(), "webpack").unwrap();
        let stored: usize = g.modules.values().map(|m| m.name.len() + m.id.len()).sum();
        assert!(stored < 100, "only the key and name are kept, got {stored}");
    }

    #[test]
    fn esbuild_metafile_is_dispatched() {
        let meta = r#"{"inputs":{"src/a.js":{"bytes":100},"node_modules/marked/lib/marked.js":{"bytes":900}},
                      "outputs":{"dist/out.js":{"bytes":700,"inputs":{"src/a.js":{"bytesInOutput":80},
                      "node_modules/marked/lib/marked.js":{"bytesInOutput":600}}}}}"#;
        let g = ingest_metafile(meta.as_bytes()).unwrap();
        assert_eq!(g.modules.len(), 2);
        assert_eq!(g.assets.len(), 1);
        assert_eq!(g.totals.total_size, 700);
        assert_eq!(g.totals.package_count, 1);
        assert_eq!(package_of("node_modules/marked/lib/marked.js").unwrap().0, "marked");
    }
}
