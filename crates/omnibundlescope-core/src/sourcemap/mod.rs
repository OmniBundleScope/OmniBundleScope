use std::collections::HashMap;
use std::io::Read;

use crate::{Error, Result};

/// Source Map v3 ingest (source maps).
///
/// Why this is not a `serde_json` job: source maps are dominated by one very
/// long `mappings` string plus `sourcesContent` blobs, and the *attribution* —
/// not the parse — is where the reference tool falls over. Measured on the
/// reference machine: `source-map-explorer` needs 0.25 s for a 97 KB real map
/// but **18.4 s at 10k sources and 562 s at 50k sources** (5x data → 30.6x
/// time), which matches the upstream report that `getWebTreeMapData` dominates.
///
/// So the design is: parse the JSON once with serde (fast, and the strings are
/// what they are), then decode the VLQ `mappings` **in place into a flat
/// `Vec<Mapping>`** with no per-mapping `String` and no per-file length string —
/// the exact allocation pattern the upstream issue blames. Per-file attribution
/// is then a single pass over sorted mappings (`O(n)`), not a nested walk.
/// One decoded mapping entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub generated_offset: u32,
    pub source_index: u32,
    pub original_line: u32,
    pub original_column: u32,
    pub name_index: Option<u32>,
}

/// The parsed, memory-flattened form of a source map.
#[derive(Debug, Clone, Default)]
pub struct ParsedSourceMap {
    pub file: Option<String>,
    pub sources: Vec<String>,
    pub names: Vec<String>,
    pub sources_content: Vec<Option<String>>,
    /// Sorted by `generated_offset`; one entry per segment.
    pub mappings: Vec<Mapping>,
}

#[derive(serde::Deserialize, Default)]
struct RawMap {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default, rename = "sourcesContent")]
    sources_content: Vec<Option<String>>,
    #[serde(default)]
    names: Vec<String>,
    #[serde(default)]
    mappings: String,
    #[serde(default)]
    sections: Vec<RawSection>,
}

#[derive(serde::Deserialize, Default)]
struct RawSection {
    #[serde(default)]
    offset: RawOffset,
    #[serde(default)]
    map: Box<RawMap>,
}

#[derive(serde::Deserialize, Default)]
struct RawOffset {
    #[serde(default)]
    line: u32,
    #[serde(default)]
    /// Kept for the report's drill-down; the attribution itself is offset-based.
    #[allow(dead_code)]
    column: u32,
}

/// Parse a source map from anything that implements [`std::io::Read`].
///
/// Streaming matters for the same reason it does for stats: a 36 MB map read
/// with `fs::read` puts 36 MB on the floor before we decode anything, and the
/// the source-map-explorer baseline baseline shows this workload reaching 642 MB.
pub fn parse_reader<R: Read>(reader: R) -> Result<ParsedSourceMap> {
    let raw: RawMap = serde_json::from_reader(std::io::BufReader::with_capacity(
        1 << 20,
        crate::bom::BomSkip::new(reader),
    ))
    .map_err(Error::Json)?;

    if raw.version != 0 && raw.version != 3 {
        return Err(Error::Unsupported(format!(
            "source map version {} (only v3 is supported)",
            raw.version
        )));
    }

    // indexed maps: concatenate the sections with their line/column offsets
    if !raw.sections.is_empty() {
        let mut merged = ParsedSourceMap { file: raw.file, ..ParsedSourceMap::default() };
        for section in raw.sections {
            let base = base_offset(&section.map.mappings, section.offset.line);
            let sub = decode(&section.map.mappings, section.map.sources.len())?;
            // A source-map v3 `sources` index is a u32 on the wire; a section
            // list that would need more than 2^32 sources is not a map, and
            // saturating keeps the index inside the field rather than wrapping.
            let source_base = u32::try_from(merged.sources.len()).unwrap_or(u32::MAX);
            let mut sub = remap_indices(sub, section.map.names.len(), source_base);
            for m in &mut sub {
                m.generated_offset = m.generated_offset.saturating_add(base);
            }
            merged.sources.extend(section.map.sources);
            merged.names.extend(section.map.names);
            merged.sources_content.extend(section.map.sources_content);
            merged.mappings.extend(sub);
        }
        merged.mappings.sort_unstable_by_key(|m| m.generated_offset);
        return Ok(merged);
    }

    if raw.mappings.is_empty() {
        return Ok(ParsedSourceMap {
            file: raw.file,
            sources: raw.sources,
            names: raw.names,
            sources_content: raw.sources_content,
            mappings: Vec::new(),
        });
    }

    let mappings = decode(&raw.mappings, raw.sources.len())?;
    Ok(ParsedSourceMap {
        file: raw.file,
        sources: raw.sources,
        names: raw.names,
        sources_content: raw.sources_content,
        mappings,
    })
}

