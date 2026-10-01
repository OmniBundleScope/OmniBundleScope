# 用語集 — OmniBundleScope（JA）

規範リストは `docs/contracts/i18n-parity.md` §4 にあります。他の言語:
[EN](../en/GLOSSARY.md) · ZH: docs/zh/GLOSSARY.md · DE: docs/de/GLOSSARY.md

| EN | JA | 備考 |
|---|---|---|
| asset | ビルド成果物 | ビルドの出力ファイル。treemap のルート |
| chunk | チャンク | webpack の非同期読み込み単位。`initial` を持つ |
| module | モジュール | 依存グラフの 1 エントリ |
| source map | ソースマップ | v3。バイト帰属の基準となる ground truth |
| attribution | 帰属 | 生成バイトを元のソースへ対応づけること |
| fusion | 融合 | stats グラフと 1 つ以上のソースマップを統合すること |
| ghost code | ゴーストコード | バンドラーが宣言したが、どのマッピングでも説明できないバイト |
| hidden code | 隠しコード | どのモジュールにも属さない生成バイト |
| tree shaking | ツリーシェイキング | 未使用の export の除去 |
| treemap | ツリーマップ | squarified レイアウト。Phase 1 は Canvas 2D |
| budget | サイズバジェット | CI ゲート。超過時は終了コード 1 と `OBS0040` |
| baseline | ベースライン | 比較の基準となる数値 |
| size dimension | サイズ軸 | `stat` / `parsed` / `gzip` / `attributed` |
| phase | フェーズ | scan / parse / attribute / fuse / report |
| fixture | フィクスチャ | テストやベンチマークに使う固定入力 |
| real fixture | 実フィクスチャ | 実在のオープンソースプロジェクトから構築 |
| synthetic fixture | 合成フィクスチャ | 実プロジェクトに無い規模を作るために生成 |
| parity | 準拠性 | 許容誤差の範囲で参照ツールと一致すること |
| unverified | 未検証 | 未測定の目標にとって正当な状態 |
| streaming ingest | ストリーミング取り込み | 文書全体をメモリに載せずに解析する方法 |
| ceiling | 上限 | ツールを使えなくするメモリ/サイズの限界 |

## 翻訳しないもの

コード、識別子、ファイル名、CLI フラグ、JSON キー、診断コード
（`OBS0001` …）、crate 名、バージョン番号。これらを翻訳すると整合性チェックが
失敗します。
