// Resolve the OmniBundle binary for this platform, preferring a local build.
//
// Why a shim instead of a JS implementation: the whole point of the tool is
// that a 1 GB stats file is parsed in a few seconds inside a fixed memory
// budget, which is a Rust claim. Re-implementing the hot path in JavaScript
// would make the npm package a different program with different numbers.
//
// Resolution order:
//   1. $OMNIBUNDLE_BIN         — explicit override, used by the test suite
//   2. ../target/release/omnibundle — a local cargo build (the dev path)
//   3. the vendored binary next to this file, installed by install.mjs
//   4. a clear error that says how to fix it
//
// On failure the error lists the exact commands. A package that prints
// "command not found" after installing successfully is worse than one that
// refuses to install.

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');

const exe = process.platform === 'win32' ? 'omnibundle.exe' : 'omnibundle';

const candidates = [
  process.env.OMNIBUNDLE_BIN,
  join(here, '..', 'vendor', `${process.platform}-${process.arch}`, exe),
  join(repoRoot, 'target', 'release', exe),
  join(repoRoot, 'target', 'debug', exe),
].filter(Boolean);

const found = candidates.find((p) => existsSync(p));

if (!found) {
  const build =
    process.platform === 'win32'
      ? 'cargo build --release --manifest-path crates/omnibundle-cli/Cargo.toml'
      : 'cargo build --release --manifest-path crates/omnibundle-cli/Cargo.toml';
  process.stderr.write(
    [
      '',
      'omnibundle: no binary found for this platform.',
      '',
      `  platform: ${process.platform}-${process.arch}`,
      `  looked in: ${candidates.join('\n            ')}`,
      '',
      '  Fix it either way:',
      `    - install the release binary:  node install.mjs`,
      `    - or build from source:        ${build}`,
      `    - or point at an existing one: OMNIBUNDLE_BIN=/path/to/omnibundle`,
      '',
    ].join('\n'),
  );
  process.exit(127);
}

// Forward argv and the exit code untouched: the CLI's exit codes are part of the
// contract (1 = budget breached, 3 = input unusable), and a wrapper that
// swallowed them would break every CI gate built on it.
const result = spawnSync(found, process.argv.slice(2), { stdio: 'inherit' });

if (result.error) {
  process.stderr.write(`omnibundle: failed to run ${found}: ${result.error.message}\n`);
  process.exit(127);
}
// A signal death is reported by Node as a null code; surface it as a failure
// rather than as success.
process.exit(result.status ?? 1);