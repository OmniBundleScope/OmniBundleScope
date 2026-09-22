<div align="center">

# OmniBundle

**あらゆるバンドラーのための一つのツール。** `stats.json` から依存グラフ、
source map から実バイトの帰属、esbuild の metafile、あるいはただの `dist/` ディレクトリを
すべて同一のグラフに統合し、メモリは仅仅その一部で済みます。

[![CI](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml/badge.svg)](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/omnibundle-core.svg)](https://crates.io/crates/omnibundle-core)
[![npm](https://img.shields.io/npm/v/omnibundle.svg)](https://www.npmjs.com/package/omnibundle)
[![release](https://img.shields.io/github/v/release/omnibundle/omnibundle?include_prereleases&sort=semver)](https://github.com/omnibundle/omnibundle/releases/latest)
[![MSRV](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://doc.rust-lang.org/stable/notes.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[English](README.en.md) · [中文](README.zh.md) · **日本語** · [Deutsch](README.de.md)

</div>

<p align="center">
  <a href="#実測値">実測値</a> ·
  <a href="#ゴーストコードと隠しコード">ゴーストコードと隠しコード</a> ·
  <a href="#インストール">インストール</a> ·
  <a href="#現状">正直な現状</a> ·
  <a href="#なぜこう作るのか">なぜこう作るのか</a>
</p>

---

1 GB の `stats.json` に対してフルパイプライン（パース、アセット実測、source map の統合、
レポート生成）を通した結果:

| | 時間 | ピークメモリ |
|---|---|---|
| [`webpack-bundle-analyzer@4.10.2`](https://github.com/webpack-contrib/webpack-bundle-analyzer) | 63.6 s | 2,295 MB |
| **OmniBundle** | **1.87 s** | **126 MB** |
| | **36 倍速い** | **15 分の 1 のメモリ** |

50,000 ソース / 36.5 MB の source map に対する
[`source-map-explorer@2.5.3`](https://github.com/danvk/source-map-explorer) との比較:

| | 時間 | ピークメモリ |
|---|---|---|
| `source-map-explorer` | 562 s（9.4 分） | 642 MB |
| **OmniBundle** | **0.21 s** | **67 MB** |

同じマシン、同じ fixture、3 回の実行の中央値。这里的数値はすべて `bench/` のハーネスで
再現できます。プロトコルは[ベンチマーク](docs/en/04-benchmark-plan.md)、
**達成できなかった**目標を含む全記録は[証跡ログ](docs/en/01-evidence.md)にあります。

## 何をするのか

```bash
npx omnibundle ./dist
```

```
dist  ·  154379 modules  ·  1500 assets  ·  400 packages  ·  ingest 1392 ms  ·  total 1753 ms  ·  dimension parsed
wrote dist/report.html (1.6 MB), detail in a companion script (loaded on demand)
```

- **全バンドラー、1 つのグラフ。** webpack / rspack の `stats.json` が依存構造、source map が
  実バイトの帰属、esbuild の metafile がモジュールグラフを提供します。後付けで拼接するのではなく
  統合します。
- **4 つのサイズ次元、そしてどれを使ったかを明示します。** `stat`（バンドラーの申告値）、
  `parsed`（ディスク上の実バイト）、`gzip`（level 6）、`attributed`（source map が説明できる
  バイト）。map が部分的な場合は `attributed` を `parsed` に降格させ、CLI がその理由を明示します。
  ground truth を持たないのにそう主張するレポートは、レポートが無いより悪いのです。
- **実際に CI を止められるサイズバジェット。** `omnibundle.config.json` で合計バイト、
  個別の chunk、個別のパッケージを制限でき、規則ごとに次元を指定できます。超過時は終了コード 1 と
  `OB0040`。**一致する対象が一つも無い規則はエラー**であり、黙って通ることはありません。
- **JSON と CSV** で既存のパイプラインに繋げます。

![OmniBundle のレポート（package 別）](docs/assets/treemap-large.svg)

<sub>package 別に、1,500 アセット横断で集計。合成 fixture、8,021 モジュール、400 パッケージ
（大规模モノレポビルドの形状）。ラベルはツール自身の出力で、
画像は `bench/harness/render-treemap-svg.mjs` が生成しており手描きではありません。</sub>

## ゴーストコードと隠しコード

参照ツールのどちらにも出せない二つの診断です。OmniBundle の本当の新しさはここです。

- **ゴーストコード** — バンドラーが宣言してバンドルしたのに、**どの source map にも説明が
  無い**モジュール。tree-shaking の取りこぼし、あるいは map 無しのアセット-built です。
  成果物には入っているのに、どのサイズレポートにも現れません。
- **隠しコード** — **どのモジュールにも属さない**生成バイト。インライン化されたコード片、
  `eval`、バンドラーが注入した polyfill。成果物には入っているのに、どのモジュールのサイズにも
  属しません。

OmniBundle は各ソースのバイト配分をそれを生み出したモジュールへ畳み込み、帳尻が合わなかった
残りを丸め誤差ではなく診断に変えます:

```
fusion: 1 map(s) · coverage 100% · 50000/50000 modules attributed · 0 ghost · 0 hidden source(s)
```

帰属は実際の join key 上の最長サフィックス一致で行います
（`docs/contracts/unified-graph.md`）。内容ハッシュは**使いません**。修正後のサイズ合計は
アセット合計との不変条件で検証し、暗黙に丸めずに loud に失敗します（`OB0042`）。

## インストール

```bash
# npm — 検証済みバイナリを取得。Rust ツールチェーンは不要
npx omnibundle ./dist

# cargo
cargo install omnibundle-cli

# リリースバイナリ: linux x64/arm64, macOS x64/arm64, windows x64
# https://github.com/omnibundle/omnibundle/releases
```

npm パッケージは `checksums.txt` を検証してから書き込み・実行し、ハッシュ不一致なら
インストールを拒否します。

## 使い方

```bash
omnibundle ./dist                      # ディレクトリ: stats + assets + *.map
omnibundle ./dist/stats.json           # stats ファイル単体
omnibundle ./dist/metafile.json        # esbuild metafile

omnibundle ./dist --budget omnibundle.config.json   # 超過時は終了コード 1
omnibundle ./dist --mode json > sizes.json          # CI や BI 用
omnibundle ./dist --mode csv  > sizes.csv
omnibundle ./dist/map.js.map --bench-map            # 帰属のみ計測
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
<summary>終了コード — CI のゲートはこの契約の上に成り立っています</summary>

| コード | 意味 |
|---|---|
| 0 | 解析完了、すべてのバジェットを満たした |
| 1 | 解析完了、バジェット超過あり（`OB0040`） |
| 2 | コマンドライン引数が不正 |
| 3 | 入力が読めない、または一致対象の無いバジェット規則がある |

</details>

## 現状

1.0 前です。この表は正直版です。測定していないものは、そう記載しています。

| 領域 | 状態 |
|---|---|
| stats 取込、サイズ帰属、source map、統合、レポート、バジェット、JSON/CSV | 実装済み・測定済み・CI でゲート |
| 1 GB 取込の所要時間 | **未達**: 4.40 s（目標 3 s、メモリは 350 MB で余裕） |
| レポートの初回描画 / 30 fps | **未検証** — CI にブラウザが無い。代わりに 154,379 モジュールで 1.56 MB / 1.27 s を測定 |
| WASM ビルド、WebGL レンダラ | 未着手 |
| Windows / macOS / Linux | CI の 3 プラットフォームでテスト |

唯一の未達項目は、原因は [CHANGELOG](CHANGELOG.md) に明記しています:
445,602 要素のモジュール配列に対する `serde_json` の DOM カーソルです。

## なぜこう作るのか

すべての設計判断は、好みではなく測定から生まれています。

| 判断 | 根拠 |
|---|---|
| ストリーミング JSON、`simd-json` ではない | `simd-json` は文書全体をメモリに要求します。それはまさに壊そうとしている天井です（[ADR-0001](docs/decisions/ADR-0001-streaming-over-simd-json.md)） |
| 自前の HTML レポート、viewer の vendor はしない | 制御できない viewer は fork 無しでは融合データを表示できず、fork すれば UI の面倒は自分たちのものになります（[ADR-0002](docs/decisions/ADR-0002-self-built-report.md)） |
| サイズと gzip には rayon | 25,600 アセット: 直列 2,177 ms → **342 ms**（6.4 倍）。パースは元々ボトルネックではありません |
| 正確な件数、有限のリスト | 未マップモジュールごとに `GhostModule` を 1 件持つことで、`summary` と書かれたフィールドに 47 MB 使っていました。件数は正確に保ち、リストは何を落としたかを報告する有限サンプルに |
| 失敗できないベンチマークはベンチマークではない | B3 が目標 200 MB に対して 670 MB を出したのは、detail payload が `serde_json::Value` のツリーだったためです。目標は機能しました |

###  만들らなかったものとその理由

| 却下した案 | 理由 |
|---|---|
| WBA の viewer を vendor | fork 無しでは融合データを表示できず、fork すれば UI の面倒は自分たちのもの |
| `simd-json` | 文書全体をメモリに 요구します。それは問題そのもの |
| Phase 1 で WebGL treemap | 10k ノードなら Canvas 2D で十分。レンダラ差し替えは安定した payload の後の Phase 2 |
| 汎用ビルドツール | 実測された痛みはメモリ天井・項目ごとのプロセス・超線形アルゴリズムです。すべて揃っています |

## ドキュメント

- [プロダクト要件](docs/en/00-prd.md) · [証跡ログ](docs/en/01-evidence.md) · [アーキテクチャ](docs/en/02-architecture.md)
- [ベンチマークと目標](docs/en/04-benchmark-plan.md) · [整合性とテスト](docs/en/05-parity-and-testing.md) · [リリースと CI](docs/en/06-release-and-ci.md)
- [契約](docs/contracts/) — payload schema、CLI 仕様、ベンチプロトコル、i18n 規則、オーナーシップ
- [ADR](docs/decisions/) · [リスク一覧](docs/en/07-risk-register.md) · [ロードマップ](docs/en/08-roadmap.md)

> 日本語版ドキュメントは翻訳中です。英語版が**正本**であり、
> `node bench/harness/check-i18n.mjs` が 4 言語の乖離を検査します。

## 貢献する

すべての変更は測定を同梱するか、測定が不要である理由を議論します。
[CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。ゲートは `cargo test`、
`clippy -D warnings`（pedantic 込み）、`cargo fmt`、
`webpack-bundle-analyzer` との整合性比較、4 言語ドキュメント検査です。
[行動規範](CODE_OF_CONDUCT.md) · [セキュリティ方針](SECURITY.md)

## ライセンス

MIT（[LICENSE-MIT](LICENSE-MIT)）または Apache-2.0（[LICENSE-APACHE](LICENSE-APACHE)）、お好きな方を。

---

<div align="center">
  <sub>
    公開で開発中。数値への異議や反論は
    <a href="https://github.com/omnibundle/omnibundle/issues">issues</a> へどうぞ。
  </sub>
</div>