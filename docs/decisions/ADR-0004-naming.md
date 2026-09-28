# ADR-0004: Naming and distribution shape

Status: **accepted** (2026-09-02) · Owner: WS-0 · Affects: WS-6, WS-8, WS-9

## Decision

| artefact | name |
|---|---|
| workspace | `omnibundle` |
| library crate | `omnibundle-core` |
| CLI crate / binary | `omnibundle-cli` / **`omnibundle`** |
| WASM crate | `omnibundle-wasm` |
| npm package | `omnibundle` |
| report file default | `omnibundle-report.html` |
| config file | `omnibundle.config.json` |
| cache dir | `.omnibundle-cache/` |
| schema URLs | `{{DOCS_URL}}/schemas/…` |

Availability was checked before deciding: `omnibundle` returned 404 on both the
npm registry and crates.io on 2026-09-02, i.e. unclaimed in both. It must be
re-checked immediately before publishing, since a squatted name is only a
discovery away.

## Reasoning

- `bundle` in the name is the honest part: the input is bundler output. Dropping
  it (something like "omnisize" or "weightwatch") would hide what the tool
  actually does, and the PRD's own position — "one tool to rule them all" — is
  a positioning statement we can live up to with a plain descriptive name.
- One binary, one name, three distribution shapes (cargo, static binary, npm
  wrapper). The npm package is a thin wrapper: it downloads the platform
  binary and execs it, because shipping a native binary inside an npm tarball
  is how you end up with a 200 MB install.
- `omnibundle` is pronounceable in English, ZH, JA and DE without
  transliteration games, which matters for the four-language documentation
  requirement.

## Consequences

- Registry publication order: crates.io first (proves the build is real), then
  the static binary release, then the npm wrapper pointing at the GitHub
  release. Publishing the npm name first would burn the name on an empty
  package.
- The repo is its own git repository, separate from the surrounding workspace,
  because the direction-search documents that led here live elsewhere and should
  not be coupled to this project's release history.
- Tagline ("One tool to rule them all") is marketing copy for the README only.
  It is not in the binary's help text and not in the docs' technical sections.