/// Parse from an in-memory buffer (used by tests and by the WASM binding).
pub fn parse_bytes(bytes: &[u8]) -> Result<ParsedSourceMap> {
    parse_reader(bytes)
}

/// The byte offset of the first segment on `line`, in generated coordinates.
fn base_offset(mappings: &str, line: u32) -> u32 {
    let mut offset = 0u32;
    for (current_line, part) in mappings.split(';').enumerate() {
        if u32::try_from(current_line).is_ok_and(|l| l == line) {
            return offset;
        }
        for seg in part.split(',') {
            if let Some((first, _)) = seg.split_once(':') {
                let v = decode_vlq(first);
                offset = offset.saturating_add(narrow(v));
            } else {
                // 1-field segment: a generated column advance with no source
                offset = offset.saturating_add(narrow(decode_vlq(seg)));
            }
        }
        offset = offset.saturating_add(1); // the newline itself
    }
    offset
}

/// Narrow a decoded VLQ delta to the `u32` a v3 map addresses with.
///
/// The format's generated position is a (line, column) pair, so one segment
/// cannot describe more than 2^32 bytes; a delta that does comes from a corrupt
/// map. Clamping keeps the parse going with a stated number, where a wrapping
/// cast would jump backwards and mis-attribute every byte after it.
fn narrow(v: i64) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

/// Shift source indices so sections can be concatenated.
///
/// Name indices are only kept when the section actually has a name table;
/// pointing at a foreign name would be worse than having none.
fn remap_indices(mut mappings: Vec<Mapping>, name_count: usize, source_base: u32) -> Vec<Mapping> {
    for m in &mut mappings {
        m.source_index = m.source_index.saturating_add(source_base);
        if m.name_index.is_some() && name_count == 0 {
            m.name_index = None;
        }
    }
    mappings
}

