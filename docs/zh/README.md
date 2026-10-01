# 文档索引 / Documentation index (ZH)

英文版是**规范性来源**（normative source）；本目录为翻译与状态记录。
规则见 `docs/contracts/i18n-parity.md`。

| 文件 | 英文源 | 状态 |
|---|---|---|
| README | [README.zh.md](../../README.zh.md) | ✅ 已发布级 |
| GLOSSARY | [GLOSSARY.md](GLOSSARY.md) | ✅ 已发布级 |
| 00 PRD 产品需求 | [en/00-prd.md](../en/00-prd.md) | ⏳ 翻译队列（发布级） |
| 01 证据台账 | [en/01-evidence.md](../en/01-evidence.md) | ⏳ 翻译队列 |
| 02 架构 | [en/02-architecture.md](../en/02-architecture.md) | ⏳ 翻译队列 |
| 03 实现计划 | [en/03-implementation-plan.md](../en/03-implementation-plan.md) | ⏳ 翻译队列 |
| 04 基准测试 | [en/04-benchmark-plan.md](../en/04-benchmark-plan.md) | ⏳ 翻译队列 |
| 05 等价性与测试 | [en/05-parity-and-testing.md](../en/05-parity-and-testing.md) | ⏳ 翻译队列 |
| 06 发布与 CI | [en/06-release-and-ci.md](../en/06-release-and-ci.md) | ⏳ 翻译队列 |
| 07 风险登记 | [en/07-risk-register.md](../en/07-risk-register.md) | ⏳ 翻译队列 |
| 08 路线图 | [en/08-roadmap.md](../en/08-roadmap.md) | ⏳ 翻译队列 |
| 契约 / ADR | [contracts](../contracts/) · [decisions](../decisions/) | 🔒 仅英文（规范性） |

## 翻译规则（译者必读）

1. 等待英文源文件被标记为 frozen，再开始翻译；
2. **数字、命令、路径、代码标识符、退出码必须逐字保留**（例如 `2.78 s`、
   `176.3 s`、`FS0040`、`--dims`、`check_size_invariant`）；
3. 文件头写入 `<!-- source: docs/en/<file> | version: <n> | status: translated -->`；
4. 运行 `node bench/harness/check-i18n.mjs --lang zh` 直到零差异；
5. 不要在翻译中"改进"英文的技术表述——如果英文有问题，去给英文源提 issue。

## 术语

见 [GLOSSARY.md](GLOSSARY.md)。表格中的"工作流 / lane"两词在中文语境下统一
用"工作流"。
