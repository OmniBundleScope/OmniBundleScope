#!/usr/bin/env bash
# Fetch the real fixtures (benchmarks). Pinned by commit, never committed themselves
# (ADR-0005). Run from the bench/ directory:  ./fixtures/fetch.sh
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repos="$here/repos"
mkdir -p "$repos"

# name  repo  ref
fixtures=(
  "preact        https://github.com/preactjs/preact        10.29.8"
  "marked        https://github.com/markedjs/marked        18.0.14"
  "chalk         https://github.com/chalk/chalk             6.0.1"
  "dayjs         https://github.com/iamkun/dayjs           1.11.23"
  "p-limit       https://github.com/sindresorhus/p-limit   latest"
  "nanoid        https://github.com/ai/nanoid              latest"
  "ms            https://github.com/sindresorhus/ms        latest"
  "mitt          https://github.com/developit/mitt         latest"
  "ufo           https://github.com/unjs/ufo              latest"
  "h3            https://github.com/unjs/h3                latest"
)

for entry in "${fixtures[@]}"; do
  read -r name repo ref <<<"$entry"
  dest="$repos/$name"
  if [ -d "$dest/.git" ]; then
    echo "== $name: already present, skipping"
    continue
  fi
  echo "== $name: cloning ($ref)"
  git clone --depth 1 --quiet "$repo" "$dest"
  ( cd "$dest" && git fetch --depth 1 --quiet origin "refs/tags/v$ref:refs/tags/v$ref" 2>/dev/null || true
    git checkout --quiet "v$ref" 2>/dev/null || true
    echo "   commit: $(git rev-parse HEAD)"
    git rev-parse HEAD > "$dest/.omnibundlescope-commit" )
done

cat <<'EOF'

Next (per fixture, in fixtures/build/):
  - build the project so the artifacts exist (npm ci && npm run build)
  - for webpack inputs use fixtures/build/webpack.preact.mjs
  - record artifact paths, sizes and sha256 in fixtures/manifest.json

Real fixtures are for correctness and parity; the scale numbers in
01-evidence.md come from the synthetic generators in harness/.
EOF
