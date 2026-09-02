//! WS-2: size attribution and the treemap tree.
//!
//! Three dimensions, in the order the report prefers them:
//! `attributed` (source map ground truth) > `parsed` (measured from the emitted
//! file) > `stat` (what the bundler claimed).
//!
//! The reference measurement that motivates the parallel design: gzipping the
//! 25,600 assets of a large build was **2,177 ms serial vs 342 ms with rayon
//! (6.4x)** at 15 MB RSS. That is the shape of the win — not "faster parsing",
//! but "no per-item process, no V8 heap ceiling, actual parallelism".

/// Per-asset measured sizes, filled in by the CLI (WS-6) or by a caller that
/// can hand us file contents (WS-2 tests). Kept as a trait so the core stays
/// IO-free.
pub trait AssetSource {
    fn asset_names(&self) -> Vec<String>;
    fn raw_bytes(&self, name: &str) -> Option<&[u8]>;
}

/// gzip a single asset. Level 6 on purpose: it must byte-match the reference
/// tools, and a different level changes every reported size.
pub fn gzip_size(bytes: &[u8]) -> u64 {
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    let mut enc = GzEncoder::new(Vec::new(), Compression::new(6));
    // Writing to an in-memory buffer cannot fail.
    let _ = enc.write_all(bytes);
    enc.finish().map_or(0, |v| v.len() as u64)
}

/// Content-addressed key for cache entries: `blake3` over the exact input
/// bytes, so a cache hit is provably about the same input.
pub fn content_key(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}
