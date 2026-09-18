---
name: Benchmark discrepancy
about: A published number does not reproduce, or two tools disagree
title: ''
labels: benchmark
assignees: ''
---

Disagreements are treated as bugs in the tool until proven otherwise, so this
is a first-class report, not a nitpick.

**The number in question**

<!-- Which target (B1-B10), which tool, which figure. -->

**What you measured instead**

| | |
|---|---|
| command | |
| input (bytes / modules / sources) | |
| fixture class | real / synthetic |
| wall time (median of N) | |
| peak memory, and how you measured it | |
| OS + CPU + core count | |
| Rust or Node version | |

**How memory was measured matters**

Working set and private bytes are different numbers for the same run — working
set includes file-backed pages, which for a 1 GB stats stream inflates with the
page cache. `bench/harness/measure.ps1` records both. If you used a different
method, say which, so the two can be compared instead of argued about.

**Reproduction command**

```bash
```