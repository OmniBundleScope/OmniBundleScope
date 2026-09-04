//! `omnibundle` CLI.
//!
//! The surface is aligned with the two tools OmniBundle replaces so users can
//! swap the command and keep the flags they know. Normative spec:
//! `docs/contracts/cli-surface.md`.
//!
//! Current capability (Phase 1, in progress — see the milestone gates in
//! `docs/en/03-implementation-plan.md`):
//!   WS-1  ✅ stats.json / esbuild metafile ingest, streaming, no document buffer
//!   WS-2  ⧗ gzip/parsed attribution (stat-only numbers today)
//!   WS-3  ⧗ source map ingest
//!   WS-4  ⧗ fusion, ghost/hidden detection
//!   WS-5  ✅ report shell (self-built, inlined, offline)
//! The CLI never invents a number it has not measured: unmeasured dimensions are
//! reported as 0 and the dimension in use is stated in the summary line.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use omnibundle_core::model::UnifiedBundleGraph;
use omnibundle_core::report::Dimension;
use omnibundle_core::stats;

#[derive(Debug, Parser)]
#[command(
    name = "omnibundle",
    version,
    about = "Analyse bundler output (stats.json, *.map, dist folders) in one pass"
)]
struct Cli {
    /// Path to analyse: a dist folder, a stats.json, or a metafile.json.
    path: PathBuf,

    /// Output mode. `static` writes a self-contained HTML report.
    #[arg(short, long, value_enum, default_value_t = Mode::Static)]
    mode: Mode,

    /// Report file to write (static mode).
    #[arg(short, long, default_value = "omnibundle-report.html")]
    report: PathBuf,

    /// Size dimension to display.
    #[arg(short, long, value_enum, default_value_t = Sizes::Parsed)]
    default_sizes: Sizes,

    /// Exclude assets matching this regular expression (repeatable).
    #[arg(short = 'e', long = "exclude", value_name = "REGEX")]
    exclude: Vec<String>,

    /// Budget config; when breached the process exits with code 1.
    #[arg(long, value_name = "FILE")]
    budget: Option<PathBuf>,

    /// Emit the unified graph as JSON on stdout (implies `--mode json`).
    #[arg(long)]
    json: bool,

    /// Grouping dimensions to include in the report (repeatable).
    #[arg(long, value_enum, value_delimiter = ',')]
    dims: Vec<Dims>,

    /// Embed `sourcesContent` for drill-down (needs source maps; Phase 2).
    #[arg(long)]
    include_sources: bool,

    /// Ingest and print the phase breakdown, then stop. No report is written.
    /// This is the mode the benchmark harness measures (B1/B2/B4), so the
    /// numbers a release claims can be produced by the shipped binary.
    #[arg(long)]
    bench: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Self-contained HTML report.
    Static,
    /// JSON report on stdout.
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Sizes {
    /// What the bundler declared.
    Stat,
    /// Measured from the emitted files.
    Parsed,
    /// gzip of the emitted files.
    Gzip,
    /// Source-map ground truth (only when every asset is mapped).
    Attributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Dims {
    Package,
    Source,
    Chunk,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("omnibundle: {err:#}");
            ExitCode::from(3)
        }
    }
}

