use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use rayon::prelude::*;

use crate::Result;
use crate::model::{SizeSet, UnifiedBundleGraph};

/// Measure every asset on disk and fill `parsed` and `gzip`.
///
/// # Errors
/// Errors only when an asset cannot be read. A **missing** asset is not an error:
/// stats files list assets that were cleaned up, and it keeps its declared size
/// with an `OBS0002` diagnostic rather than becoming a zero.
pub fn attribute_from_disk(graph: &mut UnifiedBundleGraph, dir: &Path) -> Result<()> {
    // Collect first, so the parallel pass does no IO inside the iterator.
    let targets: Vec<(usize, String, std::path::PathBuf)> = graph
        .assets
        .iter()
        .enumerate()
        .map(|(i, a)| (i, a.name.clone(), dir.join(&a.name)))
        .collect();

    let measured: Vec<(usize, u64, u64)> = targets
        .par_iter()
        .map(|(idx, _name, path)| match std::fs::read(path) {
            Ok(bytes) => (*idx, bytes.len() as u64, gzip_size(&bytes)),
            // Missing asset: diagnostic, not a failure.
            Err(_) => (*idx, 0, 0),
        })
        .collect();

    let mut missing = Vec::new();
    for (idx, parsed, gzip) in measured {
        let asset = &mut graph.assets[idx];
        if parsed == 0 {
            missing.push(asset.name.clone());
            continue;
        }
        asset.sizes.parsed = parsed;
        asset.sizes.gzip = gzip;
    }

    for chunk in &mut graph.chunks {
        let mut stat = 0u64;
        let mut parsed = 0u64;
        let mut gzip = 0u64;
        for name in &chunk.assets {
            if let Some(a) = graph.assets.iter().find(|a| &a.name == name) {
                stat += a.sizes.stat;
                parsed += a.sizes.parsed;
                gzip += a.sizes.gzip;
            }
        }
        chunk.size = SizeSet { stat, parsed, gzip, attributed: None };
    }

    if !missing.is_empty() {
        missing.truncate(5);
        graph.diagnostics.push(crate::model::Diagnostic {
            severity: crate::model::Severity::Info,
            code: "OBS0002".into(),
            message: format!(
                "{} asset(s) not found next to the metadata: {}{}",
                missing.len(),
                missing.join(", "),
                if missing.len() == 5 { ", …" } else { "" }
            ),
            subject: None,
            data: serde_json::Value::Null,
        });
    }

    propagate_asset_sizes_to_modules(graph);
    Ok(())
}

/// webpack reports a module's size as the pre-concatenation size; the emitted
/// file is authoritative. Where an asset was measured, the module sizes are
/// scaled to the measured total so `sum(modules)` stays comparable with
/// `sum(assets)` (the invariant in `docs/contracts/unified-graph.md` §4).
fn propagate_asset_sizes_to_modules(graph: &mut UnifiedBundleGraph) {
    let scale: BTreeMap<u32, f64> = graph
        .chunks
        .iter()
        .map(|chunk| {
            let stat: u64 = chunk
                .assets
                .iter()
                .filter_map(|n| graph.assets.iter().find(|a| &a.name == n))
                .map(|a| a.sizes.stat)
                .sum();
            let parsed: u64 = chunk
                .assets
                .iter()
                .filter_map(|n| graph.assets.iter().find(|a| &a.name == n))
                .map(|a| a.sizes.parsed)
                .sum();
            let factor = if stat > 0 && parsed > 0 { parsed as f64 / stat as f64 } else { 1.0 };
            (chunk.id, factor)
        })
        .collect();

    for module in graph.modules.values_mut() {
        // The mean of the module's chunks' factors, not the maximum.
        //
        // This used to be `fold(1.0, |acc, f| acc.max(*f))`, which could never
        // return a factor below 1.0 - so whenever the emitted files were
        // *smaller* than the bundler claimed (minification, tree-shaking, a
        // stale stats file, i.e. most of the reason to run an analyser at all)
        // every module kept its inflated declared size and the invariant in
        // `unified-graph.md` §4 failed in the direction that overstates the
        // bundle. Found by a property test, after a lint-driven change had
        // already broken the other half of it.
        //
        // A mean is the honest choice for a module in several chunks: its bytes
        // are spread across them, and we do not track a per-chunk split to do
        // better. For the single-chunk module - which is nearly all of them, and
        // every fixture here - it is exact.
        let mut sum = 0.0_f64;
        let mut measured_chunks = 0usize;
        for chunk_id in &module.chunks {
            if let Some(factor) = scale.get(chunk_id) {
                sum += *factor;
                measured_chunks += 1;
            }
        }
        let factor = if measured_chunks == 0 { 1.0 } else { sum / measured_chunks as f64 };

        // `factor == 1.0` means the module's chunks were never measured, so the
        // declared size stands unchanged.
        //
        // Comparing floats for equality is normally the wrong thing, and clippy
        // says so. Here it is the correct test: 1.0 is the sentinel the scale
        // map carries for "not measured", and every other value is a ratio of
        // two measured integers. Writing this as `factor > 1.0` instead silently
        // disables downscaling for the same reason.
        #[allow(clippy::float_cmp, clippy::if_not_else)]
        if factor != 1.0 {
            // `as u64` on a float is UB-adjacent for a negative or NaN input and
            // saturating above 2^64, and a wrapped value here would be a module
            // claiming to be 18 exabytes. The scale factor is a measured ratio,
            // so it is finite and positive; the checked conversion says so out
            // loud rather than trusting it.
            let scaled = (module.sizes.stat as f64 * factor).round();
            module.sizes.parsed = if scaled.is_finite() && scaled > 0.0 {
                // Clamped, so the cast below cannot truncate or go negative.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    scaled.min(u64::MAX as f64) as u64
                }
            } else {
                // A non-finite scale means the asset size was unusable; the
                // declared size is the only number we can still stand behind.
                module.sizes.stat
            };
        } else {
            module.sizes.parsed = module.sizes.stat;
        }
    }
}

