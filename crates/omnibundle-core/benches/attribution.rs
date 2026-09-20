//! Source map benchmarks: the VLQ decode and the per-source attribution pass.
//!
//! These are the two stages where `source-map-explorer` spends 9.4 minutes on a
//! 50,000-source map, so they are the two worth watching. The published numbers
//! come from the shipped binary on a pinned map; these exist to see which stage
//! moved when something changes.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use omnibundle_core::sourcemap::{ParsedSourceMap, parse_bytes};

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn vlq(value: i64) -> String {
    let mut v = if value < 0 { ((-value) << 1) | 1 } else { value << 1 };
    let mut out = String::new();
    loop {
        let mut digit = usize::try_from(v & 31).unwrap_or(0);
        v >>= 5;
        if v > 0 {
            digit |= 32;
        }
        out.push(B64[digit] as char);
        if v == 0 {
            return out;
        }
    }
}

/// A map with `sources` sources and a segment per source, spread over
/// `generated_bytes` of output. Shaped like a real one: many sources, segments
/// in order, a tail after the last mapping.
fn generate_map(sources: usize, generated_bytes: usize) -> String {
    use std::fmt::Write as _;

    let per_source = generated_bytes / sources;
    let per_source_vlq = vlq(i64::try_from(per_source).unwrap_or(i64::MAX));
    let mut mappings = String::with_capacity(sources * 12);
    let mut list = Vec::with_capacity(sources);
    for i in 0..sources {
        list.push(format!("\"webpack://fixture/./src/module-{i}.ts\""));
        if i > 0 {
            mappings.push(',');
        }
        // A segment is (generatedColumn, sourceIndex, originalLine,
        // originalColumn), all as VLQ deltas. The first segment establishes the
        // origin, so its source and line deltas are zero; hardcoding 1 there too
        // is what made an early version of this generator emit a map pointing one
        // past its own sources array.
        let _ = write!(
            mappings,
            "{}{}{}{}",
            per_source_vlq,
            vlq(i64::from(i > 0)),
            vlq(i64::from(i > 0)),
            vlq(0)
        );
    }
    format!(
        r#"{{"version":3,"file":"bundle.js","sources":[{}],"sourcesContent":[],"names":[],"mappings":"{}"}}"#,
        list.join(","),
        mappings
    )
}

fn decode_and_attribute(c: &mut Criterion) {
    let mut group = c.benchmark_group("sourcemap");

    for sources in [1_000usize, 10_000, 50_000] {
        let json = generate_map(sources, 20_000_000);
        let bytes = json.len() as u64;
        group.throughput(Throughput::Bytes(bytes));

        group.bench_with_input(BenchmarkId::new("parse", sources), &json, |b, json| {
            b.iter(|| {
                let map = parse_bytes(json.as_bytes()).expect("valid map");
                black_box(map.mappings.len())
            });
        });

        let parsed: ParsedSourceMap = parse_bytes(json.as_bytes()).expect("valid map");
        group.bench_with_input(BenchmarkId::new("attribute", sources), &parsed, |b, map| {
            b.iter(|| {
                let by_source = map.attribute_by_source_in(20_000_000);
                black_box(by_source.len())
            });
        });
        group.throughput(Throughput::Bytes(0));
    }

    group.finish();
}

criterion_group!(benches, decode_and_attribute);
criterion_main!(benches);
