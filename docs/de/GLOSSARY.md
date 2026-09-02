# Glossar — OmniBundle (DE)

Die kanonische Liste steht in `docs/contracts/i18n-parity.md` §4. Andere
Sprachen: [EN](../en/GLOSSARY.md) · [ZH](../zh/GLOSSARY.md) · [JA](../ja/GLOSSARY.md)

| EN | DE | Anmerkung |
|---|---|---|
| asset | Build-Artefakt | eine vom Build erzeugte Datei; Wurzel des Treemaps |
| chunk | Chunk | webpacks Einheit des asynchronen Ladens; hat `initial` |
| module | Modul | ein Eintrag des Abhängigkeitsgraphen |
| source map | Source Map | v3; die Referenz für die Byte-Zuordnung |
| attribution | Zuordnung | erzeugte Bytes auf Originalquellen zurückführen |
| fusion | Fusion | den Stats-Graphen mit einer oder mehreren Source Maps zusammenführen |
| ghost code | Geist-Code | vom Bundler deklariert, aber von keiner Zuordnung erklärt |
| hidden code | versteckter Code | erzeugte Bytes, die zu keinem Modul gehören |
| tree shaking | Tree Shaking | Entfernen ungenutzter Exporte |
| treemap | Treemap | Squarified-Layout; in Phase 1 mit Canvas 2D |
| budget | Größen-Budget | CI-Gate; Verletzung ⇒ Exit-Code 1 und `OB0040` |
| baseline | Ausgangswert | der Referenzwert, gegen den gemessen wird |
| size dimension | Größendimension | `stat` / `parsed` / `gzip` / `attributed` |
| phase | Phase | scan / parse / attribute / fuse / report |
| lane / workstream | Arbeitsstrang | Einheit mit exklusiven Pfaden (WS-0 … WS-9) |
| fixture | Fixture | feste Eingabe für Tests oder Benchmarks |
| real fixture | echtes Fixture | aus einem echten Open-Source-Projekt gebaut |
| synthetic fixture | synthetisches Fixture | erzeugt, um eine Größenordnung zu erreichen, die reale Projekte nicht haben |
| parity | Parität | Übereinstimmung mit einem Referenzwerkzeug innerhalb einer Toleranz |
| unverified | nicht verifiziert | ein zulässiger Zustand für ungemessene Ziele |
| streaming ingest | Streaming-Eingabe | Parsen, ohne das ganze Dokument zu materialisieren |
| ceiling | Obergrenze | die Speicher-/Größengrenze, die ein Werkzeug unbrauchbar macht |

## Nicht übersetzen

Code, Bezeichner, Dateinamen, CLI-Flags, JSON-Schlüssel, Diagnosecodes
(`OB0001` …), Crate-Namen, Versionsnummern. Wer eines davon übersetzt, bricht
die Paritätsprüfung.
