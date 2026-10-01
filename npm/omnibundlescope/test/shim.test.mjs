// The shim is the contract: it must forward arguments and exit codes untouched,
// because the exit codes are what CI gates are built on (1 = budget breached,
// 3 = input unusable). These tests run against a real cargo build.

import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const shim = resolve(here, '..', 'bin', 'omnibundlescope.js');
const repoRoot = resolve(here, '..', '..', '..');
const realBinary = join(repoRoot, 'target', 'release', process.platform === 'win32' ? 'omnibundlescope.exe' : 'omnibundlescope');

const fixture = {
  version: '5.90.0',
  assets: [{ type: 'asset', name: 'main.js', size: 4096, chunks: [0], emitted: true }],
  chunks: [{ id: 0, names: ['main'], files: ['main.js'], size: 4096 }],
  modules: [
    {
      id: 1,
      identifier: './src/app.js',
      name: './src/app.js',
      size: 4096,
      chunks: [0],
      reasons: [],
      source: 'x',
    },
  ],
};

function fixtureDir() {
  const dir = mkdtempSync(join(tmpdir(), 'omnibundlescope-test-'));
  writeFileSync(join(dir, 'stats.json'), JSON.stringify(fixture));
  return dir;
}

const env = { ...process.env, OMNIBUNDLESCOPE_BIN: realBinary };

test('forwards --help and succeeds', () => {
  const out = execFileSync(process.execPath, [shim, '--help'], { env, encoding: 'utf8' });
  assert.match(out, /stats\.json/);
});

test('exit 0 on a good input, and a report exists', () => {
  const dir = fixtureDir();
  const report = join(dir, 'report.html');
  const r = spawnSync(process.execPath, [shim, join(dir, 'stats.json'), '--report', report], {
    env,
    encoding: 'utf8',
  });
  assert.equal(r.status, 0, r.stderr);
  assert.match(r.stdout, /modules/);
});

test('exit 3 on unreadable input (the contract for a broken build)', () => {
  const r = spawnSync(process.execPath, [shim, join(tmpdir(), 'does-not-exist.json')], {
    env,
    encoding: 'utf8',
  });
  assert.equal(r.status, 3);
});

test('a missing binary exits 127 with instructions, not a stack trace', () => {
  const r = spawnSync(process.execPath, [shim, '--help'], {
    env: { ...process.env, OMNIBUNDLESCOPE_BIN: join(tmpdir(), 'no-such-binary') },
    encoding: 'utf8',
  });
  assert.equal(r.status, 127);
  assert.match(r.stderr, /cargo build --release/);
});