/// Decode the base64 VLQ `mappings` string into a flat, offset-sorted vector.
///
/// The format packs all of a segment's fields into one colon-free run: `"AAAA"`
/// is four values (0, 0, 0, 0) and `"IACE"` is (4, 0, 1, 2). Field identity comes
/// from *position* in the run, not from a separator — getting that wrong yields
/// an empty mapping list, which is exactly what our first implementation did.
///
/// One pass, no intermediate `String`s, no per-file accumulators. Capacity is
/// pre-reserved so the decode does not reallocate mid-flight.
pub fn decode(mappings: &str, sources_len: usize) -> Result<Vec<Mapping>> {
    let mut out: Vec<Mapping> = Vec::with_capacity(mappings.len() / 8 + 8);
    let mut generated_offset = 0u32;
    let mut source_index = 0i64;
    let mut original_line = 0i64;
    let mut original_column = 0i64;
    let mut name_index = 0i64;

    let mut values = [0i64; 5];
    for line in mappings.split(';') {
        for segment in line.split(',') {
            if segment.is_empty() {
                continue;
            }
            let count = decode_segment(segment, &mut values);
            if count == 0 {
                continue;
            }

            generated_offset = generated_offset.saturating_add(narrow(values[0]));
            if count < 4 {
                // A one-field segment only advances the generated column.
                continue;
            }

            source_index += values[1];
            original_line += values[2];
            original_column += values[3];
            let name = if count >= 5 {
                name_index += values[4];
                Some(name_index)
            } else {
                None
            };

            if source_index < 0 || original_line < 0 || original_column < 0 {
                return Err(Error::Malformed(
                    "source map has negative deltas (corrupt mappings)".into(),
                ));
            }

            // A source index past the end of `sources` is a corrupt map, and the
            // interesting part is what it used to do: `attribute_by_source_in`
            // credits the bytes after the last mapping to that source, and with
            // an index out of range it silently credited nothing. The report
            // then stated a smaller total as ground truth - the exact failure
            // this tool exists to remove. Found by a property test, not by a
            // fixture.
            if source_index >= i64::try_from(sources_len).unwrap_or(i64::MAX) {
                return Err(Error::Malformed(format!(
                    "source map mapping references source {source_index} but only {sources_len} are declared"
                )));
            }

            out.push(Mapping {
                generated_offset,
                // The negative cases returned above; `narrow` clamps the rest.
                source_index: narrow(source_index),
                original_line: narrow(original_line),
                original_column: narrow(original_column),
                name_index: name.map(narrow),
            });
        }
        generated_offset = generated_offset.saturating_add(1); // the newline
    }

    Ok(out)
}

/// Decode one segment's run of VLQ values into `out`, returning how many were
/// present (1 to 5, per the spec). Stops at an invalid character rather than
/// inventing a value.
fn decode_segment(segment: &str, out: &mut [i64; 5]) -> usize {
    let mut shift = 0u32;
    let mut result = 0i64;
    let mut written = 0usize;

    for byte in segment.bytes() {
        let Some(digit) = base64_digit(byte) else { break };
        result |= i64::from(digit & 31) << shift;
        if digit & 32 == 0 {
            let negative = result & 1 == 1;
            let value = result >> 1;
            let value = if negative { -value } else { value };
            if written < out.len() {
                out[written] = value;
            }
            written += 1;
            result = 0;
            shift = 0;
        } else {
            shift += 5;
        }
    }
    written
}

/// Base64 digit value, or `None` for a character outside the VLQ alphabet.
fn base64_digit(byte: u8) -> Option<u8> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0u8;
        while (i as usize) < alphabet.len() {
            t[alphabet[i as usize] as usize] = i;
            i += 1;
        }
        t
    };
    if byte < 128 { Some(TABLE[byte as usize]) } else { None }
}

/// Decode a single VLQ value (used where the field layout is known, e.g. the
/// one-field segments inside `base_offset`).
pub fn decode_vlq(input: &str) -> i64 {
    let mut values = [0i64; 5];
    if decode_segment(input, &mut values) == 0 { 0 } else { values[0] }
}

impl ParsedSourceMap {
    /// Attribute generated bytes to sources in a single pass over the sorted
    /// mappings: each segment owns the bytes up to the next segment.
    ///
    /// Without a generated length the tail after the last mapping is
    /// unattributable, so the tail is dropped — pass the real file size
    /// (see [`Self::attribute_by_source_in`]) whenever it is known, or the last
    /// source silently loses its tail.
    pub fn attribute_by_source(&self) -> Vec<(usize, u64)> {
        self.attribute_by_source_in(0)
    }

