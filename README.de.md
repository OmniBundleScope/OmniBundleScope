<div align="center">

# OmniBundle

**Ein Werkzeug für alle Bundler.** Den Abhängigkeitsgraphen aus `stats.json`, die
echte Byte-Zuordnung aus Source Maps, esbuild-Metafiles oder einfach einem
`dist/`-Ordner — zu einem Graphen zusammengeführt, mit einem Bruchteil des
Speichers.

[![CI](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml/badge.svg)](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/omnibundle-core.svg)](https://crates.io/crates/omnibundle-core)
[![npm](https://img.shields.io/npm/v/omnibundle.svg)](https://www.npmjs.com/package/omnibundle)
[![release](https://img.shields.io/github/v/release/omnibundle/omnibundle?include_prereleases&sort=semver)](https://github.com/omnibundle/omnibundle/releases/latest)
[![docs](https://img.shields.io/badge/docs-mdbook-informational)](docs/SUMMARY.md)
[![MSRV](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://doc.rust-lang.org/stable/notes.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[English](README.en.md) · [中文](README.zh.md) · [日本語](README.ja.md) · **Deutsch**

</div>

<p align="center">
  <a href="#das-problem">das Problem</a> ·
  <a href="#messwerte">Messwerte</a> ·
  <a href="#ghost-code-und-versteckter-code">Ghost &amp; versteckter Code</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#stand">Ehrlicher Stand</a> ·
  <a href="#warum">Warum so gebaut</a>
</p>

---

## Unterstützte Bundler

| Bundler | Was OmniBundle liest | Braucht `dist/` + `*.map`? | Ghost-Code-Erkennung |
|---|---|---|---|
| **webpack** 4 / 5 | `stats.json` | nein | ja |
| **rspack** | `stats.json` (gleiches Schema) | nein | ja |
| **esbuild** | `metafile.json` oder nur den Ausgabeordner | mit `--metafile` nein | mit `--metafile` ja |
| **Vite** | Ausgabeordner und seine Source Maps | ja | erfordert `--stats`-Ausgabe |
| **Rollup** | Ausgabeordner und seine Source Maps | ja | erfordert `stats.json` |
| **Parcel** | Ausgabeordner und seine Source Maps | ja | erfordert `stats.json` |
| **tsup / esbuild-Wrapper** | Ausgabeordner und seine Source Maps | ja | erfordert `stats.json` |
| **Angular / Next.js / Nuxt / SvelteKit** | was sie erzeugen, also webpack- oder vite-Ausgabe | je nach Fall | je nach Fall'
## Das Problem

Man lässt einen Bundle-Analyzer auf einen Build los, und er läuft in den
Speichernotstand oder braucht eine Minute und liefert ein Bild, mit dem man
nichts anfangen kann. `webpack-bundle-analyzer` hält die gesamte `stats.json` im
JS-Heap: bei einem 363-MB-Build sind das **63,6 Sekunden und 2,3 GB**, um die
Frage „wie groß ist das?“ zu beantworten. Und selbst wenn der Treemap gezeichnet
ist, kann er die beiden Dinge nicht sagen, die wirklich Geld kosten — siehe
[unten](#ghost-code-und-versteckter-code).

OmniBundle verarbeitet die Stats-Datei als Stream, misst die tatsächlich
erzeugten Bytes, fügt die Source Maps in den Modulgraphen ein und sagt, welche
Dimension verwendet wurde.

## Messwerte

Vollständige Pipeline — Stats parsen, jedes Asset auf der Platte messen, Source Maps
zusammenführen, Report rendern. Synthetische Fixtures, Median aus drei Läufen,
Referenzmaschine.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/chart-pipeline-dark.svg">
  <img alt="Balkendiagramme zum Vergleich von OmniBundle und webpack-bundle-analyzer. Laufzeit: 1,87 s gegenüber 63,6 s bei einer 363-MB-Stats-Datei, 6,14 s gegenüber 176,3 s bei 1 GB. Speicher: 127 MB gegenüber 2.295 MB sowie 376 MB gegenüber 1.437 MB." src="docs/assets/chart-pipeline-light.svg" width="100%">
</picture>

| Eingabe | OmniBundle | webpack-bundle-analyzer | Verhältnis |
|---|---|---|---|
| 363 MB `stats.json`, 154.379 Module | **1,87 s / 127 MB** | 63,6 s / 2.295 MB | **34× schneller, 18× kleiner** |
| 1 GB `stats.json`, 445.602 Module | **6,14 s / 376 MB** | 176,3 s / 1.437 MB | 29× schneller, 3,8× kleiner |

Source-Map-Zuordnung, aufgetragen über die Anzahl der Quellen in der Map. Das ist die
Superlinearität, die große Maps im Referenzwerkzeug unbrauchbar macht: **das
Fünffache an Daten kostet das Dreißigfache an Zeit**, während ours linear bleibt.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/chart-source-maps-dark.svg">
  <img alt="Log-log-Diagramme für Zeit und Speicher der Source-Map-Zuordnung über der Anzahl der Quellen. source-map-explorer steigt von 0,25 s bei 1.000 Quellen auf 562 s bei 50.000; OmniBundle von 4,6 ms auf 209 ms. Speicher bei 50.000 Quellen: 642 MB gegenüber 67 MB." src="docs/assets/chart-source-maps-light.svg" width="100%">
</picture>

Jede Zahl hier ist mit der Harness in `bench/` reproduzierbar, und die Rohdaten sind
eingecheckt: [`docs/assets/charts-data.json`](docs/assets/charts-data.json) listet jede
Angabe mit ihrer Herkunft, das [Evidenz-Log](docs/en/01-evidence.md) die Sitzungen
dahinter.

## Ghost-Code und versteckter Code

Die zwei Diagnosen, die keines der Referenzwerkzeuge liefern kann. Das ist das
eigentlich Neue an diesem Projekt.

- **Ghost-Code** — ein Modul, das der Bundler angegeben und ausgeliefert hat, das aber
  **keine Source Map erklärt**. Das Tree-Shaking hat es verpasst, oder das Asset wurde
  ohne Map gebaut. Es steckt im Bundle und in keinem Größenbericht.
- **Versteckter Code** — erzeugte Bytes, die zu **keinem** Modul gehören. Ein inline
  eingefügtes Stück, ein `eval`, ein injizierter Polyfill. Es steckt im Bundle und in
  keinem Modul.

OmniBundle rechnet den Byte-Anteil jeder Quelle den Modulen zu, die sie erzeugt haben,
und alles, was nicht aufgeht, wird zu einer Diagnose statt zu einem Rundungsfehler:

```
$ omnibundle ./dist
fused-app  ·  50000 modules  ·  1 assets  ·  0 packages  ·  ingest 4721 ms  ·  total 4864 ms  ·  dimension attributed
fusion: 1 map(s) · coverage 100% · 50000/50000 modules attributed · 0 ghost · 0 hidden source(s)
wrote dist/report.html (0.4 MB), detail in a companion script (loaded on demand)
```

und wenn es nicht aufgeht, sagt es in welche Richtung. Ein Build, bei dem nur eines von
zwei Assets eine Source Map mitbringt:

```
$ omnibundle ./dist
ob-partial  ·  11 modules  ·  2 assets  ·  0 packages  ·  ingest 4 ms  ·  total 5 ms  ·  dimension parsed
fusion: 1 map(s) · coverage 50% · 0/11 modules attributed · 11 ghost (226 KB of declared) · 11 hidden source(s) (45 KB)
```

Beachten Sie `dimension parsed`, nicht `attributed`: die Hälfte des Builds ist ohne Map,
also wurde die Ground-Truth-Dimension zurückgezogen, und der Report sagt das in seiner
ersten Zeile. Ein Werkzeug, das beides stillschweigend mittelt, würde eine Zahl melden,
die zu keiner Messung gehört.

Die Zuordnung erfolgt über das längste gemeinsame Pfad-Suffix auf dem echten
Join-Key ([`unified-graph.md`](docs/contracts/unified-graph.md)), **niemals** über
Content-Hashes, und die Summe der korrigierten Größen wird per Invariante gegen die
Asset-Summe geprüft — bei Verstoß gibt `OB0042` laut Fehler, statt still zu runden.

## Der Report

<p align="center">
  <img alt="OmniBundle-HTML-Report: ein squarified Treemap eines Builds, nach Paket gruppiert, mit durchsuchbarer Modulliste, drei Gruppierungsdimensionen und hellem und dunklem Design." src="docs/assets/treemap-large.svg" width="100%">
</p>

<sub>Nach Paket gruppiert, über 1.500 Assets hinweg aufsummiert — synthetisches Fixture,
8.021 Module, 400 Pakete, die Form eines großen Monorepo-Builds. Aus echten
Fixture-Daten erzeugt von `bench/harness/render-treemap-svg.mjs`, kein Screenshot; die
Beschriftungen sind die Ausgabe des Werkzeugs. Der HTML-Report bietet zusätzlich Suche,
drei Gruppierungsdimensionen, eine Detailansicht je Modul, Hell und Dunkel und keine
Netzwerkanfragen.</sub>

## Installation

```bash
# npm — lädt eine geprüfte Binärdatei, keine Rust-Toolchain nötig
npx omnibundle ./dist

# cargo
cargo install omnibundle-cli

# oder Binärdatei aus dem Release: linux x64/arm64, macOS x64/arm64, windows x64
# https://github.com/omnibundle/omnibundle/releases
```

Das npm-Paket prüft `checksums.txt`, bevor es irgendetwas schreibt oder ausführt, und
verweigert die Installation bei abweichender Prüfsumme.

## Verwendung

```bash
omnibundle ./dist                      # ein Ordner: stats + assets + *.map
omnibundle ./dist/stats.json           # nur die stats-Datei
omnibundle ./dist/metafile.json        # esbuild-Metafile

omnibundle ./dist --budget omnibundle.config.json   # Exit 1 bei Überschreitung
omnibundle ./dist --mode json > sizes.json          # für CI oder BI
omnibundle ./dist --mode csv  > sizes.csv
omnibundle ./dist/map.js.map --bench-map            # nur Zuordnung, mit Zeitmessung
```

```jsonc
// omnibundle.config.json
{
  "limits": [
    { "scope": "total",   "max": 1500000 },
    { "scope": "chunk",   "match": "vendor", "max": 800000 },
    { "scope": "package", "match": "moment", "max": 250000, "dimension": "gzip" }
  ]
}
```

<details>
<summary>Exit-Codes — Teil des Vertrags, denn CI-Gates bauen darauf</summary>

| Code | Bedeutung |
|---|---|
| 0 | analysiert, alle Budgets eingehalten |
| 1 | analysiert, ein Budget überschritten (`OB0040`) |
| 2 | ungültige Kommandozeile |
| 3 | Eingabe unlesbar, oder eine Budgetregel trifft auf nichts |

Eine Regel, die auf nichts trifft, ist **absichtlich** ein Fehler: ein Tippfehler in
einer Regel darf CI nicht stillschweigend bestehen lassen.

</details>

## Stand

Vor 1.0, und das ist die ehrliche Tabelle. Was nicht gemessen wurde, steht als solches da.

| Bereich | Zustand |
|---|---|
| stats-Eingabe, Größenzuordnung, Source Maps, Fusion, Report, Budgets, JSON/CSV | implementiert, gemessen, per CI abgesichert |
| Laufzeit der 1-GB-Eingabe | **verfehlt**: 4,40 s gegen ein Ziel von 3 s (Speicher mit 350 MB unkritisch) |
| Erster Report-Paint / 30 fps | **unverifiziert** — kein Browser in CI; stattdessen gemessen: 1,56 MB und 1,27 s bei 154.379 Modulen |
| WASM-Build, WebGL-Renderer | nicht begonnen |
| Windows / macOS / Linux | in CI auf allen drei getestet |
| 57 Rust-Tests, 4 npm-Tests, 1.018 erzeugte Layout-Fälle | grün |

Die einzige Verfehlung steht mit Ursache im [CHANGELOG](CHANGELOG.md): es ist der
DOM-Cursor von `serde_json` über ein Modul-Array mit 445.602 Elementen.

## Warum

Jede Entwurfsentscheidung folgt einer Messung, keiner Vorliebe.

| Entscheidung | Beleg |
|---|---|
| Streamendes JSON, nicht `simd-json` | `simd-json` braucht das ganze Dokument im Speicher — genau die Decke, die wir abräumen ([ADR-0001](docs/decisions/ADR-0001-streaming-over-simd-json.md)) |
| eigener HTML-Report, kein fremder Viewer | ein Viewer, den wir nicht kontrollieren, kann Fusion-Daten ohne Fork nicht zeigen ([ADR-0002](docs/decisions/ADR-0002-self-built-report.md)) |
| rayon für Größen und gzip | 25.600 Assets: 2.177 ms seriell → **342 ms** (6,4×); das Parsen war nie der Engpass |
| ein Suffix-Index für den Join | alle Quellen je Modul zu durchsuchen sind 2,5 Milliarden Vergleiche; der Index machte daraus 73,8 s → 4,9 s |
| exakte Zählungen, begrenzte Listen | ein Eintrag pro nicht gemapptem Modul kostete 47 MB für ein Feld, das *summary* heißt |
| Property-Tests statt Fixtures | zwei echte Fehler — Modulgrößen wurden **nie kleiner**, und ein Quellindex außerhalb der Liste ließ die Zuordnung still verschwinden — die kein Fixture im Repository sehen konnte |

### Was wir nicht gebaut haben, und warum

| verworfen | Grund |
|---|---|
| WBA-Viewer übernehmen | ohne Fork keine Fusion-Daten; mit Fork ist die UI unsere |
| `simd-json` | braucht das Dokument im Speicher, also genau das Problem |
| WebGL-Treemap in Phase 1 | 10k Knoten sind auf Canvas 2D in Ordnung; der Renderer-Wechsel ist Phase 2 hinter einem stabilen Payload |
| ein allgemeines Build-Werkzeug | gemessener Schmerz ist eine Speicherdecke, ein Prozess pro Element oder ein superlinearer Algorithmus. Hier liegt alle drei vor. |

## Dokumentation

- [Produktanforderungen](docs/en/00-prd.md) · [Evidenz-Log](docs/en/01-evidence.md) · [Architektur](docs/en/02-architecture.md)
- [Benchmarks und Ziele](docs/en/04-benchmark-plan.md) · [Parität und Tests](docs/en/05-parity-and-testing.md) · [Releases und CI](docs/en/06-release-and-ci.md)
- [Verträge](docs/contracts/) — Payload-Schema, CLI-Oberfläche, Benchmark-Protokoll, i18n-Regeln, Ownership
- [ADRs](docs/decisions/) · [Risikoregister](docs/en/07-risk-register.md) · [Roadmap](docs/en/08-roadmap.md)

> Die deutsche Dokumentation wird übersetzt. Englisch ist die **normative Quelle**;
> `node bench/harness/check-i18n.mjs` prüft, dass die vier Sprachen nicht auseinanderlaufen.

## Mitmachen

Jede Änderung bringt eine Messung mit oder ein Argument, warum sie keine haben kann.
Siehe [CONTRIBUTING.md](CONTRIBUTING.md); die Gates sind `cargo test`,
`clippy -D warnings` (inkl. pedantic), `cargo fmt`, eine Coverage-Untergrenze von
70 %, Parität gegen `webpack-bundle-analyzer` und eine Vier-Sprachen-Dokumentprüfung.
[Verhaltenskodex](CODE_OF_CONDUCT.md) · [Sicherheitsrichtlinie](SECURITY.md)

## Noch einmal: unterstützte Bundler

Weil es die erste Frage ist — und weil ein Projekt, das "alle Bundler" sagt, ohne
zu sagen welche, keine lesenswerte Behauptung aufstellt:

**webpack** (4 und 5, über `stats.json`) · **rspack** (über `stats.json`) ·
**esbuild** (über `metafile.json`) · **Vite** · **Rollup** · **Parcel** ·
**tsup** — sowie die Ausgabe von allem, was darauf aufbaut: **Angular**,
**Next.js**, **Nuxt**, **SvelteKit**, **React Server Components**.

Zwei Eingabeformen:

```bash
omnibundle ./dist/stats.json   # ein Bundler-Graph: webpack, rspack, esbuild --metafile
omnibundle ./dist              # nur der Ausgabeordner: vite, rollup, parcel, tsup
```

Die zweite braucht keine Konfiguration — auf `dist/` zeigen, und die Source Maps,
die der Bundler ohnehin schreibt, genügen. Die erste schaltet zusätzlich die
**Ghost-Code**-Erkennung frei, denn diese Frage braucht einen deklarierten
Modulgraphen.

| Bundler | Graph | Zuordnung | Ghost-Code |
|---|---|---|---|
| webpack, rspack | `stats.json` | Source Maps | ja |
| esbuild | `metafile.json` | Source Maps | mit Metafile |
| vite, rollup, parcel, tsup | nicht standardmäßig | Source Maps | mit `stats.json` |

---

## Projekt
## Lizenz

MIT ([LICENSE-MIT](LICENSE-MIT)) oder Apache-2.0
([LICENSE-APACHE](LICENSE-APACHE)), nach Ihrer Wahl.

---

<div align="center">
  <sub>
    öffentlich gebaut. Einwände zu den Zahlen sind willkommen in den
    <a href="https://github.com/omnibundle/omnibundle/issues">Issues</a> — sie sind das
    Einzige, was einen Benchmark bedeutsam hält.
  </sub>
</div>