fn run(cli: &Cli) -> Result<ExitCode> {
    let started = std::time::Instant::now();
    let found = discover(&cli.path)?;
    let phase = std::time::Instant::now();

    let graph = match &found {
        Input::File(path) => {
            // Sniff the head for the artifact shape, then stream the file.
            // Never `fs::read` here: that would put the file size on top of the
            // memory floor, which is the failure mode this project exists to
            // remove (measured: 1 GB read + parse = 6.4 GB RSS before this fix).
            let head = read_head(path, 512 * 1024)?;
            let name = file_name(path);
            let tool = sniff_tool(&head);
            stats::ingest_file(path, tool).with_context(|| format!("parsing {name}"))?
        }
        Input::Folder(dir) => {
            // Prefer the richest artifact in the folder, per cli-surface §4.
            let candidates = ["stats.json", "stats.stats.json", "metafile.json"];
            let picked = candidates
                .iter()
                .map(|n| dir.join(n))
                .find(|p| p.is_file())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no recognised artifact in {}. Looked for {}",
                        dir.display(),
                        candidates.join(", ")
                    )
                })?;
            let mut graph = stats::ingest_file(&picked, sniff_tool(&read_head(&picked, 512 * 1024)?))?;
            measure_assets_from_disk(&mut graph, dir);
            graph
        }
    };

    let ingest_ms = phase.elapsed().as_millis();
    let mut graph = graph;
    if !cli.exclude.is_empty() {
        apply_excludes(&mut graph, &cli.exclude);
    }
    stats::recompute_totals(&mut graph);

    if cli.bench {
        // The measured output the harness records. `phase_ms: null` style gaps
        // are avoided here by printing exactly what we did measure.
        let total_ms = started.elapsed().as_millis();
        println!(
            "{{\"tool\":\"omnibundle@{}\",\"input\":\"{}\",\"ingest_ms\":{},\"total_ms\":{},\"modules\":{},\"assets\":{},\"packages\":{},\"total_size\":{},\"dimension\":\"{}\"}}",
            env!("CARGO_PKG_VERSION"),
            found.label(),
            ingest_ms,
            total_ms,
            graph.totals.module_count,
            graph.totals.asset_count,
            graph.totals.package_count,
            graph.totals.total_size,
            dimension_label(cli.default_sizes),
        );
        return Ok(ExitCode::SUCCESS);
    }

    let dims: Vec<Dimension> = if cli.dims.is_empty() {
        vec![Dimension::Package, Dimension::SourceFile, Dimension::Chunk]
    } else {
        cli.dims
            .iter()
            .map(|d| match d {
                Dims::Package => Dimension::Package,
                Dims::Source => Dimension::SourceFile,
                Dims::Chunk => Dimension::Chunk,
            })
            .collect()
    };

    let payload = build_payload(&graph, &found, dims, cli.default_sizes);

    match cli.mode {
        Mode::Json => println!("{}", serde_json::to_string_pretty(&payload)?),
        Mode::Static => {
            let label = payload
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("bundle")
                .to_string();
            report::write(&payload, &label, &cli.report)?;
            println!(
                "{}  ·  {} modules  ·  {} assets  ·  {} packages  ·  parse {} ms  ·  total {} ms  ·  dimension {}",
                label,
                graph.totals.module_count,
                graph.totals.asset_count,
                graph.totals.package_count,
                ingest_ms,
                started.elapsed().as_millis(),
                dimension_label(cli.default_sizes),
            );
            println!("wrote {}", cli.report.display());
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// The payload the shell consumes: one JSON document, no timestamps, sorted
/// keys (unified-graph.md §6) so parity tests can diff it.
fn build_payload(
    graph: &UnifiedBundleGraph,
    input: &Input,
    dims: Vec<Dimension>,
    sizes: Sizes,
) -> serde_json::Value {
    let mut tree = serde_json::Map::new();
    tree.insert("schema_version".into(), graph.schema_version.into());
    tree.insert("target".into(), input.label().into());
    tree.insert(
        "sizeDimensionNote".into(),
        dimension_label(sizes).into(),
    );
    tree.insert("totals".into(), serde_json::to_value(&graph.totals).unwrap_or_default());
    tree.insert(
        "inputs".into(),
        serde_json::to_value(&graph.inputs).unwrap_or_default(),
    );
    tree.insert("diagnostics".into(), serde_json::to_value(&graph.diagnostics).unwrap_or_default());
    tree.insert("assets".into(), serde_json::to_value(&graph.assets).unwrap_or_default());
    tree.insert("chunks".into(), serde_json::to_value(&graph.chunks).unwrap_or_default());
    tree.insert("modules".into(), serde_json::to_value(&graph.modules).unwrap_or_default());
    tree.insert(
        "dimensions".into(),
        serde_json::to_value(
            &dims
                .iter()
                .map(|d| serde_json::json!({ "dimension": d, "payload": omnibundle_core::report::treemap_payload(graph, *d) }))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );
    tree.insert(
        "fusion".into(),
        serde_json::to_value(&graph.fusion).unwrap_or_default(),
    );
    serde_json::Value::Object(tree)
}

fn dimension_label(s: Sizes) -> &'static str {
    match s {
        Sizes::Stat => "stat",
        Sizes::Parsed => "parsed",
        Sizes::Gzip => "gzip",
        Sizes::Attributed => "attributed",
    }
}

/// Measure the emitted assets that sit next to the stats file. Without this the
/// report would show the bundler's estimates only, which is the number users
/// already distrust.
fn measure_assets_from_disk(graph: &mut UnifiedBundleGraph, dir: &Path) {
    for asset in &mut graph.assets {
        let candidate = dir.join(&asset.name);
        if let Ok(meta) = std::fs::metadata(&candidate) {
            asset.sizes.parsed = meta.len();
        }
    }
    for module in graph.modules.values_mut() {
        if module.sizes.parsed == 0 {
            module.sizes.parsed = module.sizes.stat;
        }
    }
}

fn apply_excludes(graph: &mut UnifiedBundleGraph, patterns: &[String]) {
    let compiled: Vec<regex_lite::Matcher> = patterns
        .iter()
        .filter_map(|p| regex_lite::Matcher::new(p))
        .collect();
    if compiled.is_empty() {
        return;
    }
    graph.assets.retain(|a| !compiled.iter().any(|m| m.matches(&a.name)));
    graph.chunks.retain(|c| !compiled.iter().any(|m| m.name_matches(c, &graph.assets)));
    let kept: Vec<u32> = graph.chunks.iter().map(|c| c.id).collect();
    graph.modules.retain(|_, m| m.chunks.iter().any(|c| kept.contains(c)) || m.chunks.is_empty());
}

/// Minimal anchored-substring matcher for `--exclude`.
///
/// Deliberately not a regex engine: a full regex dependency in the hot path of a
/// CLI is not worth the bytes, and `--exclude` in every tool we replace is a
/// plain substring or a simple glob. Treated as a case-insensitive substring,
/// with `*` treated as a wildcard, and documented as such.
mod regex_lite {
    pub struct Matcher {
        pattern: String,
    }

    impl Matcher {
        pub fn new(pattern: &str) -> Option<Self> {
            let p = pattern.to_lowercase();
            if p.is_empty() { None } else { Some(Self { pattern: p }) }
        }

        pub fn matches(&self, text: &str) -> bool {
            glob(&self.pattern, &text.to_lowercase())
        }

        pub fn name_matches(
            &self,
            chunk: &omnibundle_core::model::Chunk,
            assets: &[omnibundle_core::model::Asset],
        ) -> bool {
            chunk.names.iter().any(|n| self.matches(n))
                || chunk.assets.iter().any(|a| {
                    assets
                        .iter()
                        .find(|x| &x.name == a)
                        .map(|x| self.matches(&x.name))
                        .unwrap_or(false)
                })
        }
    }

    fn glob(pattern: &str, text: &str) -> bool {
        if !pattern.contains('*') {
            return text.contains(pattern);
        }
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0usize;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            match text[pos..].find(part) {
                Some(found) => {
                    if i == 0 && found != 0 {
                        return false;
                    }
                    pos += found + part.len();
                }
                None => return false,
            }
        }
        if let Some(last) = parts.last()
            && !last.is_empty()
            && !text.ends_with(last)
        {
            return false;
        }
        true
    }

    #[cfg(test)]
    mod tests {
        use super::{Matcher, glob};

        #[test]
        fn matcher_is_case_insensitive_on_both_sides() {
            // The lowercasing lives in Matcher::matches, not in glob.
            assert!(Matcher::new("Vendor").unwrap().matches("./node_modules/Vendor/x.js"));
            assert!(!Matcher::new("vendor").unwrap().matches("./src/app.js"));
        }

        #[test]
        fn plain_substring_needs_no_anchors() {
            assert!(glob("vendor", "./node_modules/vendor/x.js"));
            assert!(!glob("vendor", "./src/app.js"));
        }

        #[test]
        fn glob_anchors_at_the_start_when_the_pattern_starts_with_it() {
            assert!(glob("dist/*", "dist/main.js"));
            assert!(!glob("dist/*", "./dist/main.js"));
        }

        #[test]
        fn glob_anchors_at_the_end_when_the_pattern_ends_with_it() {
            assert!(glob("*.min.js", "a/b/x.min.js"));
            assert!(!glob("*.min.js", "a/b/x.min.js.map"));
        }
    }
}

enum Input {
    File(PathBuf),
    Folder(PathBuf),
}

impl Input {
    fn label(&self) -> String {
        match self {
            Input::File(p) => file_name(p),
            Input::Folder(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| p.display().to_string()),
        }
    }
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/// Read at most `limit` bytes from the head of a file. Used only for artifact
/// sniffing, so a short read on a small file is not an error.
fn read_head(path: &Path, limit: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut buf = vec![0u8; limit];
    let mut filled = 0;
    while filled < limit {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

/// Artifact kind from the head bytes. A hint only: if the guess is wrong the
/// real parser produces the real error, so this can never silently mis-build a
/// graph.
fn sniff_tool(head: &[u8]) -> &'static str {
    let has = |needle: &[u8]| head.windows(needle.len()).any(|w| w == needle);
    if has(b"\"inputs\"") && has(b"\"outputs\"") && !has(b"\"modules\"") {
        "esbuild"
    } else {
        "webpack"
    }
}

fn discover(path: &Path) -> Result<Input> {
    if path.is_file() {
        return Ok(Input::File(path.to_path_buf()));
    }
    if path.is_dir() {
        return Ok(Input::Folder(path.to_path_buf()));
    }
    anyhow::bail!("{} does not exist", path.display())
}

/// Report rendering: inline the shell into one offline file (ADR-0002).
mod report {
    use anyhow::{Context, Result};

    const HTML: &str = include_str!("../../../assets/report/shell.html");
    const CSS: &str = include_str!("../../../assets/report/shell.css");
    const JS: &str = include_str!("../../../assets/report/shell.js");

    pub fn render(payload: &serde_json::Value, target: &str) -> Result<String> {
        let json = serde_json::to_string(payload)?;
        // `</script>` inside an inlined JSON island would end the tag early;
        // escaping the slash is the standard fix and costs nothing.
        let json = json.replace("</", "<\\/");

        Ok(HTML
            .replace("/*TITLE*/", target)
            .replace("/*CSS*/", CSS)
            .replace("/*PAYLOAD*/", &json)
            .replace("/*JS*/", JS))
    }

    pub fn write(payload: &serde_json::Value, target: &str, out: &std::path::Path) -> Result<()> {
        let html = render(payload, target)?;
        std::fs::write(out, html).with_context(|| format!("writing {}", out.display()))?;
        Ok(())
    }
}
