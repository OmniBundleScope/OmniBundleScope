# OmniBundle — Product Requirements (V1.0 + verification annotations)

Owner: WS-0 · Status: product scope frozen for Phase 1 · Translations:
[ZH](../zh/00-prd.md) · [JA](../ja/00-prd.md) · [DE](../de/00-prd.md)

> The first section is the original PRD, kept verbatim. The second section
> annotates every measurable claim with what we have actually verified. That
> split is deliberate: the vision is the owner's, the numbers are ours, and
> conflating them is how projects end up promising things nobody measured.

---

## Part 1 — The PRD (verbatim)

**Document version:** V1.0
**Product:** OmniBundle
**Positioning:** A next-generation, unified analysis and visualisation platform
for front-end bundle output, built on Rust.

### 1. Background and pain points

**1.1 Industry background.** The front-end build ecosystem is moving from
webpack dominance to a plurality: Vite, Rspack, esbuild, Turbopack. Each
produces different output and different metadata formats.

**1.2 Core pain points.**

*Tool fragmentation ("island effect").* Analysing webpack requires
`webpack-bundle-analyzer` (WBA). Analysing a generic `.map` requires
`source-map-explorer` (SME). Analysing Vite/Rollup output usually means
`rollup-plugin-visualizer`. A monorepo or multi-stack team has to learn and
configure several tools and cannot establish one size baseline.

*Depth versus breadth.* WBA has depth (module graph, tree-shaking visibility)
but only webpack breadth. SME has breadth (every `.map`) but shallow analysis
(file-level aggregation, no dependency graph).

*Performance.* Large `stats.json` files run to 200-500 MB. Node-based tools
frequently OOM while parsing, or freeze the browser when rendering.

### 2. Vision and value

*One-line vision:* "One Tool to Rule Them All" — a bundler-agnostic,
extremely fast analysis tool with both depth and breadth.

*Value propositions:* unification (`.map` + `stats.json`); performance (Rust +
WASM, second-level parsing of hundred-megabyte metadata, no OOM); privacy (pure
in-browser computation via WASM, so sensitive build metadata never leaves the
machine).

### 3. Feature requirements

**3.1 Data ingestion.** Automatic detection and parsing of: source maps v3
(all tools), webpack/rspack `stats.json` (streaming JSON; module tree, chunk
grouping, tree-shaking status), esbuild `metafile.json`, and
rollup/vite-plugin visualizer stats.

**3.2 Fusion engine (core moat).** When a `.map` and a `stats.json` are both
present: build the skeleton from the stats dependency graph (keeping chunk
boundaries and async relationships); fill it with reverse-mapped byte data to
compute each module's real post-gzip physical size (correcting the bundler's
estimates); cross-validate to identify **ghost code** (introduced but never
mapped — tree-shaking failures) and **hidden code** (inline/injected bytes with
no stats module).

**3.3 Visualisation.** A high-performance web UI with multi-dimensional
treemaps (source file / npm package / chunk / module dependency), smooth zoom,
global fuzzy search, code-level drill-down, dark mode, responsive layout.

**3.4 CLI and CI.** Zero-config `npx omnibundle ./dist` with auto-detection;
exports to `report.json`, `report.html`, `csv`; a size budget in
`omnibundle.config.json` that exits non-zero and blocks the pipeline.

**4. Non-functional requirements.** Performance: parse 500 MB of `stats.json`
plus its maps in **< 2 s** (Node tools typically need 10-30 s); peak memory
**< 500 MB**; treemap interaction **> 50 FPS** at 10,000+ modules.
Distribution: native single-file binaries for macOS (Intel/Apple Silicon),
Linux (x86/ARM), Windows, with no Node.js required; core compiled to WASM for
static hosting.

**5. Architecture blueprint.** Ingestion (`.map`, `stats.json`, `metafile.json`)
→ Rust core engine (map parser / stats parser / meta parser) → fusion engine
(unified bundle graph) → CLI output (JSON/HTML) and WASM module → web UI.

