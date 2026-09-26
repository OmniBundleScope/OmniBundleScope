//! End-to-end tests for the two input shapes, through the shipped binary.
//!
//! The folder-only path is the one that used to not exist: a `dist/` with assets
//! and source maps but no `stats.json` was a hard error, which meant the tool
//! could not read the output of vite, rollup, parcel or tsup without configuring
//! them to emit bundler-specific metadata. It is also the path where it is
//! easiest to lie — reporting "0 ghost code" when there is no declared graph to
//! detect ghosts against — so that string is asserted here rather than left to a
//! manual look.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> PathBuf {
    // `target/debug` when run by `cargo test`, `target/release` for a release run.
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop(); // deps
    if path.ends_with("deps") {
        path.pop();
    }
    let exe = if cfg!(windows) { "omnibundle.exe" } else { "omnibundle" };
    let candidate = path.join(exe);
    assert!(candidate.exists(), "the CLI binary is missing at {}", candidate.display());
    candidate
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("omnibundle-cli-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Encode one signed VLQ delta, as the source map v3 format defines it.
///
/// Written out rather than pasted as literal base64 because a hand-typed
/// `"GAAMA"` is wrong in a way that still parses: it decodes to one segment with
/// a name index instead of two segments, and the test then fails for a reason
/// that has nothing to do with the CLI.
fn vlq(value: i64) -> String {
    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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

/// A source map with two sources: the first owns the first half of the generated
/// file, the second owns everything after it. Two segments, four fields each —
/// generated column, source index, original line, original column.
fn two_source_map(split_at: u32) -> String {
    let first = format!("{}{}{}{}", vlq(0), vlq(0), vlq(0), vlq(0));
    let second = format!("{}{}{}{}", vlq(i64::from(split_at)), vlq(1), vlq(0), vlq(0));
    format!(
        r#"{{"version":3,"file":"index.js","sources":["webpack:///src/first.js","webpack:///src/second.js"],"sourcesContent":[],"names":[],"mappings":"{first},{second}"}}"#
    )
}
fn run(dir: &Path) -> (String, String, i32) {
    let out = Command::new(binary())
        .arg(dir)
        .args(["--mode", "static", "--report"])
        .arg(dir.join("report.html"))
        .output()
        .expect("the CLI runs");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn a_vite_dist_folder_without_bundler_metadata_is_analysed() {
    let dir = temp_dir("no-metadata");
    // Vite's actual layout: hashed files, and the source maps, under
    // dist/assets/. A flat fixture passes while the tool finds nothing, because
    // that is the only place vite puts them.
    fs::create_dir_all(dir.join("assets")).expect("create assets dir");
    fs::write(dir.join("index.html"), b"<!doctype html>").expect("write html");
    let map = two_source_map(2_048);
    fs::write(dir.join("assets/index-DiwrgTda.js"), vec![b'x'; 4_096]).expect("write bundle");
    fs::write(dir.join("assets/index-DiwrgTda.js.map"), map.as_bytes()).expect("write map");

    let (stdout, stderr, code) = run(&dir);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stdout.contains("2 assets"), "the nested bundle and the html both count: {stdout}");
    assert!(stdout.contains("modules attributed"), "no fusion line in: {stdout}");
    assert!(
        stdout.contains("2/2 modules attributed"),
        "both sources should be attributed from the map alone: {stdout}"
    );
    assert!(dir.join("report.html").exists(), "no report was written");
}

#[test]
fn no_declared_graph_means_no_ghost_claim() {
    let dir = temp_dir("ghost-honesty");
    let map = two_source_map(2_048);
    fs::write(dir.join("index.js"), vec![b'x'; 4_096]).expect("write bundle");
    fs::write(dir.join("index.js.map"), map.as_bytes()).expect("write map");

    let (stdout, _, _) = run(&dir);
    assert!(
        stdout.contains("ghost code needs a stats.json to detect"),
        "a folder with no declared graph must not report a ghost count: {stdout}"
    );
    assert!(
        !stdout.contains("0 ghost"),
        "'0 ghost' would be an absence of evidence dressed as a clean bill of health: {stdout}"
    );
}

#[test]
fn a_map_that_cannot_be_read_is_said_out_loud() {
    // A truncated map. Before, it was dropped without a word: the report showed
    // 0 modules and no explanation, which reads as "this build shipped no source
    // maps" rather than "one map is broken".
    let dir = temp_dir("unreadable-map");
    fs::write(dir.join("index.js"), vec![b'x'; 4_096]).expect("write bundle");
    fs::write(dir.join("index.js.map"), br#"{"version":3,"sources":["a.ts"],"mapp"#)
        .expect("write truncated map");

    let (stdout, _, code) = run(&dir);
    assert_eq!(code, 0, "one broken map is not a broken build");
    assert!(
        stdout.contains("could not read") && stdout.contains("index.js.map"),
        "an unreadable map must be named: {stdout}"
    );
}

#[test]
fn a_folder_with_no_build_output_at_all_is_an_error() {
    let dir = temp_dir("empty-folder");
    let (_, stderr, code) = run(&dir);
    assert_ne!(code, 0, "an empty folder must not produce a clean report");
    assert!(
        stderr.contains("no build output") || stderr.contains("no stats.json"),
        "the error should say what it looked for: {stderr}"
    );
}

#[test]
fn the_stats_path_still_reports_ghosts_normally() {
    // The same folder *with* a stats file must go back to reporting ghosts,
    // otherwise the fix would have quietly disabled the feature everywhere.
    let dir = temp_dir("with-metadata");
    let map = two_source_map(2_048);
    fs::write(dir.join("index.js"), vec![b'x'; 4_096]).expect("write bundle");
    fs::write(dir.join("index.js.map"), map.as_bytes()).expect("write map");
    fs::write(
        dir.join("stats.json"),
        r#"{"version":"5.90.0",
            "assets":[{"type":"asset","name":"index.js","size":4096,"chunks":[0],"emitted":true}],
            "chunks":[{"id":0,"names":["main"],"files":["index.js"],"size":4096}],
            "modules":[{"id":1,"identifier":"./src/first.js","name":"./src/first.js","size":2048,"chunks":[0],"reasons":[]},
                       {"id":2,"identifier":"./src/ghost.js","name":"./src/ghost.js","size":2048,"chunks":[0],"reasons":[]}]}"#,
    )
    .expect("write stats");

    let (stdout, _, code) = run(&dir);
    assert_eq!(code, 0, "the stats path should still work");
    assert!(
        stdout.contains("ghost"),
        "with a declared graph, ghosts must be reported again: {stdout}"
    );
    assert!(
        !stdout.contains("needs a stats.json"),
        "the stats path has a graph and must not claim it cannot detect ghosts: {stdout}"
    );
}
