# Fetch the real fixtures (WS-A). Pinned by commit, never committed themselves
# (ADR-0005). Run from the bench/ directory:  .\fixtures\fetch.ps1
$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$repos = Join-Path $here 'repos'
New-Item -ItemType Directory -Force -Path $repos | Out-Null

$fixtures = @(
    @{ name = 'preact';   repo = 'https://github.com/preactjs/preact';     ref = '10.29.8' },
    @{ name = 'marked';   repo = 'https://github.com/markedjs/marked';     ref = '18.0.14' },
    @{ name = 'chalk';    repo = 'https://github.com/chalk/chalk';          ref = '6.0.1' },
    @{ name = 'dayjs';    repo = 'https://github.com/iamkun/dayjs';        ref = '1.11.23' },
    @{ name = 'p-limit';  repo = 'https://github.com/sindresorhus/p-limit'; ref = 'latest' },
    @{ name = 'nanoid';   repo = 'https://github.com/ai/nanoid';            ref = 'latest' },
    @{ name = 'ms';       repo = 'https://github.com/sindresorhus/ms';      ref = 'latest' },
    @{ name = 'mitt';     repo = 'https://github.com/developit/mitt';       ref = 'latest' },
    @{ name = 'ufo';      repo = 'https://github.com/unjs/ufo';            ref = 'latest' },
    @{ name = 'h3';       repo = 'https://github.com/unjs/h3';              ref = 'latest' }
)

foreach ($f in $fixtures) {
    $dest = Join-Path $repos $f.name
    if (Test-Path (Join-Path $dest '.git')) {
        Write-Output "== $($f.name): already present, skipping"
        continue
    }
    Write-Output "== $($f.name): cloning ($($f.ref))"
    git clone --depth 1 --quiet $f.repo $dest
    Push-Location $dest
    try {
        if ($f.ref -ne 'latest') {
            git fetch --depth 1 --quiet origin "refs/tags/v$($f.ref):refs/tags/v$($f.ref)" 2>$null
            git checkout --quiet "v$($f.ref)" 2>$null
        }
        $commit = git rev-parse HEAD
        Write-Output "   commit: $commit"
        $commit | Out-File -FilePath (Join-Path $dest '.omnibundle-commit') -Encoding utf8
    } finally {
        Pop-Location
    }
}

Write-Output ''
Write-Output 'Next (per fixture, in fixtures/build/):'
Write-Output '  - build the project so the artifacts exist (npm ci; npm run build)'
Write-Output '  - for webpack inputs use fixtures/build/webpack.preact.mjs'
Write-Output '  - record artifact paths, sizes and sha256 in fixtures/manifest.json'
Write-Output ''
Write-Output 'Real fixtures are for correctness and parity; the scale numbers in'
Write-Output '01-evidence.md come from the synthetic generators in harness/.'
