---
name: Bug report
about: Something is wrong, or a number looks wrong
title: ''
labels: bug
assignees: ''
---

**What happened**

<!-- The command you ran and what came out. -->

**What you expected**

**Reproduction**

The smallest input that shows it. A synthetic fixture is fine and often better
than a customer's build:

```bash
cd bench
node harness/gen-stats.mjs 5000000 /tmp/small.json
../target/release/omnibundle /tmp/small.json --report /tmp/report.html
```

If the bug is about size numbers, please state which dimension you expected and
why — `parsed`, `gzip` and `attributed` legitimately disagree, and the tool
prints which one it used.

| field | value |
|---|---|
| OmniBundle version | `omnibundle --version` |
| input class | real project / synthetic |
| bundler | webpack / rspack / vite / rollup / esbuild / other |
| OS | |
| Rust version | `rustc --version` |

**Anything else**

<!-- A link to the generated report is rarely useful: it embeds your whole
     module graph. Share the numbers, not the file. -->