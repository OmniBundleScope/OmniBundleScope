# ADR-0004: Naming and distribution shape

Status: **superseded** (2026-10-01) by ADR-0006 · Owner: maintainers · Affects: the CLI, distribution and CI, docs and translations

> Superseded on the naming decision only. The distribution shape below - one binary,
> three publication channels, and the order they go out in - is still current.
> The name itself is `omnibundlescope`; see [ADR-0006](ADR-0006-renaming.md) for why,
> including why this record defended the right word for the wrong question.

## Decision

| artefact | name |
|---|---|
| workspace | `omnibundlescope` |
| library crate | `omnibundlescope-core` |
| CLI crate / binary | `omnibundlescope-cli` / **`omnibundlescope`** |
| WASM crate | `omnibundlescope-wasm` |
| npm package | `omnibundlescope` |
| report file default | `omnibundlescope-report.html` |
| config file | `omnibundlescope.config.json` |
| cache dir | `.omnibundlescope-cache/` |
| schema URLs | `{{DOCS_URL}}/schemas/…` |

Availability was checked before deciding: `omnibundlescope` returned 404 on both the
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
- `omnibundlescope` is pronounceable in English, ZH, JA and DE without
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
