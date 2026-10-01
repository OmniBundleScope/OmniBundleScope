# ADR-0006: Renaming to `fastscope`

Status: **accepted** (2026-10-01) · Owner: WS-0 · Supersedes: ADR-0004 (naming) · Affects: WS-6, WS-8, WS-9

## Context

ADR-0004 chose `omnibundle` and defended the word *bundle* on the grounds that
"the input is bundler output". That argument is true and it was the wrong
question. It never asked what a reader infers from a bare noun ending in
"Bundle", and the answer was: *this is a bundler*.

A tool called `omnibundle` that has never bundled anything is a promise it
cannot keep. The failure is not that the name is inaccurate - bundle analysis
*is* what it does, and `bundlesize` and `bundlewatch` both put "bundle" in the
name of an analyzer. The failure is that `OmniBundle` is a bare noun: it names
the artifact rather than the action, so the first misread is "another bundler",
and the first thing a reader tries is `omnibundle build`.

The tagline compounded it. "One tool for every bundler" parses two ways, and the
wrong one is the one people read.

## Options considered

| option | verdict |
|---|---|
| keep `omnibundle`, fix the messaging | cheapest, and the self-descriptions were already correct: the CLI help says "Analyse bundler output", the npm description says the same, and the keywords already include `bundle-analyzer` |
| `bundlescope` | subject plus action; the strongest name, and the one recommended |
| `omnisize` | names the output, not the input; rejected by ADR-0004 for hiding what the tool consumes |
| `bytescope` | **taken on npm** (a telecom utility, published 2026-02) |
| `sourcemeter` | **taken on npm** (a dead Source-dedicated-server package) |
| `fastscope` | **chosen.** Two words, so the name reads as a tool rather than as the artifact, which removes the "is it a bundler?" question entirely |

`bundlescope` was the recommendation and `fastscope` was the decision. Recorded
here rather than quietly implemented, because the reason for the choice is not
the one the recommendation argued for: `fastscope` sells speed, which is the
least durable of the tool's three true claims, over byte attribution, which is
the one neither reference tool does. Speed is at least defensible as a
positioning - 36x faster than `webpack-bundle-analyzer` on the stats fixture and
2,690x faster than `source-map-explorer` at 50k sources are the numbers the
README leads with - and a memorable name is worth something at launch that a
durable one is not.

## Decision

| artefact | name |
|---|---|
| workspace | `fastscope` |
| library crate | `fastscope-core` |
| CLI crate / binary | `fastscope-cli` / **`fastscope`** |
| WASM crate | `fastscope-wasm` |
| npm package | `fastscope` |
| report file default | `fastscope-report.html` |
| config file | `fastscope.config.json` |
| cache dir | `.fastscope-cache/` |
| env vars | `FASTSCOPE_BIN`, `FASTSCOPE_VERSION`, `FASTSCOPE_REPO` |
| diagnostic codes | `FS00xx` (was `OB00xx`) |
| schema URLs | `{{DOCS_URL}}/schemas/…` |

Everything else in ADR-0004 stands: publication order is unchanged (crates.io,
then the static binary, then the npm wrapper), and the name must be re-checked
on both registries immediately before publishing. `fastscope` was free on npm and
crates.io when this was written, which is a snapshot and not a reservation.

## Consequences

- The diagnostic code prefix changes from `OB` to `FS`. They are a published
  contract in `docs/contracts/unified-graph.md`, so this is a breaking change
  for anything parsing them - which is nothing yet, since 0.1.0 has not shipped.
  Leaving `OB` behind would have been the cheaper option and the wrong one: a
  project called fastscope that emits `OB0051` reads as a rename that stopped
  halfway.
- The git history before this commit says `OmniBundle`, and stays that way. It
  happened under that name; rewriting it would make the record less true, not
  more.
- ADR-0004's fallback-name clause is superseded. The fallback is now
  `fastscope-cli` for a squatted binary name, and it must be decided in the ADR
  that is active at the time rather than quoted forward from a superseded one -
  the risk register had been quoting it from a document that never contained it.
- The tagline is now "One analyzer for every bundler." The word *analyzer* is
  load-bearing: it is the one word that answers "is this a bundler?" in the
  first three words of the README.
