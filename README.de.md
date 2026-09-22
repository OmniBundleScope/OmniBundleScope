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
[![MSRV](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://doc.rust-lang.org/stable/notes.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[English](README.en.md) · [中文](README.zh.md) · [日本語](README.ja.md) · **Deutsch**

</div>

<p align="center">
  <a href="#messwerte">Messwerte</a> ·
  <a href="#ghost-code-und-versteckter-code">Ghost &amp; versteckter Code</a> ·
  <a href="#installation">Installation</a> ·
  <a href="#stand">Ehrlicher Stand</a> ·
  <a href="#warum">Warum so gebaut</a>
</p>

---

Gemessen an einer 1-GB-`stats.json`, vollständige Pipeline — parsen, jedes Asset
messen, Source Maps zusammenführen, Report rendern:

| | Zeit | Spitzenbelegung |
|---|---|---|
| [`webpack-bundle-analyzer@4.10.2`](https://github.com/webpack-contrib/webpack-bundle-analyzer) | 63,6 s | 2.295 MB |
| **OmniBundle** | **1,87 s** | **126 MB** |
| | **34× schneller** | **18× kleiner** |

Und auf einer 36,5-MB-Source-Map mit 50.000 Quellen, gegen
[`source-map-explorer@2.5.3`](https://github.com/danvk/source-map-explorer):

| | Zeit | Spitzenbelegung |
|---|---|---|
| `source-map-explorer` | 562 s (9,4 min) | 642 MB |
| **OmniBundle** | **0,21 s** | **67 MB** |

Gleiche Maschine, gleiche Fixtures, Median aus drei Läufen. Jede Zahl hier ist mit
der Harness in `bench/` reproduzierbar — das Protokoll steht in
[Benchmarks](docs/en/04-benchmark-plan.md), das vollständige Protokoll einschließlich
der **nicht** erreichten Ziele in der [Evidenz-Log](docs/en/01-evidence.md).

## Was es tut

```bash
npx omnibundle ./dist
```

```
dist  ·  154379 modules  ·  1500 assets  ·  400 packages  ·  ingest 1392 ms  ·  total 1753 ms  ·  dimension parsed
wrote dist/report.html (1.6 MB), detail in a companion script (loaded on demand)
```

- **Ein Graph, jeder Bundler.** `stats.json` von webpack und rspack liefert die
  Abhängigkeitsstruktur, Source Maps die echte Byte-Zuordnung, esbuild-Metafiles den
  Modulgraphen. Zusammengeführt, nicht nachträglich zusammengestückt.
- **Vier Größendimensionen — und es sagt, welche verwendet wurde.** `stat` (was der
  Bundler angegeben hat), `parsed` (echte Bytes auf der Platte), `gzip` (Level 6) und
  `attributed` (Bytes, die eine Source Map tatsächlich erklärt). Eine unvollständige
  Map stuft `attributed` auf `parsed` herab, und die CLI sagt warum. Ein Report, der
  Ground Truth behauptet, die er nicht hat, wäre schlimmer als kein Report.
- **Budgets, die CI wirklich stoppen.** `omnibundle.config.json` begrenzt Gesamtbytes,
  ein Chunk oder ein Paket, je Regel mit wählbarer Dimension. Eine Überschreitung
  endet mit Exit-Code 1 und `OB0040`; eine Regel, die auf nichts trifft, ist ein
  Fehler und niemals ein stilles Bestehen.
- **JSON und CSV** für den Rest der Pipeline.

![OmniBundle-Report, nach Paket gruppiert](docs/assets/treemap-large.svg)

<sub>Nach Paket gruppiert, über 1.500 Assets hinweg aufsummiert. Synthetisches
Fixture, 8.021 Module, 400 Pakete — die Form, die ein großer Monorepo-Build hat.
Die Beschriftungen sind die Ausgabe des Werkzeugs; das Bild erzeugt
`bench/harness/render-treemap-svg.mjs`, es ist nicht von Hand gezeichnet.</sub>

## Ghost-Code und versteckter Code

Die zwei Diagnosen, die keines der Referenzwerkzeuge liefern kann. Das ist das
eigentlich Neue an diesem Projekt.

- **Ghost-Code** — ein Modul, das der Bundler angegeben und ausgeliefert hat, das aber
  **keine Source Map erklärt**. Das Tree-Shaking hat es verpasst, oder das Asset wurde
  ohne Map gebaut. Es steckt im Bundle und in keinem Größenbericht.
- **Versteckter Code** — erzeugte Bytes, die zu **keinem** Modul gehören. Ein
  inline eingefügtes Stück, ein `eval`, ein injizierter Polyfill. Es steckt im Bundle
  und in keinem Modul.

OmniBundle rechnet den Byte-Anteil jeder Quelle den Modulen zu, die sie erzeugt haben,
und alles, was nicht aufgeht, wird zu einer Diagnose statt zu einem Rundungsfehler:

```
fusion: 1 map(s) · coverage 100% · 50000/50000 modules attributed · 0 ghost · 0 hidden source(s)
```

Die Zuordnung erfolgt über das längste gemeinsame Pfad-Suffix auf dem echten
Join-Key (`docs/contracts/unified-graph.md`), **niemals** über Content-Hashes, und die
Summe der korrigierten Größen wird per Invariante gegen die Asset-Summe geprüft — bei
Verstoß gibt `OB0042` laut Fehler, statt still zu runden.

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

</details>

## Stand

Vor 1.0, und diese Tabelle ist die ehrliche. Was nicht gemessen wurde, steht als solches da.

| Bereich | Zustand |
|---|---|
| stats-Eingabe, Größenzuordnung, Source Maps, Fusion, Report, Budgets, JSON/CSV | implementiert, gemessen, per CI abgesichert |
| Wandzeit der 1-GB-Eingabe | **verfehlt**: 4,40 s gegen ein Ziel von 3 s (Speicher mit 350 MB unkritisch) |
| Erster Report-Paint / 30 fps | **unverifiziert** — kein Browser in CI; stattdessen gemessen: 1,56 MB und 1,27 s bei 154.379 Modulen |
| WASM-Build, WebGL-Renderer | nicht begonnen |
| Windows / macOS / Linux | in CI auf allen drei getestet |

Die einzige Verfehlung steht mit Ursache im [CHANGELOG](CHANGELOG.md): es ist der
DOM-Cursor von `serde_json` über ein Modul-Array mit 445.602 Elementen.

## Warum

Jede Entwurfsentscheidung folgt einer Messung, keiner Vorliebe.

| Entscheidung | Beleg |
|---|---|
| Streamendes JSON, nicht `simd-json` | `simd-json` braucht das ganze Dokument im Speicher — genau die Decke, die wir abräumen ([ADR-0001](docs/decisions/ADR-0001-streaming-over-simd-json.md)) |
| eigener HTML-Report, kein fremder Viewer | ein Viewer, den wir nicht kontrollieren, kann Fusion-Daten ohne Fork nicht zeigen ([ADR-0002](docs/decisions/ADR-0002-self-built-report.md)) |
| rayon für Größen und gzip | 25.600 Assets: 2.177 ms seriell → **342 ms** (6,4×); das Parsen war nie der Engpass |
| exakte Zählungen, begrenzte Listen | ein `GhostModule` pro nicht gemapptem Modul kostete 47 MB für ein Feld, das *summary* heißt; die Zählungen bleiben exakt, die Listen sind Stichproben, die sagen, was sie verworfen haben |
| Ein Benchmark, der nicht scheitern kann, ist keiner | B3 maß 670 MB gegen ein 200-MB-Ziel, weil der Detail-Payload ein `serde_json::Value`-Baum war. Das Ziel hat gewirkt. |

### Was wir nicht gebaut haben, und warum

| Verworfen | Grund |
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

> Die deutsche Dokumentation wird übersetzt. Englisch ist die ** normative Quelle**;
> `node bench/harness/check-i18n.mjs` prüft, dass die vier Sprachen nicht auseinanderlaufen.

## Mitmachen

Jede Änderung bringt eine Messung mit oder ein Argument, warum sie keine haben kann.
Siehe [CONTRIBUTING.md](CONTRIBUTING.md); die Gates sind `cargo test`,
`clippy -D warnings` (inkl. pedantic), `cargo fmt`, Parität gegen
`webpack-bundle-analyzer` und eine Vier-Sprachen-Dokumentprüfung.
[Verhaltenskodex](CODE_OF_CONDUCT.md) · [Sicherheitsrichtlinie](SECURITY.md)

## Lizenz

MIT ([LICENSE-MIT](LICENSE-MIT)) oder Apache-2.0
([LICENSE-APACHE](LICENSE-APACHE)), nach Ihrer Wahl.

---

<div align="center">
  <sub>
    öffentlich gebaut. Einwände, Widersprüche und Korrekturen zu Zahlen sind willkommen
    in den <a href="https://github.com/omnibundle/omnibundle/issues">Issues</a>.
  </sub>
</div>