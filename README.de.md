# OmniBundle

**Ein Werkzeug für alle Bundler.** Analysiert die Ausgabe eines Builds —
`stats.json`, Source Maps, esbuild-Metafiles oder einfach einen `dist/`-Ordner —
und liefert eine gemeinsame Größenbilanz für webpack, rspack, Vite, Rollup und
esbuild, mit einem Bruchteil des Speichers.

[English](README.en.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

Status: **Phase 1 in Arbeit.** CLI-Oberfläche und Datenverträge sind
eingefroren; die Engines für Eingabe, Fusion und Report werden dagegen gebaut.

## Was es leisten wird

```bash
npx omnibundle ./dist
# → omnibundle-report.html   eine eigenständige Datei, keine Netzwerkzugriffe
# → Exit-Code 0, oder Exit-Code 1 mit OB0040 bei verletztem Größen-Budget
```

- **Ein Graph für alle Bundler.** `stats.json` liefert die Abhängigkeitsstruktur,
  Source Maps die echte Byte-Zuordnung, esbuild-Metafiles den Modulgraph;
  OmniBundle führt alles zu einem einheitlichen Graphen zusammen.
- **Zwei Diagnosen, die die Referenzwerkzeuge nicht liefern können.**
  *Geist-Code*: vom Bundler deklariert, aber von keiner Zuordnung erklärt.
  *Versteckter Code*: erzeugte Bytes, die zu keinem Modul gehören (Inlines,
  `eval`, injizierte Polyfills).
- **Es passt in den Speicher.** Gemessen bei einer `stats.json` von 1.049 MB:
  **2,78 s bei 59 MB RSS**, gegenüber **176,3 s bei 1.437 MB** für
  `webpack-bundle-analyzer` bei derselben Eingabe.
- **Budgets, die CI stoppen.** `omnibundle.config.json` begrenzt pro Chunk, pro
  Paket oder insgesamt; eine Verletzung führt zu einem Exit-Code ungleich null.

## Warum es so gebaut ist

Jede Entscheidung folgt einer Messung, keiner Präferenz — siehe
[Evidenzprotokoll](docs/en/01-evidence.md) und die
[ADRs](docs/decisions/). Leistungsangaben werden je Phase genannt; was nicht
gemessen ist, wird als `unverified` markiert und nicht geschätzt.

## Dokumentation

- [Produktanforderungen mit Prüfvermerken](docs/en/00-prd.md)
- [Evidenzprotokoll](docs/en/01-evidence.md) · [Architektur](docs/en/02-architecture.md)
- [Implementierungsplan](docs/en/03-implementation-plan.md) · [Benchmarks](docs/en/04-benchmark-plan.md)
- [Verträge](docs/contracts/) · [ADRs](docs/decisions/) · [Risiken](docs/en/07-risk-register.md)

## Lizenz

MIT OR Apache-2.0.
