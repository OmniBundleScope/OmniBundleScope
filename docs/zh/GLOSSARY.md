# 术语表 — OmniBundle（ZH）

规范列表见 `docs/contracts/i18n-parity.md` §4。其他语言：
[EN](../en/GLOSSARY.md) · [JA](../ja/GLOSSARY.md) · [DE](../de/GLOSSARY.md)

| EN | ZH | 说明 |
|---|---|---|
| asset | 构建产物 | 构建输出的文件，treemap 的根节点 |
| chunk | 分块 | webpack 的异步加载单元，含 `initial` 标记 |
| module | 模块 | 依赖图中的一个条目 |
| source map | Source Map | v3；字节归因的基准真值 |
| attribution | 归因 | 把生成字节映射回原始源文件 |
| fusion | 融合 | 把 stats 图与一个或多个 Source Map 合并 |
| ghost code | 幽灵代码 | 打包器声明了、却没有任何映射能解释的字节 |
| hidden code | 隐藏代码 | 无法映射回任何模块的生成字节 |
| tree shaking | Tree Shaking | 消除未使用的导出 |
| treemap | 矩阵树图 | squarified 布局，Phase 1 用 Canvas 2D 绘制 |
| budget | 体积卡点 | CI 门禁；超限退出码 1 并输出 `OB0040` |
| baseline | 基线 | 用来对比的参照数值 |
| size dimension | 体积维度 | `stat` / `parsed` / `gzip` / `attributed` |
| phase | 阶段 | scan / parse / attribute / fuse / report |
| lane / workstream | 工作流 | 拥有独占路径的职责单元（WS-0 … WS-9） |
| fixture | 固定样本 | 测试或基准使用的固定输入 |
| real fixture | 真实样本 | 由真实开源项目构建而来 |
| synthetic fixture | 合成样本 | 为达到真实项目不具备的规模而生成 |
| parity | 等价性 | 与参照工具在容差内一致 |
| unverified | 未验证 | 未测量目标的合法状态 |
| streaming ingest | 流式摄取 | 不把整个文档读进内存的解析方式 |
| ceiling | 上限 | 让工具不可用的内存/体积上限 |

## 不翻译的内容

代码、标识符、文件名、CLI 参数、JSON 键、诊断码（`OB0001` …）、crate 名称、
版本号。翻译其中任何一项都会让一致性检查失败。
