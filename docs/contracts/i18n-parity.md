# Contract: documentation parity across EN / ZH / JA / DE

Status: **frozen**. Owner: WS-9. Checker: `bench/harness/check-i18n.mjs`.

## 1. Which documents are release-grade

| tier | files | rule |
|---|---|---|
| **release-grade** | `README.*.md`, `docs/*/00-prd.md`, `docs/*/GLOSSARY.*.md` | must exist in all four languages before any release |
| **working** | `01-evidence` … `08-roadmap` | EN is normative; translations are produced by translators after the EN text is frozen, and the checker reports them as `pending` until then |

A `pending` file is a normal, visible state. What is not allowed is a stale
translation pretending to be current — see §3.

## 2. Layout

```
docs/en/…  docs/zh/…  docs/ja/…  docs/de/…
README.md          -> index, points at the four language versions
README.en.md  README.zh.md  README.ja.md  README.de.md
```

English is the source of truth. Translations never get to change numbers,
command lines, file paths, code identifiers or exit codes; if a translation
disagrees with EN on any of those, the translation is wrong.

## 3. Parity rules the checker enforces

1. **Same file set** in all four trees, by name.
2. **Same heading count and order** per file.
3. **Same link targets** — relative links must resolve to files that exist in
   that language tree (or explicitly to the EN tree with a note).
4. **Same fenced code blocks and commands** — normalised, whitespace-insensitive.
5. **Same numbers** — every numeric literal ≥ 1000 in the EN text must appear
   identically in the translation. This is the rule that stops a "we measured
   2.78 s" from becoming "2,7 s" (locale) or "278 ms" (unit drift).
6. **Glossary conformance** — the terms in `GLOSSARY.*.md` must be used
   consistently inside each language tree.
7. **Header stamp** — every translated file carries
   `<!-- source: docs/en/<file> | version: <n> | status: translated -->`
   where `<n>` is the EN revision counter. A translation whose `version` is
   behind the EN file is reported as `stale` and blocks the release tier.

## 4. Glossary (canonical terms)

The authoritative list lives in each `GLOSSARY.*.md`. Head of the table:

| EN | ZH | JA | DE |
|---|---|---|---|
| asset (build output file) | 构建产物 | ビルド成果物 | Build-Artefakt |
| chunk | 分块 | チャンク | Chunk |
| module | 模块 | モジュール | Modul |
| source map | Source Map | ソースマップ | Source Map |
| attribution (byte → source) | 归因 | 帰属 | Zuordnung |
| fusion | 融合 | 融合 | Fusion |
| ghost code (declared, never mapped) | 幽灵代码 | ゴーストコード | Geist-Code |
| hidden code (mapped, not declared) | 隐藏代码 | 隠しコード | versteckter Code |
| tree shaking | Tree Shaking | ツリーシェイキング | Tree Shaking |
| treemap | 矩阵树图 | ツリーマップ | Treemap |
| budget (size gate) | 体积卡点 | サイズバジェット | Größen-Budget |
| baseline | 基线 | ベースライン | Ausgangswert |

## 5. Workflow for translators

1. wait for `status: frozen` on the EN file;
2. translate, keeping every number, command, path and code identifier verbatim;
3. add/update the header stamp with the new EN revision;
4. run `node bench/harness/check-i18n.mjs --lang <code>` and fix until clean;
5. do not "improve" the English technical content in the translation; if the EN
   is wrong, file an issue against the EN file instead.

## 6. What is explicitly not translated

- code, identifiers, file names, CLI flags, JSON keys, diagnostic codes
- crate/dependency names and version numbers
- the `<!-- source: … -->` header itself

Translating any of these breaks §3 and the checker will fail the build.