**6. Roadmap.** Phase 1 (M1-2) core chain + basic fusion + static HTML;
Phase 2 (M3-4) WASM + WebGL UI + browser-side analysis; Phase 3 (M5-6)
esbuild metafile, Rollup/Vite, CI budget; Phase 4 (M7+) diagnostics, history
trends, optional SaaS.

**7. Success metrics.** 2,000 GitHub stars within 6 months; 50,000 weekly npm
downloads; **≥5x faster than `webpack-bundle-analyzer`** in benchmarks;
memory down 70% on 500 MB inputs.

---

## Part 2 — Verification annotations (ours, 2026-09-02)

| PRD claim | Status | Evidence / action |
|---|---|---|
| Node tools need 10-30 s for 500 MB | **understated** | Measured: WBA 4.10.2 took **61.4 s** on a 381 MB stats file and **176.3 s** on 1,049 MB, peaking at **1,985 MB** RSS. The real problem is the memory ceiling, which is what we lead with. |
| Parse 500 MB + maps in < 2 s | **re-scoped** | Verified for the *parse* segment: a streaming prototype did 1,049 MB in **2.78 s at 59 MB RSS**. The full pipeline (including the attribution tree WBA spends ~60 s on) is **unverified** — target B3/B4 in `bench-spec.md`, first measured by WS-2. Reports must break time down per phase. |
| Peak memory < 500 MB | **partially verified** | Parse stage: 17 MB at 381 MB, 59 MB at 1,049 MB. Full fusion pipeline: **unverified** (B8). |
| `simd-json` for parsing | **changed** | ADR-0001: `simd-json` needs the whole document in memory, which would put the floor at ~2 GB for a 1 GB input. Replaced by streaming `serde_json` seeds, which match Node's parse speed at 1/50th the memory. |
| WebGL treemap, > 50 FPS at 10k modules | **deferred** | Phase 1 uses Canvas 2D (target > 30 FPS, B10); WebGL lands in Phase 2 behind the same payload contract. |
| Four grouping dimensions | **reduced** | Phase 1 ships source file / npm package / chunk. The module-dependency (edge) view needs its own interaction model and is Phase 2. |
| WASM in Phase 1 | **deferred** | `omnibundle-core` is already IO-free so Phase 2 is additive, but no `wasm-bindgen` dependency enters the tree until then (ADR-0003). |
| Ghost / hidden code detection | **specified, unproven** | The rule set is written down (`fusion::classify_ghost`) and gated: ≥95% agreement with hand-labelled expectations on three real projects before it ships. |
| ≥5x faster than WBA | **true for one segment only** | Parsing: 176.3 s → 2.78 s is ~63x. Full pipeline: unverified. Public claims must state which segment. |
| Memory down 70% at 500 MB | **likely exceeded** | 1,985 MB → 59 MB at 1,049 MB is ~97% at the parse stage. Re-state as a measured, segmented number after B3/B8. |
| SME is a bottleneck worth fusing with | **the open question** | We have never measured SME on a realistic `.map`. Its own issues report 45 s in `getWebTreeMapData` for a 10k-file bundle, with the rest of the pipeline at 3 s — and the project has not shipped since 2022-09. **WS-S measures this first**; if SME is sub-second in practice, the positioning changes (ADR-0003 stop-the-line #1). |
| "Privacy via in-browser WASM" | **Phase 2** | Same structural prerequisite, same deferral. |
| 2,000 stars / 50k weekly downloads | **aspirational** | Not an engineering target; we do not make roadmap promises we cannot verify. |

### Reading the vision honestly

Two parts of the vision are engineering claims (performance, memory), one is a
product bet (unification wins because teams are multi-bundler today), and one is
a distribution bet (npm + WASM). We are building to the engineering claims, using
the product bet to scope the work, and deferring the distribution bet to Phase 2
— with the reasoning recorded here rather than in a promise.
