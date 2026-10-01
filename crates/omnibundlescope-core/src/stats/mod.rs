//! stats ingest: webpack / rspack `stats.json` streaming ingest.
//!
//! Contract: ingest the file **without materialising the document**. The
//! reference implementation parses `stats.json` with a custom
//! `serde_json` seed so the 160k-440k module arrays never exist as one
//! `serde_json::Value`. Measured on the reference machine (see
//! `docs/en/01-evidence.md`):
//!
//! | input | Node `JSON.parse` floor | `OmniBundleScope` streaming target |
//! |---|---|---|
//! | 381 MB / 160,728 modules | 0.94 s / 919 MB RSS | <= 2 s / <= 200 MB |
//! | 1,049 MB / 441,976 modules | (176.3 s / 1,437 MB for the full WBA run) | <= 3 s / <= 400 MB |
//!
//! `stats::ingest` (stats ingest) turns a `stats.json` into the unified graph without
//! ever materialising the document; `recompute_totals` keeps the rollups honest
//! after every mutation.

mod ingest;

pub use ingest::{
    ingest_bytes, ingest_file, ingest_metafile, ingest_reader, ingest_stats, recompute_totals,
};

/// Identity fields we keep for every module. Unknown keys are ignored without
/// allocation (serde's `IgnoredAny`), which is what keeps the parse cheap on
/// real-world stats files that embed module sources.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawModule {
    pub identifier: Option<String>,
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub chunks: Vec<u32>,
    pub issuer_name: Option<String>,
    #[serde(default)]
    pub reasons: Vec<RawReason>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawReason {
    #[serde(default)]
    pub module_name: Option<String>,
    #[serde(rename = "type", default)]
    pub reason_type: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAsset {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub chunks: Vec<u32>,
    #[serde(default)]
    pub chunk_names: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChunk {
    pub id: u32,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub initial: bool,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub size: u64,
}

/// The stable join key used by every other areas.
///
/// Rules (also in `docs/contracts/unified-graph.md`):
/// 1. `identifier` when present — it is the only field webpack keeps stable
///    across builds.
/// 2. otherwise `chunk-scoped name`: `"{chunk_id}:{name}"` because the same
///    name can legally appear in several chunks.
/// 3. never a hash of anything mutable; the key must survive a rebuild with no
///    source change.
pub fn join_key(identifier: Option<&str>, name: &str, chunk: Option<u32>) -> String {
    match identifier {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => match chunk {
            Some(c) => format!("{c}:{name}"),
            None => name.to_string(),
        },
    }
}

/// Extract the npm package a module belongs to from a stats `name` like
/// `./node_modules/preact/compat/src/index.js`, or `None` for application code.
///
/// Deliberately simple: a package is the segment (or `@scope/segment` pair)
/// immediately after a `node_modules/` component. Anything we cannot place is
/// application code, and the treemap labels it `<app>` rather than guessing.
pub fn package_of(name: &str) -> Option<(String, String)> {
    let normalized = name.replace('\\', "/");
    let idx = normalized.rfind("node_modules/")?;
    let rest = &normalized[idx + "node_modules/".len()..];
    let mut segments = rest.split('/');
    let first = segments.next()?;
    if first.is_empty() {
        return None;
    }
    let (pkg, path) = if first.starts_with('@') {
        match segments.next() {
            Some(second) if !second.is_empty() => {
                (format!("{first}/{second}"), format!("node_modules/{first}/{second}"))
            }
            _ => return None,
        }
    } else {
        (first.to_string(), format!("node_modules/{first}"))
    };
    Some((pkg, path))
}

#[cfg(test)]
mod tests {
    use super::{join_key, package_of};

    #[test]
    fn identifier_wins() {
        assert_eq!(join_key(Some("/repo/a.js"), "a.js", Some(3)), "/repo/a.js");
    }

    #[test]
    fn name_is_chunk_scoped() {
        assert_eq!(join_key(None, "./src/a.js", Some(3)), "3:./src/a.js");
        assert_eq!(join_key(None, "./src/a.js", Some(9)), "9:./src/a.js");
    }

    #[test]
    fn packages_are_found_behind_node_modules() {
        assert_eq!(
            package_of("./node_modules/preact/compat/src/index.js"),
            Some(("preact".into(), "node_modules/preact".into()))
        );
        assert_eq!(
            package_of("./node_modules/@scope/pkg/dist/i.js"),
            Some(("@scope/pkg".into(), "node_modules/@scope/pkg".into()))
        );
        assert_eq!(package_of("./src/app.js"), None);
    }

    #[test]
    fn windows_separators_are_handled() {
        assert_eq!(
            package_of(".\\node_modules\\marked\\lib\\marked.js"),
            Some(("marked".into(), "node_modules/marked".into()))
        );
    }
}