    /// As [`Self::attribute_by_source`], but `generated_len` is the byte length
    /// of the generated file, so the bytes after the final mapping are credited
    /// to that mapping's source. A map whose last segment owns a lot of bytes
    /// (typically an inlined runtime) is the common case this fixes.
    ///
    /// # What is deliberately *not* attributed
    ///
    /// The bytes **before** the first mapping. A map that starts at column 0 has
    /// no such gap, so this only matters for maps that start mid-line; crediting
    /// the prologue to the first source would be a guess, and this function's
    /// output becomes the `attributed` dimension that a report claims as ground
    /// truth. Under-attributing by a few bytes makes coverage marginally lower,
    /// which is the safe direction: it can downgrade `attributed` to `parsed`,
    /// never inflate a size.
    ///
    /// The total therefore satisfies
    /// `sum(attribution) == generated_len - first_mapping_offset`.
    pub fn attribute_by_source_in(&self, generated_len: u64) -> Vec<(usize, u64)> {
        let mut out = vec![0u64; self.sources.len()];
        for pair in self.mappings.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let len = u64::from(b.generated_offset.saturating_sub(a.generated_offset));
            if len > 0
                && let Some(slot) = out.get_mut(a.source_index as usize)
            {
                *slot += len;
            }
        }
        if let Some(last) = self.mappings.last() {
            let tail = generated_len.saturating_sub(u64::from(last.generated_offset));
            if tail > 0
                && let Some(slot) = out.get_mut(last.source_index as usize)
            {
                *slot += tail;
            }
        }
        out.into_iter().enumerate().filter(|(_, b)| *b > 0).collect()
    }

    /// Per-source attribution keyed by normalised file path, for the fusion
    /// engine's join. `map_dir` is the directory the map lives in: source paths
    /// inside a map are relative to it, and webpack paths are absolute-ish, so
    /// the join is done on the longest common suffix (unified-graph.md §2).
    pub fn attribution_by_path(&self, map_dir: Option<&std::path::Path>) -> HashMap<String, u64> {
        self.attribution_by_path_in(map_dir, 0)
    }

    /// As above, with the generated file length so the tail is attributed.
    pub fn attribution_by_path_in(
        &self,
        map_dir: Option<&std::path::Path>,
        generated_len: u64,
    ) -> HashMap<String, u64> {
        let mut out = HashMap::new();
        for (index, bytes) in self.attribute_by_source_in(generated_len) {
            let Some(raw) = self.sources.get(index) else { continue };
            out.insert(normalise_source_path(raw, map_dir), bytes);
        }
        out
    }

    pub fn total_generated_bytes(&self) -> u64 {
        self.mappings.last().map_or(0, |m| u64::from(m.generated_offset))
    }

    /// `sourcesContent` index for a source, if the map embedded it.
    pub fn content_for(&self, index: usize) -> Option<&str> {
        self.sources_content.get(index).and_then(|c| c.as_deref())
    }
}

/// Normalise a path from a source map into something comparable with a bundler's
/// module path: forward slashes, no `file://`, no map-directory prefix, no `./`.
pub fn normalise_source_path(raw: &str, map_dir: Option<&std::path::Path>) -> String {
    let mut path = raw.replace('\\', "/");
    if let Some(rest) = path.strip_prefix("file://") {
        path = rest.to_string();
    }
    // webpack://project/./src/a.js  →  src/a.js
    if let Some(idx) = path.find("://") {
        let after_scheme = &path[idx + 3..];
        path = match after_scheme.find('/') {
            Some(slash) => after_scheme[slash + 1..].to_string(),
            None => after_scheme.to_string(),
        };
    }
    if let Some(dir) = map_dir {
        let prefix = dir.to_string_lossy().replace('\\', "/");
        if let Some(rest) = path.strip_prefix(&format!("{prefix}/")) {
            path = rest.to_string();
        }
    }
    while let Some(rest) = path.strip_prefix("./") {
        path = rest.to_string();
    }
    path
}

#[cfg(test)]
mod tests {
    use super::{ParsedSourceMap, decode, decode_vlq, normalise_source_path, parse_bytes};

