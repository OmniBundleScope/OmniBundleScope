//! `omnibundle` CLI.
//!
//! The surface is aligned with the two tools `OmniBundle` replaces so users can
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
use omnibundle_core::sizes;
use omnibundle_core::stats;

#[derive(Debug, Parser)]
#[command(
    name = "omnibundle",
    version,
    about = "Analyse bundler output (stats.json, *.map, dist folders) in one pass"
)]
/// Clap flags are booleans by nature. The lint is about readability at a call
/// site, and every one of these is a named `--flag` in `cli-surface.md`.
#[allow(clippy::struct_excessive_bools)]
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

    /// Ingest a source map, attribute its bytes, print the totals, stop.
    /// Measured by the harness for B5, against the same fixtures as the
    /// `source-map-explorer` baseline.
    #[arg(long)]
    bench_map: bool,
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

// Display arithmetic (ytes as f64 / 1_048_576.0) is the only precision loss
// allowed here, and it is allowed for the reason given in omnibundle-core.
#[allow(clippy::cast_precision_loss)]
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

/// The one pipeline, in the order the phases depend on: discover, ingest,
/// measure, fuse, gate, report.
///
/// It is long because splitting it would hide the ordering that makes the
/// numbers correct (measure before fuse, fuse before the budget, budget before
/// the report so a breach is in the report too).
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn run(cli: &Cli) -> Result<ExitCode> {
    if cli.bench_map {
        return bench_source_map(&cli.path);
    }

    let started = std::time::Instant::now();
    let found = discover(&cli.path)?;
    let phase = std::time::Instant::now();
    let mut fusion: Option<(omnibundle_core::fusion::FusionOutcome, usize)> = None;

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
            let picked = candidates.iter().map(|n| dir.join(n)).find(|p| p.is_file());

            let from_metadata = picked.is_some();
            let mut graph = match picked {
                Some(path) => {
                    stats::ingest_file(&path, sniff_tool(&read_head(&path, 512 * 1024)?))?
                }
                // No bundler metadata: this is what vite, rollup, parcel and tsup
                // hand you by default. The output and its source maps are enough
                // for measured sizes and per-source attribution; ghost code needs
                // a declared graph and is reported as undetectable rather than as
                // a clean bill of health.
                None => omnibundle_core::folder::ingest(dir).with_context(|| {
                    format!(
                        "no stats.json or metafile.json in {}, and it holds no build output either",
                        dir.display()
                    )
                })?,
            };
            sizes::attribute_from_disk(&mut graph, dir)?;

            // Fusion: any *.map next to the assets joins the graph (PRD 3.2).
            let maps = omnibundle_core::fusion::maps_in_dir(dir);
            if !maps.is_empty() {
                let outcome = if from_metadata {
                    omnibundle_core::fusion::analyse(&mut graph, &maps)
                } else {
                    omnibundle_core::fusion::analyse_sources_only(&mut graph, &maps)
                };
                fusion = Some((outcome, maps.len()));
            }
            graph
        }
    };

    let ingest_ms = phase.elapsed().as_millis();
    let mut graph = graph;
    if !cli.exclude.is_empty() {
        apply_excludes(&mut graph, &cli.exclude);
    }
    stats::recompute_totals(&mut graph);

    // Budget gate (cli-surface §3): a breach exits 1 so CI fails, and a rule that
    // matches nothing is an error rather than a silent pass.
    let mut budget_failed = false;
    let mut budget_config_error = false;
    if let Some(config_path) = &cli.budget {
        // Through the BOM-skipping reader: Windows editors add byte order marks
        // to config files, and serde_json rejects them, which would make budget
        // config the most fragile input the tool has.
        let mut text = String::new();
        {
            use std::io::Read as _;
            let mut reader = omnibundle_core::bom::BomSkip::new(
                std::fs::File::open(config_path)
                    .with_context(|| format!("opening {}", config_path.display()))?,
            );
            reader
                .read_to_string(&mut text)
                .with_context(|| format!("reading {}", config_path.display()))?;
        }
        let config: BudgetConfig = serde_json::from_str(&text)
            .with_context(|| format!("parsing {}", config_path.display()))?;
        let (errors, ok, breaches) = evaluate_budget(&graph, &config);
        if !errors.is_empty() {
            budget_config_error = true;
        }
        for e in errors {
            eprintln!("omnibundle: {e}");
        }
        for d in &breaches {
            eprintln!("omnibundle: {} {}", d.code, d.message);
            graph.diagnostics.push(d.clone());
        }
        budget_failed = !ok;
    }

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

    // `--mode json` is a machine contract and carries every module; the HTML
    // report is a picture and does not.
    let payload = build_payload(&graph, &found, &dims, cli.default_sizes, cli.mode == Mode::Json);

    match cli.mode {
        Mode::Json => println!("{}", serde_json::to_string_pretty(&payload)?),
        Mode::Static => {
            let label =
                payload.get("target").and_then(|v| v.as_str()).unwrap_or("bundle").to_string();

            // The contract says a report must state the dimension it *used*, not
            // the one that was requested: a partial source map downgrades
            // `attributed` to `parsed`, and claiming otherwise is exactly the
            // kind of lie this tool exists to remove. Decided *before* writing,
            // because the old code wrote the report and then rewrote it.
            let used = match graph.totals.size_dimension {
                omnibundle_core::model::SizeDimension::Stat => "stat",
                omnibundle_core::model::SizeDimension::Parsed => "parsed",
                omnibundle_core::model::SizeDimension::Gzip => "gzip",
                omnibundle_core::model::SizeDimension::Attributed => "attributed",
            };
            let requested = dimension_label(cli.default_sizes);
            let mut payload = payload;
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("sizeDimension".into(), used.into());
            }

            let detail_bytes = report::write_detail_file(&graph, &cli.report)?;
            report::write(&payload, &label, &cli.report, detail_bytes)?;
            if used != requested {
                // Say *why*, and say it in the right direction: `attributed` is an
                // upgrade when the maps cover the build, and a downgrade when they
                // do not. "Not available" is neither.
                let reason = if used == "attributed" {
                    "source maps cover the build, so this is ground truth"
                } else {
                    "that dimension is not measurable for this input"
                };
                println!("showing `{used}` instead of `{requested}`: {reason} (OB0050)");
            }
            let report_bytes = std::fs::metadata(&cli.report).map_or(0, |m| m.len());
            println!(
                "{}  ·  {} modules  ·  {} assets  ·  {} packages  ·  ingest {} ms  ·  total {} ms  ·  dimension {}",
                label,
                graph.totals.module_count,
                graph.totals.asset_count,
                graph.totals.package_count,
                ingest_ms,
                started.elapsed().as_millis(),
                used,
            );
            if let Some((outcome, maps)) = fusion {
                // Ghost code is defined against a declared module graph. Without
                // one, "0 ghost" would be an absence of evidence dressed as a
                // clean bill of health, so it says what it can and cannot see.
                let ghost = if outcome.ghosts_detectable {
                    format!(
                        "{} ghost ({} of declared)",
                        outcome.ghost_count,
                        human_bytes(outcome.ghost_bytes)
                    )
                } else {
                    "ghost code needs a stats.json to detect".to_string()
                };
                println!(
                    "fusion: {maps} map(s) · coverage {:.0}% · {}/{} modules attributed · {ghost} · {} hidden source(s) ({})",
                    outcome.coverage * 100.0,
                    outcome.attributed_modules,
                    graph.totals.module_count,
                    outcome.hidden_count,
                    human_bytes(outcome.hidden_bytes),
                );
            }
            // Display math on a file size; the byte count itself is exact.
            let report_mb = report_bytes as f64 / 1_048_576.0;
            println!(
                "wrote {} ({report_mb:.1} MB){}",
                cli.report.display(),
                if detail_bytes <= report::INLINE_LIMIT as u64 {
                    ", detail inlined"
                } else {
                    ", detail in a companion script (loaded on demand)"
                }
            );
        }
    }

    // A budget breach is exit 1 (the CI gate). A budget *config* problem is
    // reported on stderr and the analysis still runs, so a typo in a rule does
    // not hide the rest of the report — but it never counts as a pass.
    if budget_config_error {
        return Ok(ExitCode::from(3));
    }
    Ok(if budget_failed { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

/// The payload the shell consumes.
///
/// Two pieces on purpose (`docs/en/04-benchmark-plan.md` §6):
/// - `trees`: one two-level treemap per dimension, sizes only. This is what
///   makes the HTML small enough to open; the previous "dump every module"
///   shape produced a 125.6 MB file and ~24 s of generation for 154k modules.
/// - `detail`: per-module facts (reasons, sources, per-dimension sizes), emitted
///   as a **companion script**, because a 125 MB inline island is what we are
///   avoiding and because `fetch()` cannot load a sibling file from `file://`
///   in a browser. A `<script src>` can, so the detail data is a JS assignment.
///
/// Both are deterministic (sorted keys, no timestamps) so parity tests can diff
/// them.
fn build_payload(
    graph: &UnifiedBundleGraph,
    input: &Input,
    dims: &[Dimension],
    sizes: Sizes,
    include_modules: bool,
) -> serde_json::Value {
    let mut tree = serde_json::Map::new();
    tree.insert("schema_version".into(), graph.schema_version.into());
    tree.insert("target".into(), input.label().into());
    tree.insert("sizeDimension".into(), dimension_label(sizes).into());
    tree.insert("totals".into(), serde_json::to_value(&graph.totals).unwrap_or_default());
    tree.insert("inputs".into(), serde_json::to_value(&graph.inputs).unwrap_or_default());
    tree.insert("diagnostics".into(), serde_json::to_value(&graph.diagnostics).unwrap_or_default());
    tree.insert("assets".into(), serde_json::to_value(&graph.assets).unwrap_or_default());
    tree.insert("chunks".into(), serde_json::to_value(&graph.chunks).unwrap_or_default());
    // The full module table is what `--mode json` consumers (parity tests, CI,
    // BI) need, and the detail payload carries it for the report's drill-down.
    // The HTML does *not* get it: including it here put 154,379 modules into
    // every report and took it from 1.6 MB to 55.3 MB, for data the shell loads
    // from the companion script on demand anyway.
    if include_modules {
        tree.insert("modules".into(), serde_json::to_value(&graph.modules).unwrap_or_default());
    }
    tree.insert(
        "trees".into(),
        serde_json::to_value(
            dims.iter()
                .map(|d| {
                    serde_json::json!({
                        "dimension": d.as_str(),
                        "tree": omnibundle_core::report::treemap_tree(graph, *d),
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default(),
    );
    tree.insert("fusion".into(), serde_json::to_value(&graph.fusion).unwrap_or_default());
    serde_json::Value::Object(tree)
}

#[allow(clippy::cast_precision_loss)]
fn human_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let mb = n as f64 / (1024.0 * 1024.0);
    if mb >= 1.0 { format!("{mb:.1} MB") } else { format!("{:.0} KB", n as f64 / KB) }
}

/// A budget rule from `omnibundle.config.json` (cli-surface §3).
#[derive(Debug, serde::Deserialize)]
struct BudgetConfig {
    #[serde(default)]
    limits: Vec<BudgetLimit>,
}

#[derive(Debug, serde::Deserialize)]
struct BudgetLimit {
    /// `total`, `chunk` or `package`
    scope: String,
    /// Which chunk/package the rule applies to; `None` for `total`.
    #[serde(default)]
    r#match: Option<String>,
    /// Bytes, on the dimension the report is showing.
    max: u64,
    /// Optional per-limit dimension: `stat`, `parsed`, `gzip` or `attributed`.
    #[serde(default)]
    dimension: Option<String>,
}

/// Read one dimension off a `SizeSet` by name.
///
/// A budget that silently ignored its `dimension` field would compare a `gzip`
/// limit against parsed bytes, which passes or fails for reasons that have
/// nothing to do with the rule the author wrote. An unknown name is an error,
/// not a fallback.
fn size_for(
    sizes: &omnibundle_core::model::SizeSet,
    dimension: Option<&str>,
) -> Result<u64, String> {
    match dimension {
        None => Ok(sizes.effective()),
        Some("stat") => Ok(sizes.stat),
        Some("parsed") => Ok(sizes.parsed),
        Some("gzip") => Ok(sizes.gzip),
        Some("attributed") => sizes.attributed.ok_or_else(|| {
            "dimension `attributed` is not available: no source map covered this build".into()
        }),
        Some(other) => {
            Err(format!("unknown dimension `{other}` (expected stat, parsed, gzip or attributed)"))
        }
    }
}

fn dimension_name(dimension: Option<&str>) -> String {
    dimension.unwrap_or("(report default)").to_string()
}

/// Evaluate the budget. Returns the diagnostics and whether the build passed.
///
/// Two rules that are easy to get wrong and are therefore explicit:
/// - a `match` that matches nothing is an **error**, not a no-op: a typo in a
///   budget rule must not silently pass CI;
/// - a breach is reported against the dimension actually used, and the summary
///   line says which one, so a limit is never compared against a different
///   measurement than the one a human sees.
///
/// The three scopes differ only in what they sum, so they share one pass; the
/// length comes from that, not from a chain of helpers that would each need the
/// graph and the config anyway.
#[allow(clippy::too_many_lines)]
fn evaluate_budget(
    graph: &UnifiedBundleGraph,
    config: &BudgetConfig,
) -> (Vec<anyhow::Error>, bool, Vec<omnibundle_core::model::Diagnostic>) {
    let mut errors = Vec::new();
    let mut breached = 0usize;
    let mut diagnostics = Vec::new();

    for limit in &config.limits {
        let dimension = limit.dimension.as_deref();
        let (scope, value) = match limit.scope.as_str() {
            "total" => {
                // The graph total is only meaningful in whatever dimension the
                // graph settled on, so a limit that names a different dimension
                // sums the assets itself rather than reusing `totals`.
                let value = if dimension.is_none() {
                    graph.totals.total_size
                } else {
                    let mut sum = 0u64;
                    let mut usable = true;
                    for asset in &graph.assets {
                        match size_for(&asset.sizes, dimension) {
                            Ok(v) => sum = sum.saturating_add(v),
                            Err(e) => {
                                errors.push(anyhow::anyhow!("budget: {e}"));
                                usable = false;
                                break;
                            }
                        }
                    }
                    if usable { sum } else { continue }
                };
                (limit.scope.clone(), value)
            }
            "chunk" => {
                let Some(needle) = limit.r#match.as_deref() else {
                    errors.push(anyhow::anyhow!("budget: `chunk` scope needs a `match`"));
                    continue;
                };
                let mut sum = 0u64;
                let mut matched = 0usize;
                for chunk in &graph.chunks {
                    if chunk.names.iter().any(|n| n == needle) {
                        match size_for(&chunk.size, dimension) {
                            Ok(v) => sum = sum.saturating_add(v),
                            Err(e) => {
                                errors.push(anyhow::anyhow!("budget: {e}"));
                                continue;
                            }
                        }
                        matched += 1;
                    }
                }
                if matched == 0 {
                    errors.push(anyhow::anyhow!(
                        "budget: no chunk matches `{needle}`; a rule that matches nothing is an error"
                    ));
                    continue;
                }
                (limit.scope.clone(), sum)
            }
            "package" => {
                let Some(needle) = limit.r#match.as_deref() else {
                    errors.push(anyhow::anyhow!("budget: `package` scope needs a `match`"));
                    continue;
                };
                let mut sum = 0u64;
                let mut matched = 0usize;
                for module in graph.modules.values() {
                    if module.package.as_ref().is_some_and(|p| {
                        p.name == needle || p.name.starts_with(&format!("{needle}/"))
                    }) {
                        match size_for(&module.sizes, dimension) {
                            Ok(v) => sum = sum.saturating_add(v),
                            Err(e) => {
                                errors.push(anyhow::anyhow!("budget: {e}"));
                                continue;
                            }
                        }
                        matched += 1;
                    }
                }
                if matched == 0 {
                    errors.push(anyhow::anyhow!(
                        "budget: no package matches `{needle}`; a rule that matches nothing is an error"
                    ));
                    continue;
                }
                (limit.scope.clone(), sum)
            }
            other => {
                errors.push(anyhow::anyhow!(
                    "budget: unknown scope `{other}` (expected total, chunk or package)"
                ));
                continue;
            }
        };

        if value > limit.max {
            breached += 1;
            diagnostics.push(omnibundle_core::model::Diagnostic {
                severity: omnibundle_core::model::Severity::Error,
                code: "OB0040".into(),
                message: format!(
                    "budget {scope}{} on {} is {} over its {} limit",
                    limit.r#match.as_ref().map(|m| format!(" `{m}`")).unwrap_or_default(),
                    dimension_name(dimension),
                    human_bytes(value - limit.max),
                    human_bytes(limit.max)
                ),
                subject: None,
                data: serde_json::json!({
                    "scope": scope,
                    "dimension": dimension.unwrap_or("report default"),
                    "actual": value,
                    "max": limit.max
                }),
            });
        }
    }

    (errors, breached == 0, diagnostics)
}

fn dimension_label(s: Sizes) -> &'static str {
    match s {
        Sizes::Stat => "stat",
        Sizes::Parsed => "parsed",
        Sizes::Gzip => "gzip",
        Sizes::Attributed => "attributed",
    }
}

fn apply_excludes(graph: &mut UnifiedBundleGraph, patterns: &[String]) {
    let compiled: Vec<regex_lite::Matcher> =
        patterns.iter().filter_map(|p| regex_lite::Matcher::new(p)).collect();
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
                || chunk
                    .assets
                    .iter()
                    .any(|a| assets.iter().any(|x| &x.name == a && self.matches(&x.name)))
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

/// B5: parse a source map, attribute its bytes, print the numbers.
///
/// The map is streamed from disk, exactly like the stats path, because the
/// WS-S baseline showed this workload reaching 642 MB — a `fs::read` here would
/// put the file size straight back on the floor.
fn bench_source_map(path: &Path) -> Result<ExitCode> {
    let started = std::time::Instant::now();
    let dir = path.parent();
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;

    let parse_started = std::time::Instant::now();
    let map = omnibundle_core::sourcemap::parse_reader(file)
        .with_context(|| format!("parsing {}", path.display()))?;
    let parse_ms = parse_started.elapsed().as_millis();

    let attr_started = std::time::Instant::now();
    let by_path = map.attribution_by_path(dir);
    let attributed_total: u64 = by_path.values().sum();
    let attribute_ms = attr_started.elapsed().as_millis();

    println!(
        "{{\"tool\":\"omnibundle@{}\",\"input\":\"{}\",\"parse_ms\":{},\"attribute_ms\":{},\"total_ms\":{},\"sources\":{},\"mappings\":{},\"attributed_files\":{},\"attributed_total\":{}}}",
        env!("CARGO_PKG_VERSION"),
        file_name(path),
        parse_ms,
        attribute_ms,
        started.elapsed().as_millis(),
        map.sources.len(),
        map.mappings.len(),
        by_path.len(),
        attributed_total,
    );
    Ok(ExitCode::SUCCESS)
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
                .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().to_string()),
        }
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().to_string())
}

/// Read at most `limit` bytes from the head of a file. Used only for artifact
/// sniffing, so a short read on a small file is not an error.
fn read_head(path: &Path, limit: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
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

    /// Below this size the per-module detail is inlined and the report stays a
    /// single file. Above it, the detail moves to `<report>.data.js` and the
    /// HTML loads it with a `<script>` tag — which works from `file://`, unlike
    /// `fetch`. Measured: 154,379 modules is ~121 MB of detail, which is a file
    /// nobody opens.
    pub const INLINE_LIMIT: usize = 2 * 1024 * 1024;

    pub fn render(
        payload: &serde_json::Value,
        target: &str,
        detail: &[u8],
        inline_detail: bool,
    ) -> Result<String> {
        let json = escape_for_script_tag(&serde_json::to_string(payload)?);

        let detail_block = if inline_detail {
            format!(
                "<script id=\"detail\" type=\"application/json\">{}</script>",
                escape_for_script_tag(std::str::from_utf8(detail).unwrap_or("{}"))
            )
        } else {
            // `write` fills in the real file name. It used to leave the literal
            // placeholder in the HTML, so every report above the inline limit
            // shipped a detail file that nothing ever loaded.
            format!("<script src=\"{DATA_SCRIPT_PLACEHOLDER}\"></script>")
        };

        Ok(HTML
            .replace("/*TITLE*/", target)
            .replace("/*CSS*/", CSS)
            .replace("/*PAYLOAD*/", &json)
            .replace("<!--DETAIL-->", &detail_block)
            .replace("/*JS*/", JS))
    }

    pub const DATA_SCRIPT_PLACEHOLDER: &str = "REPLACED_BY_DATA_SCRIPT";

    /// Write the detail to `<report>.data.js`, then the HTML that loads it.
    ///
    /// The detail is streamed to disk by the caller, so nothing here holds the
    /// 36 MB payload in memory: only reports small enough to inline (<=2 MB) are
    /// read back, and then the companion file is removed so a single-file report
    /// really is a single file.
    /// Stream `<report>.data.js`: a JS assignment (not raw JSON) because the
    /// shell reads `window.__OB_DETAIL__`, and because a `<script src>` on
    /// `file://` is the only way a report can pull in sibling data at all.
    ///
    /// Returns the byte size so the caller can decide inline vs companion
    /// without ever holding the payload in memory.
    pub fn write_detail_file(
        graph: &omnibundle_core::model::UnifiedBundleGraph,
        out: &std::path::Path,
    ) -> Result<u64> {
        use std::io::Write;
        let data_path = data_path(out);
        let file = std::fs::File::create(&data_path)
            .with_context(|| format!("creating {}", data_path.display()))?;
        let mut w = std::io::BufWriter::with_capacity(1 << 20, file);
        w.write_all(b"window.__OB_DETAIL__=")?;
        omnibundle_core::report::write_detail(graph, &mut w)
            .with_context(|| format!("writing {}", data_path.display()))?;
        w.write_all(b";\n")?;
        w.flush()?;
        Ok(std::fs::metadata(&data_path)?.len())
    }

    pub fn write(
        payload: &serde_json::Value,
        target: &str,
        out: &std::path::Path,
        detail_bytes: u64,
    ) -> Result<()> {
        let data_path = data_path(out);
        let inline_detail = detail_bytes <= INLINE_LIMIT as u64;
        let html = if inline_detail {
            let detail = std::fs::read(&data_path)
                .with_context(|| format!("reading back {}", data_path.display()))?;
            render(payload, target, &detail, true)?
        } else {
            let file_name = data_path
                .file_name()
                .map_or_else(|| "report.data.js".to_string(), |n| n.to_string_lossy().to_string());
            // Only the file name, never a path: the report and its data live side
            // by side, and an absolute path here would break the report the moment
            // the directory is moved or opened from a different root.
            let mut html = render(payload, target, &[], false)?;
            html = html.replace(DATA_SCRIPT_PLACEHOLDER, &file_name);
            html
        };

        std::fs::write(out, html).with_context(|| format!("writing {}", out.display()))?;
        if inline_detail {
            let _ = std::fs::remove_file(&data_path);
        }
        Ok(())
    }

    pub fn data_path(report: &std::path::Path) -> std::path::PathBuf {
        let mut name = report.file_name().unwrap_or_default().to_os_string();
        name.push(".data.js");
        report.with_file_name(name)
    }

    /// `</script>` inside an inlined JSON island would end the tag early;
    /// escaping the slash is the standard fix and costs nothing.
    fn escape_for_script_tag(s: &str) -> String {
        s.replace("</", "<\\/")
    }
}
