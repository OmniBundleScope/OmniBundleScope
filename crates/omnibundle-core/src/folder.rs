//! Ingest a build folder that has no bundler metadata.
//!
//! Most bundlers emit `dist/` and nothing else. Vite, Rollup, Parcel, tsup and
//! a plain esbuild `--outdir` all produce JavaScript and, if you asked for it,
//! `*.map` - and no `stats.json`, because there is no cross-bundler format for
//! one. Requiring bundler metadata to produce a report means the tool is
//! useless for exactly the projects most people build with, so this path reads
//! the output and its source maps, and nothing else.
//!
//! What is genuinely knowable here, and what is not:
//!
//! - **Sizes are measured, not declared.** Every byte on disk is real, so the
//!   `parsed` and `gzip` dimensions are exact and `stat` stays zero - there is
//!   nobody to declare anything.
//! - **Attribution comes from the source maps.** Bytes map back to original
//!   source paths without any bundler metadata, which is the whole reason source
//!   maps exist. The report groups by source file.
//! - **Ghost code cannot be detected.** A ghost is a module the bundler
//!   declared and shipped but no map explains; with no declared graph there is
//!   nothing to compare against, and reporting zero ghosts would be inventing a
//!   clean bill of health. [`crate::fusion::analyse_sources_only`] says so with a
//!   diagnostic instead.

use crate::error::{Error, Result};
use crate::model::{
    Asset, InputArtifact, PackageRef, Severity, SizeDimension, SizeSet, UnifiedBundleGraph,
};
use std::path::Path;

/// Files we read as assets. Everything else in the folder is either the source
/// map of one of these or not ours to interpret.
fn is_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| matches!(ext, "js" | "mjs" | "cjs" | "css" | "map" | "html"))
        && path.is_file()
}

/// A package name inferred from a source path, mirroring what the stats ingest
/// does for a `node_modules` path. Vite does not tell us the dependency version,
/// so `version` stays `None` rather than being invented.
pub fn package_of(path: &str) -> Option<(String, String)> {
    let idx = path.find("node_modules/")?;
    let rest = &path[idx + "node_modules/".len()..];
    let segment = rest.split(['/', '\\']).next()?;
    if segment.is_empty() {
        return None;
    }
    let root = format!("node_modules/{segment}");
    Some((segment.to_string(), root))
}

/// Build a graph from a directory's contents alone.
///
/// Walks the whole tree, because the interesting files are rarely at the top:
/// vite writes `dist/assets/index-a1b2c3.js`, parcel writes
/// `dist/index.<hash>.js` next to a `dist/*.map`, and a Next.js build nests
/// everything under `dist/static/chunks/`. Reading only the first level finds
/// `index.html` and nothing else, which is a report with one asset in it.
pub fn ingest(dir: &Path) -> Result<UnifiedBundleGraph> {
    let mut graph = UnifiedBundleGraph::new();
    graph.inputs.push(InputArtifact::DistFolder);

    let mut paths = Vec::new();
    collect_assets(dir, dir, &mut paths, 0);
    if paths.is_empty() {
        return Err(Error::Malformed(format!(
            "no build output found in {} (looked for .js, .mjs, .cjs, .css and .html)",
            dir.display()
        )));
    }
    paths.sort();
    for path in paths {
        let Ok(size) = std::fs::metadata(&path).map(|m| m.len()) else {
            continue;
        };
        if size == 0 {
            continue;
        }
        let name = path.strip_prefix(dir).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        graph.assets.push(Asset {
            name,
            size,
            chunks: Vec::new(),
            // No declared size: `stat` stays zero so `effective()` resolves to the
            // measured `parsed` once the disk pass runs.
            sizes: SizeSet { stat: 0, parsed: 0, gzip: 0, attributed: None },
        });
    }

    graph.assets.sort_by(|a, b| a.name.cmp(&b.name));
    graph.totals.size_dimension = SizeDimension::Parsed;
    Ok(graph)
}

/// How deep to walk. A build output is shallow, but `node_modules` can be
/// inside `dist/` for some tools, and an unbounded walk of a self-referential
/// symlink loop is a hang rather than an error.
const MAX_DEPTH: usize = 12;