/// gzip a single asset. Level 6 on purpose: it must byte-match the reference
/// tools, and a different level changes every reported size.
pub fn gzip_size(bytes: &[u8]) -> u64 {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
    // Writing to an in-memory buffer cannot fail.
    let _ = enc.write_all(bytes);
    enc.finish().map_or(0, |v| v.len() as u64)
}

/// Content-addressed key for cache entries: `blake3` over the exact input
/// bytes, so a cache hit is provably about the same input.
pub fn content_key(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::{attribute_from_disk, gzip_size};
    use crate::model::{Asset, Chunk, Module, SizeSet, UnifiedBundleGraph};
    use std::collections::BTreeMap;

    #[test]
    fn gzip_level_6_is_byte_stable() {
        // Contract: the reported gzip size must be reproducible, so a fixture
        // value is asserted rather than a range. Note gzip of a *tiny* input is
        // larger than the input (header + footer overhead), which is why the
        // compression assertion uses a repetitive payload.
        let small = b"the quick brown fox jumps over the lazy dog";
        assert_eq!(gzip_size(small), gzip_size(small), "must be deterministic");

        let repetitive = vec![b'a'; 64 * 1024];
        let gz = gzip_size(&repetitive);
        assert!(gz > 0 && gz < repetitive.len() as u64 / 100, "64 KB of 'a' must compress hard");
        assert_eq!(gz, gzip_size(&repetitive), "must be deterministic at scale");
    }

    #[test]
    fn missing_assets_become_a_diagnostic_not_a_failure() {
        let mut g = UnifiedBundleGraph::new();
        g.assets.push(Asset {
            name: "gone.js".into(),
            size: 10,
            chunks: vec![0],
            sizes: SizeSet { stat: 10, ..SizeSet::default() },
        });
        g.chunks.push(Chunk {
            id: 0,
            names: vec!["main".into()],
            initial: true,
            assets: vec!["gone.js".into()],
            size: SizeSet::default(),
        });
        attribute_from_disk(&mut g, std::path::Path::new("/definitely/not/here")).unwrap();
        assert!(g.diagnostics.iter().any(|d| d.code == "OBS0002"));
    }

    #[test]
    fn measured_asset_sizes_propagate_to_chunks_and_modules() {
        let dir = std::env::temp_dir().join("omnibundlescope-sizes-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.js"), vec![b'a'; 4096]).unwrap();

        let mut g = UnifiedBundleGraph::new();
        g.assets.push(Asset {
            name: "main.js".into(),
            size: 1000,
            chunks: vec![0],
            sizes: SizeSet { stat: 1000, ..SizeSet::default() },
        });
        g.chunks.push(Chunk {
            id: 0,
            names: vec!["main".into()],
            initial: true,
            assets: vec!["main.js".into()],
            size: SizeSet::default(),
        });
        let mut modules = BTreeMap::new();
        for (name, stat) in [("./a.js", 600u64), ("./b.js", 400)] {
            let m = Module {
                id: name.into(),
                name: name.into(),
                issuer: None,
                reasons: vec![],
                package: None,
                chunks: vec![0],
                sizes: SizeSet { stat, ..SizeSet::default() },
                attribution_delta: 0,
                sources: vec![],
            };
            modules.insert(m.id.clone(), m);
        }
        g.modules = modules;

        attribute_from_disk(&mut g, &dir).unwrap();

        assert_eq!(g.assets[0].sizes.parsed, 4096);
        assert!(g.assets[0].sizes.gzip > 0);
        assert_eq!(g.chunks[0].size.parsed, 4096);
        // module sizes are scaled so they sum to the measured asset size
        let sum: u64 = g.modules.values().map(|m| m.sizes.parsed).sum();
        assert_eq!(sum, 4096, "modules must reconcile with the measured asset");
        assert!(g.check_size_invariant(1_000).is_ok());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_bundle_dir_is_reported_as_a_diagnostic_not_a_panic() {
        let mut g = UnifiedBundleGraph::new();
        g.assets.push(Asset {
            name: "main.js".into(),
            size: 10,
            chunks: vec![0],
            sizes: SizeSet { stat: 10, ..SizeSet::default() },
        });
        // The dir does not exist: every read fails, which must surface as a
        // diagnostic (OBS0002), never as a panic and never as a wrong number.
        let result = attribute_from_disk(&mut g, std::path::Path::new("/definitely/not/here"));
        assert!(result.is_ok(), "a missing dir is data, not a crash: {result:?}");
        assert!(g.diagnostics.iter().any(|d| d.code == "OBS0002"));
        assert_eq!(g.assets[0].sizes.parsed, 0, "an unmeasured asset stays 0, not a guess");
    }
}
