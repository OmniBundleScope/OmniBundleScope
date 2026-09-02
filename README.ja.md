# OmniBundle

**すべてのバンドラーに対応する一つのツール。** ビルド成果物（`stats.json`、
ソースマップ、esbuild の metafile、あるいは単なる `dist/` ディレクトリ）を解析
し、webpack・rspack・Vite・Rollup・esbuild を横断した単一のサイズ見積もりを、
はるかに少ないメモリで提供します。

[English](README.en.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

ステータス: **Phase 1 進行中。** CLI サーフェスとデータ契約は凍結済みで、
取り込み・融合・レポートの各エンジンはその契約に沿って開発中です。

## 提供する機能

```bash
npx omnibundle ./dist
# → omnibundle-report.html   単一ファイル、ネットワーク通信なし
# → 終了コード 0。サイズバジェット超過時は OB0040 とともに終了コード 1
```

- **全バンドラーを一枚のグラフに。** webpack/rspack の `stats.json` が依存関係、
  ソースマップが実バイトの帰属、esbuild の metafile がモジュールグラフを与え、
  OmniBundle はこれを 1 つの統一グラフに融合します。
- **参照ツールでは出せない 2 つの診断。** *ゴーストコード*（バンドラーが宣言して
  いながら、いかなるマッピングでも説明できないバイト）と、*隠しコード*
  （インライン断片・`eval`・注入されたポリフィルなど、どのモジュールにも属さない
  生成バイト）。
- **メモリに収まる。** 実測: 1,049 MB の `stats.json` を **2.78 秒 / 59 MB RSS**
  で解析。同一入力での `webpack-bundle-analyzer` は **176.3 秒 / 1,437 MB**。
- **CI を止められるサイズバジェット。** `omnibundle.config.json` でチャンク別・
  パッケージ別・全体の制限を設け、超過時は非ゼロ終了します。

## なぜこの設計か

すべての判断は測定値から導かれています（推測ではありません）。
[エビデンス記録](docs/en/01-evidence.md)と [ADR](docs/decisions/) を参照して
ください。性能に関する主張は段階ごとに述べ、未測定の部分は推測せず
`unverified` と明記します。

## ドキュメント

- [製品要件と検証注記](docs/en/00-prd.md)
- [エビデンス](docs/en/01-evidence.md) · [アーキテクチャ](docs/en/02-architecture.md)
- [実装計画](docs/en/03-implementation-plan.md) · [ベンチマーク](docs/en/04-benchmark-plan.md)
- [契約](docs/contracts/) · [ADR](docs/decisions/) · [リスク](docs/en/07-risk-register.md)

## ライセンス

MIT OR Apache-2.0。