/// Collect asset paths under `dir`, depth-first, skipping source maps.
///
/// Source maps are the *input* to attribution, not a build output in their own
/// right: counting them as assets would double every byte in the build, which
/// is the kind of error a treemap hides beautifully.
fn collect_assets(dir: &Path, root: &Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut dirs = Vec::new();
    for entry in entries.filter_map(std::result::Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
            continue;
        }
        if !is_asset(&path) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("map") {
            continue;
        }
        if path == root {
            continue;
        }
        out.push(path);
    }
    for dir in dirs {
        collect_assets(&dir, root, out, depth + 1);
    }
}

/// The package a source path belongs to, as a shared reference.
pub fn package_ref(path: &str) -> Option<std::sync::Arc<PackageRef>> {
    package_of(path)
        .map(|(name, root)| std::sync::Arc::new(PackageRef { name, version: None, path: root }))
}

/// The diagnostic that says what this mode cannot tell you.
pub fn no_declared_graph_diagnostic() -> crate::model::Diagnostic {
    crate::model::Diagnostic {
        severity: Severity::Info,
        code: "OB0051".into(),
        message: "no bundler metadata in this folder: sizes are measured and sources are attributed \
                  from the source maps, but ghost code cannot be detected (it needs a declared module \
                  graph to compare against)"
            .into(),
        subject: None,
        data: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::{ingest, package_of};
    use crate::model::SizeDimension;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("omnibundle-folder-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn a_folder_of_javascript_becomes_assets() {
        let dir = temp_dir("assets");
        fs::write(dir.join("index.js"), b"console.log(1)").unwrap();
        fs::write(dir.join("index.css"), b"body{}").unwrap();
        // A source map is an input to attribution, not an asset: counting it
        // would double the reported bundle size.
        fs::write(dir.join("index.js.map"), b"{}").unwrap();
        fs::write(dir.join("README.txt"), b"not an asset").unwrap();

        let graph = ingest(&dir).expect("folder ingests");
        let names: Vec<&str> = graph.assets.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["index.css", "index.js"]);
        assert_eq!(graph.totals.size_dimension, SizeDimension::Parsed);
        assert!(graph.assets.iter().all(|a| a.sizes.stat == 0));
    }

    #[test]
    fn the_whole_tree_is_walked_because_vite_nests_its_output() {
        // The shape vite actually emits. A test that only writes files at the top
        // level passes while the tool reports one asset for a real build, because
        // everything vite emits lives under dist/assets/.
        let dir = temp_dir("vite");
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::create_dir_all(dir.join("static/chunks")).unwrap();
        fs::write(dir.join("index.html"), b"<!doctype html>").unwrap();
        fs::write(dir.join("assets/index-a1b2c3.js"), b"console.log(1)").unwrap();
        fs::write(dir.join("assets/index-a1b2c3.js.map"), b"{}").unwrap();
        fs::write(dir.join("assets/index-d4e5.css"), b"body{}").unwrap();
        fs::write(dir.join("static/chunks/main.e6f7.js"), b"console.log(2)").unwrap();

        let graph = ingest(&dir).expect("a vite build ingests");
        let names: Vec<&str> = graph.assets.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "assets/index-a1b2c3.js",
                "assets/index-d4e5.css",
                "index.html",
                "static/chunks/main.e6f7.js",
            ],
            "the map is an input to attribution, not an asset; the rest are all outputs"
        );
    }

    #[test]
    fn an_empty_folder_is_an_error_not_an_empty_report() {
        let dir = temp_dir("empty");
        let err = ingest(&dir).expect_err("an empty folder has nothing to report");
        assert!(err.to_string().contains("no build output"), "{err}");
    }

    #[test]
    fn package_names_come_out_of_a_node_modules_path() {
        assert_eq!(
            package_of("node_modules/react/index.js"),
            Some(("react".to_string(), "node_modules/react".to_string()))
        );
        assert_eq!(
            package_of("/repo/node_modules/@scope/pkg/dist/x.js"),
            Some(("@scope".to_string(), "node_modules/@scope".to_string()))
        );
        assert_eq!(package_of("./src/app.ts"), None);
    }
}
