//! WS-1: webpack / rspack `stats.json` streaming ingest.
//!
//! Contract: ingest the file **without materialising the document**. The
//! reference implementation parses `stats.json` with a custom
//! `serde_json` seed so the 160k-440k module arrays never exist as one
//! `serde_json::Value`. Measured on the reference machine (see
//! `docs/en/01-evidence.md`):
//!
//! | input | Node `JSON.parse` floor | `OmniBundle` streaming target |
//! |---|---|---|
//! | 381 MB / 160,728 modules | 0.94 s / 919 MB RSS | <= 2 s / <= 200 MB |
//! | 1,049 MB / 441,976 modules | (176.3 s / 1,437 MB for the full WBA run) | <= 3 s / <= 400 MB |
//!
//! Everything else about this module is still to be built; see
//! `docs/en/03-implementation-plan.md`.

/// Identity fields we keep for every module. Unknown keys are ignored without
/// allocation (serde's `IgnoredAny`), which is what keeps the parse cheap on
/// real-world stats files that embed module sources.
#[derive(Debug, Clone, serde::Deserialize)]
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
pub struct RawReason {
    #[serde(default)]
    pub module_name: Option<String>,
    #[serde(rename = "type", default)]
    pub reason_type: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
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

/// The stable join key used by every other workstream.
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

#[cfg(test)]
mod tests {
    use super::join_key;

    #[test]
    fn identifier_wins() {
        assert_eq!(join_key(Some("/repo/a.js"), "a.js", Some(3)), "/repo/a.js");
    }

    #[test]
    fn name_is_chunk_scoped() {
        assert_eq!(join_key(None, "./src/a.js", Some(3)), "3:./src/a.js");
        assert_eq!(join_key(None, "./src/a.js", Some(9)), "9:./src/a.js");
    }
}
