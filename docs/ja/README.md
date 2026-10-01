# ドキュメント索引 / Documentation index (JA)

英語版が**規範となる原本**（normative source）です。このディレクトリは翻訳と
ステータスの記録です。規則は `docs/contracts/i18n-parity.md` を参照してください。

| ファイル | 英語原本 | 状態 |
|---|---|---|
| README | [README.ja.md](../../README.ja.md) | ✅ リリース級 |
| GLOSSARY | [GLOSSARY.md](GLOSSARY.md) | ✅ リリース級 |
| 00 PRD 製品要件 | [en/00-prd.md](../en/00-prd.md) | ⏳ 翻訳キュー（リリース級） |
| 01 エビデンス | [en/01-evidence.md](../en/01-evidence.md) | ⏳ 翻訳キュー |
| 02 アーキテクチャ | [en/02-architecture.md](../en/02-architecture.md) | ⏳ 翻訳キュー |
| 03 実装計画 | [en/03-implementation-plan.md](../en/03-implementation-plan.md) | ⏳ 翻訳キュー |
| 04 ベンチマーク | [en/04-benchmark-plan.md](../en/04-benchmark-plan.md) | ⏳ 翻訳キュー |
| 05 準拠性とテスト | [en/05-parity-and-testing.md](../en/05-parity-and-testing.md) | ⏳ 翻訳キュー |
| 06 リリースと CI | [en/06-release-and-ci.md](../en/06-release-and-ci.md) | ⏳ 翻訳キュー |
| 07 リスク登録簿 | [en/07-risk-register.md](../en/07-risk-register.md) | ⏳ 翻訳キュー |
| 08 ロードマップ | [en/08-roadmap.md](../en/08-roadmap.md) | ⏳ 翻訳キュー |
| 契約 / ADR | [unified graph](../contracts/unified-graph.md) · [CLI](../contracts/cli-surface.md) · [ADRs](../decisions/ADR-0001-streaming-over-simd-json.md) | 🔒 英語のみ（規範） |

## 翻訳の規則

1. 英語原本が frozen になるまで待ってから翻訳を開始する
2. **数値・コマンド・パス・識別子・終了コードは逐語のまま**（例: `2.78 s`、
   `176.3 s`、`OBS0040`、`--dims`、`check_size_invariant`）
3. ファイル頭に `<!-- source: docs/en/<file> | version: <n> | status: translated -->`
   を記載する
4. `node bench/harness/check-i18n.mjs --lang ja` がゼロ差分になるまで直す
5. 翻訳の中で英語の技術的な記述を「改善」しない。問題があれば英語原本に
   issue を起票する

## 用語

[GLOSSARY.md](GLOSSARY.md) を参照。