    const TINY: &str = r#"{
      "version": 3,
      "file": "out.js",
      "sources": ["a.ts", "b.ts"],
      "sourcesContent": ["const a = 1;", "const b = 2;"],
      "names": [],
      "mappings": "AAAA,IACE;AACD"
    }"#;

    #[test]
    fn decodes_the_canonical_example() {
        // "AAAA,IACE;AACD" is the shape the spec uses: line 1 has two segments
        // (both from source 0, the second 4 columns later, one line down), line 2
        // continues from there.
        let map = parse_bytes(TINY.as_bytes()).unwrap();
        assert_eq!(map.sources, vec!["a.ts", "b.ts"]);
        assert_eq!(map.mappings.len(), 3);
        assert_eq!(map.mappings[0].generated_offset, 0);
        assert_eq!(map.mappings[0].source_index, 0);
        assert_eq!(map.mappings[0].original_line, 0);
        assert_eq!(map.mappings[1].generated_offset, 4);
        assert_eq!(map.mappings[1].source_index, 0, "'IACE' has a zero source delta");
        assert_eq!(map.mappings[1].original_line, 1);
        assert_eq!(map.mappings[2].generated_offset, 5, "the newline advances by one");
        assert_eq!(map.mappings[2].source_index, 0);
        assert_eq!(map.mappings[2].original_line, 2);
    }

    #[test]
    fn fields_are_positional_not_colon_separated() {
        // "AAAA,UCAA,UDAA" — fields are (genCol, srcIdx, origLine, origCol):
        //   AAAA = (+0,  +0, +0, +0)  -> gen 0,  src 0
        //   UCAA = (+10, +1, +0, +0)  -> gen 10, src 1
        //   UDAA = (+10, -1, +0, +0)  -> gen 20, src 0
        // A colon-splitting decoder gets this wrong and returns nothing; that
        // was the first implementation's bug.
        let raw = r#"{"version":3,"file":"o.js","sources":["a","b"],
                      "sourcesContent":[],"names":[],"mappings":"AAAA,UCAA,UDAA"}"#;
        let map = parse_bytes(raw.as_bytes()).unwrap();
        assert_eq!(map.mappings.len(), 3);
        assert_eq!(map.mappings[0].generated_offset, 0);
        assert_eq!(map.mappings[1].generated_offset, 10);
        assert_eq!(map.mappings[1].source_index, 1);
        assert_eq!(map.mappings[2].generated_offset, 20);
        assert_eq!(map.mappings[2].source_index, 0, "negative source deltas are legal");

        let got = map.attribute_by_source();
        assert_eq!(got, vec![(0, 10), (1, 10)], "a owns 0..10, b owns 10..20");
    }

    #[test]
    fn vlq_sign_handling() {
        assert_eq!(decode_vlq("A"), 0);
        assert_eq!(decode_vlq("C"), 1);
        assert_eq!(decode_vlq("D"), -1);
        assert_eq!(decode_vlq("gB"), 16);
    }

    #[test]
    fn attribution_uses_gaps_between_sorted_mappings() {
        let map = parse_bytes(TINY.as_bytes()).unwrap();
        let got = map.attribute_by_source();
        // "AAAA,IACE;AACD" decodes to offsets 0, 4, 5, all from source 0.
        assert_eq!(got, vec![(0, 5)], "only source 0 is referenced: {got:?}");
        assert_eq!(map.total_generated_bytes(), 5);
    }

    #[test]
    fn non_v3_maps_are_rejected_not_silently_parsed() {
        let v2 = r#"{"version":2,"sources":[],"names":[],"mappings":""}"#;
        let err = parse_bytes(v2.as_bytes()).unwrap_err();
        assert!(matches!(err, crate::Error::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn indexed_maps_are_concatenated_with_offsets() {
        let indexed = r#"{
          "version": 3,
          "file": "out.js",
          "sections": [
            {"offset": {"line": 0, "column": 0}, "map": {"version": 3, "sources": ["a.ts"], "names": [], "mappings": "AAAA"}},
            {"offset": {"line": 1, "column": 0}, "map": {"version": 3, "sources": ["b.ts"], "names": [], "mappings": "AAAA"}}
          ]
        }"#;
        let map = parse_bytes(indexed.as_bytes()).unwrap();
        assert_eq!(map.sources, vec!["a.ts", "b.ts"]);
        assert_eq!(map.mappings.len(), 2);
        assert_eq!(map.mappings[0].source_index, 0);
        assert_eq!(map.mappings[1].source_index, 1, "section sources are re-indexed");
        assert!(map.mappings[1].generated_offset > map.mappings[0].generated_offset);
    }

    #[test]
    fn large_map_decodes_without_quadratic_allocation() {
        // 200k segments: a per-segment String implementation shows up here.
        let mut mappings = String::new();
        for i in 0..200_000 {
            if i > 0 {
                mappings.push(',');
            }
            mappings.push_str("AAAA");
        }
        let raw = format!(
            r#"{{"version":3,"file":"o.js","sources":["s.js"],"sourcesContent":[],"names":[],"mappings":"{mappings}"}}"#
        );
        let t0 = std::time::Instant::now();
        let map = parse_bytes(raw.as_bytes()).unwrap();
        let dt = t0.elapsed();
        assert_eq!(map.mappings.len(), 200_000);
        assert!(dt.as_secs_f64() < 5.0, "decode took {:.2}s", dt.as_secs_f64());
    }

    #[test]
    fn path_normalisation_handles_the_shapes_bundlers_emit() {
        assert_eq!(normalise_source_path("webpack://app/./src/a.js", None), "src/a.js");
        assert_eq!(normalise_source_path("./src/b.ts", None), "src/b.ts");
        // file:// URLs keep their leading slash: that is a real absolute path.
        assert_eq!(normalise_source_path("file:///repo/src/c.ts", None), "/repo/src/c.ts");
        assert_eq!(
            normalise_source_path("/repo/src/d.ts", Some(std::path::Path::new("/repo"))),
            "src/d.ts"
        );
    }

    #[test]
    fn attribution_by_path_keys_on_normalised_names() {
        let m = ParsedSourceMap {
            sources: vec!["webpack://fixture/./src/module-0.ts".into()],
            names: vec![],
            sources_content: vec![Some("x".into())],
            mappings: vec![
                super::Mapping {
                    generated_offset: 0,
                    source_index: 0,
                    original_line: 0,
                    original_column: 0,
                    name_index: None,
                },
                super::Mapping {
                    generated_offset: 500,
                    source_index: 0,
                    original_line: 1,
                    original_column: 0,
                    name_index: None,
                },
            ],
            file: None,
        };
        let by_path = m.attribution_by_path(None);
        assert_eq!(by_path.get("src/module-0.ts"), Some(&500));
    }

    #[test]
    fn empty_mappings_are_valid() {
        let empty = r#"{"version":3,"file":"o.js","sources":[],"names":[],"mappings":""}"#;
        let map = parse_bytes(empty.as_bytes()).unwrap();
        assert!(map.mappings.is_empty());
        assert_eq!(decode("", 0).unwrap().len(), 0);
    }

    /// A mapping that points past the end of `sources` must be rejected.
    ///
    /// Found by a property test: the tail after the last mapping is credited to
    /// that mapping's source, so an out-of-range index used to make the tail
    /// vanish silently, and the report then stated a smaller total as ground
    /// truth.
    #[test]
    fn an_out_of_range_source_index_is_rejected() {
        // Two segments, both pointing at source 1, with only one source declared.
        let raw =
            br#"{"version":3,"file":"b.js","sources":["a.ts"],"names":[],"mappings":"AAAA,CCAA"}"#;
        let err = parse_bytes(raw).expect_err("source index 1 of 1 must be rejected");
        assert!(err.to_string().contains("source"), "the error should name the problem: {err}");
    }

    /// The same map with the sources declared must parse: the point is to reject
    /// the corrupt input, not to become stricter than the format.
    #[test]
    fn a_well_formed_map_still_parses() {
        let raw = br#"{"version":3,"file":"b.js","sources":["a.ts","b.ts"],"names":[],"mappings":"AAAA,CCAA"}"#;
        let map = parse_bytes(raw).expect("valid map");
        assert_eq!(map.mappings.len(), 2);
        assert_eq!(map.mappings[1].source_index, 1);
    }
}
