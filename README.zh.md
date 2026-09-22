<div align="center">

# OmniBundle

**一个工具，分析所有打包器。** 依赖图来自 `stats.json`，真实字节归因来自 source map，
esbuild metafile，或者一个普通的 `dist/` 目录——合并成同一张图，内存占用只占对方的一小部分。

[![CI](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml/badge.svg)](https://github.com/omnibundle/omnibundle/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/omnibundle-core.svg)](https://crates.io/crates/omnibundle-core)
[![npm](https://img.shields.io/npm/v/omnibundle.svg)](https://www.npmjs.com/package/omnibundle)
[![release](https://img.shields.io/github/v/release/omnibundle/omnibundle?include_prereleases&sort=semver)](https://github.com/omnibundle/omnibundle/releases/latest)
[![MSRV](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://doc.rust-lang.org/stable/notes.html)
[![license](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

[English](README.en.md) · **中文** · [日本語](README.ja.md) · [Deutsch](README.de.md)

</div>

<p align="center">
  <a href="#实测数据">实测数据</a> ·
  <a href="#幽灵代码与隐藏代码">幽灵代码与隐藏代码</a> ·
  <a href="#安装">安装</a> ·
  <a href="#当前状态">诚实的状态</a> ·
  <a href="#为什么这样做">为什么这样做</a>
</p>

---

在一份 1 GB 的 `stats.json` 上跑完整流程——解析、测量每个 asset、融合 source map、生成报告：

| | 耗时 | 峰值内存 |
|---|---|---|
| [`webpack-bundle-analyzer@4.10.2`](https://github.com/webpack-contrib/webpack-bundle-analyzer) | 63.6 s | 2,295 MB |
| **OmniBundle** | **1.87 s** | **126 MB** |
| | **快 36 倍** | **内存少 15 倍** |

在一份 50,000 个 source、36.5 MB 的 source map 上，对比
[`source-map-explorer@2.5.3`](https://github.com/danvk/source-map-explorer)：

| | 耗时 | 峰值内存 |
|---|---|---|
| `source-map-explorer` | 562 s（9.4 分钟） | 642 MB |
| **OmniBundle** | **0.21 s** | **67 MB** |

同一台机器、同一批 fixture、3 次取中位数。这里每个数字都能用 `bench/` 里的
harness 复现——协议见 [基准测试](docs/en/04-benchmark-plan.md)，完整记录（含我们**没达标**的
目标）见 [证据日志](docs/en/01-evidence.md)。

## 它做什么

```bash
npx omnibundle ./dist
```

```
dist  ·  154379 modules  ·  1500 assets  ·  400 packages  ·  ingest 1392 ms  ·  total 1753 ms  ·  dimension parsed
wrote dist/report.html (1.6 MB), detail in a companion script (loaded on demand)
```

- **一张图，所有打包器。** webpack / rspack 的 `stats.json` 提供依赖结构，source map 提供
  真实字节归因，esbuild metafile 提供模块图。它们被**合并**，而不是事后拼接。
- **四种尺寸维度，并且会告诉你用的是哪一种。** `stat`（打包器声明的）、`parsed`（磁盘上的真实
  字节）、`gzip`（level 6）、`attributed`（source map 真正能解释的字节）。如果 map 覆盖不全，
  `attributed` 会降级为 `parsed` 并且 CLI 会说明原因——一份声称自己给出 ground truth 却没有的
  报告，比没有报告更糟。
- **能真的让 CI 失败的预算。** `omnibundle.config.json` 可以限制总字节数、某个 chunk 或某个
  package，每个规则都能指定维度。超限退出码为 1 并给出 `OB0040`；**匹配不到任何对象的规则视为
  错误**，绝不会静默通过。
- **JSON 和 CSV 输出**，方便接入其余流水线。

![OmniBundle 报告，按 package 分组](docs/assets/treemap-large.svg)

<sub>按 package 分组，跨 1,500 个 asset 汇总。合成 fixture，8,021 个模块，400 个 package——
这正是大型 monorepo 构建的形状。图上的标签是工具自己的输出；图片由
`bench/harness/render-treemap-svg.mjs` 生成，不是手工画的。</sub>

## 幽灵代码与隐藏代码

这是两个参考工具都告诉不了你的东西，也是这个项目真正新的部分。

- **幽灵代码（ghost code）**——打包器声明并打包了，但**没有任何 source map 解释得了**的模块。
  要么是 tree-shaking 漏掉了，要么那个 asset 根本没带 map。它在你的产物里，却不在任何人的体积报告里。
- **隐藏代码（hidden code）**——生成的字节**不属于任何模块**：内联的代码片段、`eval`、打包器注入的
  polyfill。它在你的产物里，却不属于任何模块的体积。

OmniBundle 把每个 source 的字节份额折算回产生它的模块，剩下无法对账的部分变成诊断信息，而
不是被四舍五入掉：

```
fusion: 1 map(s) · coverage 100% · 50000/50000 modules attributed · 0 ghost · 0 hidden source(s)
```

归因依据是真实 join key 上的最长路径后缀匹配（见 `docs/contracts/unified-graph.md`），
**从不使用内容哈希**；修正后的尺寸之和会与 asset 总大小做不变量校验，不通过就大声报错
（`OB0042`），而不是悄悄取整。

## 安装

```bash
# npm —— 下载经过校验的预编译二进制，不需要 Rust 工具链
npx omnibundle ./dist

# cargo
cargo install omnibundle-cli

# 或直接下载 release 二进制：linux x64/arm64、macOS x64/arm64、windows x64
# https://github.com/omnibundle/omnibundle/releases
```

npm 包会在写入或执行任何东西之前校验 `checksums.txt`，哈希不匹配就拒绝安装。

## 使用

```bash
omnibundle ./dist                      # 目录：stats + assets + *.map
omnibundle ./dist/stats.json           # 单个 stats 文件
omnibundle ./dist/metafile.json        # esbuild metafile

omnibundle ./dist --budget omnibundle.config.json   # 超限退出码 1
omnibundle ./dist --mode json > sizes.json          # 给 CI 或 BI 用
omnibundle ./dist --mode csv  > sizes.csv
omnibundle ./dist/map.js.map --bench-map            # 只跑归因，并计时
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
<summary>退出码——属于契约的一部分，因为 CI 门禁就是建立在它上面的</summary>

| 退出码 | 含义 |
|---|---|
| 0 | 分析完成，且所有预算都通过 |
| 1 | 分析完成，但有预算被突破（`OB0040`） |
| 2 | 命令行参数错误 |
| 3 | 输入不可读，或某条预算规则匹配不到任何对象 |

</details>

## 当前状态

1.0 之前，下面这张表是诚实的版本。没测量的东西就写没测量。

| 领域 | 状态 |
|---|---|
| stats 摄取、尺寸归因、source map、融合、报告、预算、JSON/CSV | 已实现、已测量、CI 门禁 |
| 1 GB 摄取的墙钟时间 | **未达标**：4.40 s，目标 3 s（内存 350 MB，很宽裕） |
| 报告首屏渲染 / 30 fps | **未验证**——CI 里没有浏览器；改为测量 154,379 模块下 1.56 MB、1.27 s |
| WASM 构建、WebGL 渲染器 | 尚未开始 |
| Windows / macOS / Linux | CI 三平台均测试 |

唯一那个未达标项，在 [CHANGELOG](CHANGELOG.md) 里写明了原因：瓶颈是 `serde_json` 的 DOM 游标
在 445,602 个元素的模块数组上。

## 为什么这样做

每一个设计决策都来自一次测量，而不是偏好。

| 决策 | 依据 |
|---|---|
| 流式 JSON，不用 `simd-json` | `simd-json` 需要把整个文档读进内存——而那正是我们要拆掉的天花板（[ADR-0001](docs/decisions/ADR-0001-streaming-over-simd-json.md)） |
| 自研 HTML 报告，不 vendor 别人的 viewer | 不受我们控制的 viewer 无法展示融合数据，除非 fork；fork 之后这份 UI 就归我们维护了（[ADR-0002](docs/decisions/ADR-0002-self-built-report.md)） |
| 尺寸与 gzip 用 rayon 并行 | 25,600 个 asset：串行 2,177 ms → **342 ms**（6.4 倍）；解析从来不是瓶颈 |
| 计数精确、列表有界 | 每个未映射模块存一条 `GhostModule`，为一个文档里写着"summary"的字段花掉 47 MB；现在计数精确，列表是有界样本并说明丢弃了多少 |
| 不能失败的基准测试没有意义 | B3 之所以量到 670 MB（目标 200 MB），是因为 detail payload 建成了 `serde_json::Value` 树。目标起作用了。 |

### 我们没有做什么，以及为什么

| 放弃的方案 | 原因 |
|---|---|
| vendor WBA 的 viewer | 不 fork 就无法展示融合数据；fork 之后这份 UI 就归我们维护了 |
| `simd-json` | 需要把文档读进内存，而这正是问题本身 |
| Phase 1 用 WebGL treemap | 10k 个节点 Canvas 2D 足够；渲染器替换是 Phase 2 的事，放在稳定 payload 之后 |
| 通用的构建工具 | 实测的痛点是内存天花板、逐项进程，或超线性算法。这三条这个项目都占。 |

## 文档

- [产品需求](docs/en/00-prd.md) · [证据日志](docs/en/01-evidence.md) · [架构](docs/en/02-architecture.md)
- [基准与目标](docs/en/04-benchmark-plan.md) · [一致性与测试](docs/en/05-parity-and-testing.md) · [发布与 CI](docs/en/06-release-and-ci.md)
- [契约](docs/contracts/)——payload schema、CLI 接口、基准协议、i18n 规则、归属
- [ADR](docs/decisions/) · [风险清单](docs/en/07-risk-register.md) · [路线图](docs/en/08-roadmap.md)

> 完整的中文文档正在翻译中。英文文档是**规范源**；`node bench/harness/check-i18n.mjs`
> 会检查四种语言是否漂移。

## 参与贡献

每一次改动都要么附带一次测量，要么给出"为什么这次不需要测量"的论证。详见
[CONTRIBUTING.md](CONTRIBUTING.md)；门禁是 `cargo test`、`clippy -D warnings`（含 pedantic）、
`cargo fmt`、与 `webpack-bundle-analyzer` 的一致性比对，以及四语文档检查。
[行为准则](CODE_OF_CONDUCT.md) · [安全策略](SECURITY.md)

## 许可证

MIT（[LICENSE-MIT](LICENSE-MIT)）或 Apache-2.0（[LICENSE-APACHE](LICENSE-APACHE)），由你选择。

---

<div align="center">
  <sub>
    公开构建中。欢迎在
    <a href="https://github.com/omnibundle/omnibundle/issues">issues</a>
    里提出数字上的质疑、异议和纠正。
  </sub>
</div>