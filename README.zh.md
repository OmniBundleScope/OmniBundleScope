# OmniBundle

**一款通吃所有打包器的工具。** 分析构建产物——`stats.json`、Source Map、
esbuild metafile，或者直接给一个 `dist/` 目录——在低得多的内存占用下，给出
webpack、rspack、Vite、Rollup、esbuild 统一的体积视图。

[English](README.en.md) · [中文](README.zh.md) · [日本語](README.ja.md) · [Deutsch](README.de.md)

状态：**Phase 1 进行中。** CLI 接口与数据契约已冻结，摄取、融合、报告引擎正
在按契约实现。

## 将要提供的能力

```bash
npx omnibundle ./dist
# → omnibundle-report.html   单文件报告，不发起任何网络请求
# → 退出码 0；体积预算超限时退出码 1 并输出 OB0040
```

- **一张图，通吃所有打包器。** webpack/rspack 的 `stats.json` 提供依赖结构，
  Source Map 提供真实的字节归因，esbuild metafile 提供模块图；OmniBundle
  把它们融合成一张统一图。
- **两项参照工具给不出的诊断。** *幽灵代码*：打包器声明了却没有任何映射能
  解释的字节；*隐藏代码*：无法映射回任何模块的生成字节（内联片段、`eval`、
  注入的 polyfill）。
- **内存装得下。** 实测 1,049 MB 的 `stats.json`：**2.78 秒 / 59 MB RSS**；
  同一输入下 `webpack-bundle-analyzer` 为 **176.3 秒 / 1,437 MB**。
- **体积卡点让 CI 失败。** `omnibundle.config.json` 可按 chunk、按包或总量
  设限，超限即非零退出。

## 为什么这样设计

每一个决定都来自实测，而不是偏好——详见[证据台账](docs/en/01-evidence.md)与
[ADR](docs/decisions/)。所有性能结论都按阶段分别陈述；尚未测量的部分一律标记
`unverified`，不做估算。

## 文档

- [产品需求与逐条验证标注](docs/en/00-prd.md)
- [证据台账](docs/en/01-evidence.md) · [架构](docs/en/02-architecture.md)
- [实现计划](docs/en/03-implementation-plan.md) · [基准测试](docs/en/04-benchmark-plan.md)
- [契约](docs/contracts/) · [ADR](docs/decisions/) · [风险登记](docs/en/07-risk-register.md)

## 许可证

MIT OR Apache-2.0。
