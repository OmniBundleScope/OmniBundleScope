# Dokumentationsindex (DE)

Die englische Fassung ist die **normative Quelle**; dieses Verzeichnis enthält
Übersetzungen und Statusangaben. Die Regeln stehen in
`docs/contracts/i18n-parity.md`.

| Datei | Englische Quelle | Status |
|---|---|---|
| README | [README.de.md](../../README.de.md) | ✅ release-stufe |
| GLOSSARY | [GLOSSARY.md](GLOSSARY.md) | ✅ release-stufe |
| 00 PRD Produkzanforderungen | [en/00-prd.md](../en/00-prd.md) | ⏳ Übersetzungswarteschlange (Release) |
| 01 Evidenzprotokoll | [en/01-evidence.md](../en/01-evidence.md) | ⏳ Warteschlange |
| 02 Architektur | [en/02-architecture.md](../en/02-architecture.md) | ⏳ Warteschlange |
| 03 Implementierungsplan | [en/03-implementation-plan.md](../en/03-implementation-plan.md) | ⏳ Warteschlange |
| 04 Benchmarks | [en/04-benchmark-plan.md](../en/04-benchmark-plan.md) | ⏳ Warteschlange |
| 05 Parität und Tests | [en/05-parity-and-testing.md](../en/05-parity-and-testing.md) | ⏳ Warteschlange |
| 06 Release und CI | [en/06-release-and-ci.md](../en/06-release-and-ci.md) | ⏳ Warteschlange |
| 07 Risikoregister | [en/07-risk-register.md](../en/07-risk-register.md) | ⏳ Warteschlange |
| 08 Roadmap | [en/08-roadmap.md](../en/08-roadmap.md) | ⏳ Warteschlange |
| Verträge / ADR | [contracts](../contracts/) · [decisions](../decisions/) | 🔒 nur Englisch (normativ) |

## Übersetzungsregeln

1. Warten, bis die englische Quelle als `frozen` markiert ist
2. **Zahlen, Befehle, Pfade, Bezeichner und Exit-Codes bleiben wörtlich**
   (z. B. `2.78 s`, `176.3 s`, `FS0040`, `--dims`, `check_size_invariant`)
3. Kopfzeile ergänzen:
   `<!-- source: docs/en/<file> | version: <n> | status: translated -->`
4. `node bench/harness/check-i18n.mjs --lang de` muss null Differenzen melden
5. Technische Aussagen in der Übersetzung nicht „verbessern“; stattdessen ein
   Issue gegen die englische Quelle eröffnen

## Begriffe

Siehe [GLOSSARY.md](GLOSSARY.md).
