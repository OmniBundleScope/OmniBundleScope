<div align="center">

# OmniBundle

**あらゆるバンドラーのための一つのツール。** `stats.json` から依存グラフ、source map から
実バイトの帰属、esbuild の metafile、あるいはただの `dist/` ディレクトリを、すべて同一の
グラフに統合します。メモリは仅仅その一部で済みます。

[![CI]({{REPO_URL}}/actions/workflows/ci.yml/badge.svg)]({{REPO_URL}}/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/{{CRATES_CORE_PACKAGE}}.svg)]({{CRATES_CORE_URL}})
[![npm](https://img.shields.io/npm/v/{{NPM_PACKAGE}}.svg)]({{NPM_URL}})
[![release](https://img.shields.io/github/v/release/{{REPO_SLUG}}?include_prereleases&sort=semver)]({{REPO_URL}}/releases/latest)
[![docs](https://img.shields.io/badge/docs-mdbook-informational)]({{DOCS_URL}})
[![MSRV](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://doc.rust-lang.org/stable/notes.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[English](README.en.md) · [中文](README.zh.md) · **日本語** · [Deutsch](README.de.md)

</div>

<p align="center">
  <a href="#問題">問題</a> ·
  <a href="#実測値">実測値</a> ·
  <a href="#ゴーストコードと隠しコード">ゴーストと隠しコード</a> ·
  <a href="#インストール">インストール</a> ·
  <a href="#現状">正直な現状</a> ·
  <a href="#なぜこう作るのか">なぜこう作るのか</a>
</p>

---

## サポートするバンドラー

| バンドラー | OmniBundle が読むもの | `dist/` + `*.map` が要る? | ゴーストコード検出 |
|---|---|---|---|
| **webpack** 4 / 5 | `stats.json` | 不要 | あり |
| **rspack** | `stats.json`（同じスキーマ） | 不要 | あり |
| **esbuild** | `metafile.json`、または出力フォルダのみ | `--metafile` なら不要 | `--metafile` があればあり |
| **Vite** | 出力フォルダとソースマップ | 必要 | `--stats` 出力を有効にする必要あり |
| **Rollup** | 出力フォルダとソースマップ | 必要 | `stats.json` が必要 |
| **Parcel** | 出力フォルダとソースマップ | 必要 | `stats.json` が必要 |
| **tsup / esbuild ラッパー** | 出力フォルダとソースマップ | 必要 | `stats.json` が必要 |
| **Angular / Next.js / Nuxt / SvelteKit** | それらが出力するもの（webpack か vite のビルド） | 場合による | 場合による'
## 問題

バンドル解析ツールをビルドに向けて差し出すと、メモリが足りないか、1 分かかり、
そして再利用できない図が返ってきます。`webpack-bundle-analyzer` は `stats.json` 全体を
JS ヒープに載せます。363 MB のビルドでは、それで「これ有多大？」に答えるために
**63.6 秒と 2.3 GB**。ツリーマップを描き終えても、本当にコストになっている二つのことは
依然として答えられません（[後述](#ゴーストコードと隠しコード)）。

OmniBundle は stats をストリームで読み、生成されたバイトを実測し、source map を
モジュールグラフに join し、使った次元を明示します。

## 実測値

フルパイプライン — stats のパース、ディスク上の全アセットの実測、source map の統合、
レポートの生成。合成 fixture、参照マシンで 3 回実行の中央値。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/chart-pipeline-dark.svg">
  <img alt="OmniBundle と webpack-bundle-analyzer を比較した横棒グラフ。所要時間: 363 MB の stats で 1.87 s 対 63.6 s、1 GB で 6.14 s 対 176.3 s。ピークメモリ: 127 MB 対 2,295 MB、および 376 MB 対 1,437 MB。" src="docs/assets/chart-pipeline-light.svg" width="100%">
</picture>

| 入力 | OmniBundle | webpack-bundle-analyzer | 比 |
|---|---|---|---|
| 363 MB `stats.json`、154,379 モジュール | **1.87 s / 127 MB** | 63.6 s / 2,295 MB | **34 倍速く、18 分の 1** |
| 1 GB `stats.json`、445,602 モジュール | **6.14 s / 376 MB** | 176.3 s / 1,437 MB | 29 倍速く、3.8 分の 1 |

Source map の帰属を、map に含まれる source 数で見たもの。これが大きな map を参照ツールで
扱えなくしている超線形性です。**データは 5 倍なのに時間は 30 倍になり**、こちらはほぼ直線です。

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/chart-source-maps-dark.svg">
  <img alt="source 数に対する source map 帰属の時間とメモリの両対数グラフ。source-map-explorer は 1,000 source の 0.25 s から 50,000 source の 562 s へ。OmniBundle は 4.6 ms から 209 ms へ。50,000 source でのメモリは 642 MB 対 67 MB。" src="docs/assets/chart-source-maps-light.svg" width="100%">
</picture>

ここにある数値はすべて `bench/` のハーネスで再現でき、原材料の記録もコミットされています:
[`docs/assets/charts-data.json`](docs/assets/charts-data.json) が出所を一覧し、
[証跡ログ](docs/en/01-evidence.md) が取得セッションを示します。

## ゴーストコードと隠しコード

参照ツールのどちらにも出せない二つの診断です。OmniBundle の本当の新しさです。

- **ゴーストコード** — バンドラーが宣言してバンドルしたのに、**どの source map にも
  説明が無い**モジュール。tree-shaking の取りこぼし、あるいは map 無しのアセット。
  成果物には入っているのに、どのサイズレポートにも現れません。
- **隠しコード** — **どのモジュールにも属さない**生成バイト。インライン化されたコード片、
  `eval`、バンドラーが注入した polyfill。成果物には入っているのに、どのモジュールの
  サイズにも属しません。

OmniBundle は各ソースのバイト配分をそれを生み出したモジュールへ畳み込み、帳尻が合わなかった
残りを丸め誤差ではなく診断に変えます:

```
$ omnibundle ./dist
dist  ·  50000 modules  ·  1 assets  ·  0 packages  ·  ingest 2143 ms  ·  total 2845 ms  ·  dimension attributed
fusion: 1 map(s) · coverage 100% · 50000/50000 modules attributed · ghost code needs a stats.json to detect · 0 hidden source(s) (0 KB)
wrote dist/report.html (0.1 MB), detail in a companion script (loaded on demand)
```

そして合わなかったときは、どちらの方向にズレたかを明示します。2 アセットのうち 1 つだけ
map を持つビルドの場合:

```
$ omnibundle ./dist
dist  ·  22 modules  ·  2 assets  ·  11 packages  ·  ingest 4 ms  ·  total 6 ms  ·  dimension parsed
fusion: 1 map(s) · coverage 47% · 0/22 modules attributed · 22 ghost (83 KB of declared) · 3 hidden source(s) (39 KB)
```

`dimension attributed` ではなく `dimension parsed` です。半分のビルドに map が無いので、
ground truth の次元は取り消され、レポートの 1 行目にそう書かれています。両者を
こっそり平均するツールは、どの測定にも属さない数字を報告してしまいます。

帰属は実際の join key 上の最長サフィックス一致で行います
（[`unified-graph.md`](docs/contracts/unified-graph.md)）。内容ハッシュは**使いません**。
修正後のサイズ合計はアセット合計との不変条件で検証し、勝手に丸めずに loud に失敗します
（`OB0042`）。

## レポート

<p align="center">
  <img alt="OmniBundle の HTML レポート: パッケージ別の squarified treemap で 400 パッケージ中最大の 40 を表示、検索可能なモジュール一覧、3 つのグルーピング軸、ライトとダークのテーマ。" src="docs/assets/treemap-large.svg" width="100%">
</p>

<sub>合成 fixture の 400 パッケージのうち最大の 40 パッケージ：1,500 アセット、
8,041 モジュール、サイズはバンドラーが宣言した値です。ツール自身の graph payload から
`bench/harness/render-treemap.py` が描画しており、スクリーンショットではありません。
そのためラベルはツールの出力そのものです。CI が再生成し、差分があれば失敗します。
HTML レポートにはさらに検索、3 つのグルーピング軸、モジュール単位の内訳、
ライト/ダーク、そしてネットワーク通信ゼロが含まれます。</sub>

## インストール

```bash
# npm — 検証済みバイナリを取得。Rust ツールチェーンは不要
npx omnibundle ./dist

# cargo
cargo install omnibundle-cli

# リリースバイナリ: linux x64/arm64, macOS x64/arm64, windows x64
# {{REPO_URL}}/releases
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

一致対象の無い規則は**意図的に**エラーです。規則の書き間違いで CI が黙って
通ってよいはずがありません。

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
| Rust テスト 57 件、npm テスト 4 件、生成レイアウト 1,018 ケース | すべて green |

唯一の未達項目は、原因は [CHANGELOG](CHANGELOG.md) に明記しています:
445,602 要素のモジュール配列に対する `serde_json` の DOM カーソルです。

## なぜこう作るのか

すべての設計判断は、好みではなく測定から生まれています。

| 判断 | 根拠 |
|---|---|
| ストリーミング JSON、`simd-json` ではない | `simd-json` は文書全体をメモリに要求します。それはまさに壊そうとしている天井です（[ADR-0001](docs/decisions/ADR-0001-streaming-over-simd-json.md)） |
| 自前の HTML レポート、viewer の vendor はしない | 制御できない viewer は fork 無しでは融合データを表示できません（[ADR-0002](docs/decisions/ADR-0002-self-built-report.md)） |
| サイズと gzip には rayon | 25,600 アセット: 直列 2,177 ms → **342 ms**（6.4 倍）。パースは元々ボトルネックではありません |
| join にサフィックス索引 | モジュールごとに全ソースを走査すると 25 億回の比較。索引で 73.8 s → 4.9 s |
| 正確な件数、有限のリスト | 未マップモジュールごとに 1 件持つことで、`summary` と書かれたフィールドに 47 MB 使っていました |
| fixture より性質テスト | 実際の欠陥を 2 つ捕まえました — モジュールサイズが**縮小しない**ことと、範囲外の source index で帰属が黙って消えること。どちらもリポジトリ内の fixture では見えませんでした |

###  만들らなかったものとその理由

|  Verworfen | Grund |
|---|---|
| WBA の viewer を取り込む | fork 無しでは融合データを表示できず、fork すれば UI の面倒は自分たちのもの |
| `simd-json` | 文書全体をメモリに要求します。それは問題そのもの |
| Phase 1 で WebGL treemap | 10k ノードは Canvas 2D で十分。レンダラ差し替えは安定した payload の後の Phase 2 |
| 汎用ビルドツール | 実測された痛みはメモリ天井、項目ごとのプロセス、または超線形アルゴリズム。すべて揃っています |

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
`clippy -D warnings`（pedantic 込み）、`cargo fmt`、70% のカバレッジ下限、
`webpack-bundle-analyzer` との整合性比較、4 言語ドキュメント検査です。
[行動規範](CODE_OF_CONDUCT.md) · [セキュリティ方針](SECURITY.md)

## もう一度、サポートするバンドラー

最初の疑問であり、「すべてのバンドラーをサポート」とだけ書いて、具体的に何を指すのかを示さない
プロジェクトには、読む価値がありません:

**webpack**（4 と 5、`stats.json` 経由）· **rspack**（`stats.json` 経由）·
**esbuild**（`metafile.json` 経由）· **Vite** · **Rollup** · **Parcel** · **tsup**、
およびそれらを基盤とするあらゆるビルドの出力——**Angular**、**Next.js**、
**Nuxt**、**SvelteKit**、**React Server Components**。

入力は 2 つの形:

```bash
omnibundle ./dist/stats.json   # バンドラーのグラフ: webpack, rspack, esbuild --metafile
omnibundle ./dist              # 出力フォルダのみ: vite, rollup, parcel, tsup
```

後者は設定不要です。`dist/` を指せば、バンドラーがすでに書いているソースマップが
十分です。前者は **ゴーストコード**検出を有効にします。あの問いには宣言された
モジュールグラフが必要だからです。

| バンドラー | グラフ | 帰属 | ゴーストコード |
|---|---|---|---|
| webpack、rspack | `stats.json` | ソースマップ | あり |
| esbuild | `metafile.json` | ソースマップ | metafile があればあり |
| vite、rollup、parcel、tsup | 既定ではない | ソースマップ | `stats.json` があればあり |

---

## プロジェクト
## ライセンス

MIT（[LICENSE-MIT](LICENSE-MIT)）または Apache-2.0（[LICENSE-APACHE](LICENSE-APACHE)）、お好きな方を。

---

<div align="center">
  <sub>
    公開で開発中。数値への異議は
    <a href="{{REPO_URL}}/issues">issues</a> へどうぞ。
    ベンチマークを意味あるままに保つ唯一の手段です。
  </sub>
</div>