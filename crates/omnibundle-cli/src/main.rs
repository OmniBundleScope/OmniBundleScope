//! `omnibundle` CLI.
//!
//! The surface is deliberately aligned with the two tools OmniBundle replaces,
//! so `webpack-bundle-analyzer` / `source-map-explorer` users can swap the
//! command and keep the flags they already know. Full contract:
//! `docs/contracts/cli-surface.md`.
//!
//! Current state: the CLI surface and the exit-code contract are frozen; the
//! ingest backends arrive from WS-1/WS-3 and the fusion from WS-4. Until then
//! the binary reports what it can and never invents numbers.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "omnibundle",
    version,
    about = "Analyse bundler output (stats.json, *.map, dist folders) in one pass",
    long_about = None
)]
struct Cli {
    /// Path to analyse: a dist folder, a stats.json, or a *.map file.
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
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    /// Self-contained HTML report.
    Static,
    /// JSON report on stdout.
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    println!(
        "omnibundle {} — analysing {} (mode={:?}, sizes={:?})",
        env!("CARGO_PKG_VERSION"),
        cli.path.display(),
        cli.mode,
        cli.default_sizes
    );
    println!(
        "ingest backends are still being built; see docs/en/03-implementation-plan.md \
         (WS-1 stats, WS-3 source maps, WS-4 fusion)"
    );
    ExitCode::SUCCESS
}
