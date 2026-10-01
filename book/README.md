# FastScope documentation

The site is built from this directory, with English as the normative source.
There is exactly one copy of every document: the chapters in
[src/SUMMARY.md](src/SUMMARY.md) point at the files that live here, so a
translation is a translation of a real file rather than a fork of the site.

```bash
# requires mdbook: cargo install mdbook
cd book && mdbook serve      # http://localhost:3000
mdbook build                 # book/book/
```

## Layout

| path | holds | rule |
|---|---|---|
| `en/` | the normative documents | edits land here first |
| `zh/`, `ja/`, `de/` | translations | same file set, same headings, same numbers — checked by `bench/harness/check-i18n.mjs` |
| `contracts/` | payload schema, CLI surface, benchmark protocol, i18n rules, ownership | changes need an ADR |
| `decisions/` | ADRs | append-only; supersede rather than edit |
| `assets/` | images used by the docs, generated from real data | regenerate, never hand-edit |

## Why the site is generated rather than written separately

A documentation site that duplicates the repository's documents has two
answers to every question within a release, and only one of them gets fixed.
The alternative - the site being a view over the repo - means the README, the
ADRs and the site cannot disagree.

## Adding a document

1. Write it in `en/`.
2. Add a chapter to `book/src/SUMMARY.md`, in the reading order someone would
   want it rather than the order it happened to be written.
3. If it changes a contract, update `contracts/` and add an ADR.
4. Translations come from `check-i18n.mjs` reporting them `pending`; the EN
   revision is tracked in `REVISION` files per language.
