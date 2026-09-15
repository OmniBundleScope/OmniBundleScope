// Full-pipeline benchmark (B3, B4, B8) and B10's input side.
//
//   node bench-pipeline.mjs --stats <stats.json> [--dir <bundle dir>] [--maps]
//
// Measures the phases the shipped binary actually reports, and records what it
// did *not* measure as `null` rather than guessing (bench-spec.md §2).
//
// Note on the folder case: with `--dir`, the run also measures asset sizes and
// (with `--maps`) fuses source maps, which is the "full pipeline" of B3/B4/B8.
// Without a bundle directory there is nothing to measure on disk, so only the
// ingest phase runs — the report says so.

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..', '..');
const binary = process.env.OB_BINARY ?? join(repoRoot, 'target', 'release', 'omnibundle.exe');
const resultsDir = join(repoRoot, 'bench', 'results');

const arg = (flag, fallback = null) => {
  const i = process.argv.indexOf(flag);
  return i > -1 ? process.argv[i + 1] : fallback;
};
const has = (flag) => process.argv.includes(flag);

const stats = arg('--stats');
const dir = arg('--dir');
const withMaps = has('--maps');
const runs = Number(arg('--runs', '3'));

if (!stats || !existsSync(binary)) {
  console.error('usage: node bench-pipeline.mjs --stats <stats.json> [--dir <dir>] [--maps] [--runs 3]');
  process.exit(2);
}

function runOnce() {
  const args = [dir ?? stats, '--mode', 'static', '--report', resultsDir + '/bench-report.html'];
  if (dir) {
    // the folder path is what triggers on-disk measurement + fusion
  } else {
    args[0] = stats;
    args.push('--bench');
  }
  const t0 = Date.now();
  const r = spawnSync(binary, args, { encoding: 'utf8' });
  return { wall_ms: Date.now() - t0, stdout: r.stdout ?? '', status: r.status };
}

const samples = [];
for (let i = 0; i < runs; i++) {
  const s = runOnce();
  samples.push(s.wall_ms);
  if (i === 0) {
    const line = s.stdout
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean)
      .find((l) => l.startsWith('{') || l.includes('modules')) ?? s.stdout;
    console.log(line);
  }
}
samples.sort((a, b) => a - b);
const median = samples[Math.floor(samples.length / 2)];

const out = {
  measurement: 'WS-2 full pipeline',
  input: { stats, dir: dir ?? null, maps_fused: withMaps, bytes: statSync(stats).size },
  runs: samples,
  wall_ms_median: median,
  phase_breakdown:
    dir === null
      ? { scan: null, parse: 'reported in the CLI line', attribute: null, fuse: null, report: null }
      : { note: 'the CLI reports ingest and total; attribute/fuse/report are inside the total' },
  targets: {
    B3: dir === null ? 'not applicable (no bundle directory: ingest only)' : '<= 5000 ms, <= 200 MB at 381 MB',
    B4: dir === null ? 'not applicable' : '<= 15000 ms, <= 400 MB at 1 GB',
    B8: withMaps ? '< 500 MB peak on 1 GB stats + 50 MB map' : 'maps not included in this run',
  },
  honest_gaps: [
    dir === null ? 'no bundle directory, so asset measurement and fusion did not run' : null,
    'peak RSS is not sampled by this script; use harness-memory.sh for that',
  ].filter(Boolean),
};

mkdirSync(resultsDir, { recursive: true });
const file = join(resultsDir, `ws2-pipeline-${Date.now()}.json`);
writeFileSync(file, `${JSON.stringify(out, null, 2)}\n`);
console.log(`\nmedian ${median} ms — written to ${file}`);
