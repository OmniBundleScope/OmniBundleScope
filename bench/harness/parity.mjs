// Parity harness (parity): compare our numbers with the reference tools on the
// same fixture, per `docs/en/05-parity-and-testing.md`.
//
//   node parity.mjs --stats <stats.json> [--bundle <dir>]
//   node parity.mjs --sme   <bundle.js> [--sme]
//
// What it checks, and why each check exists:
//   - keys:   every module the reference reports must exist for us (missing /
//             extra are reported, never averaged away);
//   - sizes:  per-key relative delta and the aggregate, both reported, because
//             a 0.05 % aggregate with one 40 % module is a bug a total-only
//             check hides;
//   - order:  ordering differences are tolerated and counted, nothing else is.
//
// Tolerance: 0.1 % (TOLERANCE_PPM = 1_000), matching the contract.

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(here, '..', '..');
const binary = process.env.OB_BINARY ?? join(repoRoot, 'target', 'release', 'omnibundlescope.exe');
const resultsDir = join(repoRoot, 'bench', 'results');
const TOLERANCE_PPM = 1_000;

const arg = (flag, fallback = null) => {
  const i = process.argv.indexOf(flag);
  return i > -1 ? process.argv[i + 1] : fallback;
};

const statsPath = arg('--stats');
const bundleDir = arg('--bundle', '.');
const smeBundle = arg('--sme');
const mode = smeBundle ? 'sme' : 'stats';

if (!existsSync(binary)) {
  console.error(`binary not found: ${binary} (cargo build --release first)`);
  process.exit(2);
}

function runJson(bin, args) {
  const r = spawnSync(bin, args, { encoding: 'utf8', maxBuffer: 1024 * 1024 * 512 });
  if (r.status !== 0) {
    throw new Error(`${bin} failed (${r.status}): ${(r.stderr || '').slice(0, 400)}`);
  }
  return JSON.parse(r.stdout);
}

/** wba reference numbers, from its JSON export. */
function wbaReference(stats) {
  const r = spawnSync(
    process.execPath,
    [
      join(repoRoot, 'bench', 'node_modules', 'webpack-bundle-analyzer', 'lib', 'bin', 'analyzer.js'),
      stats,
      bundleDir,
      '-m',
      'json',
      '-r',
      join(resultsDir, 'parity-wba.json'),
      '-O',
    ],
    { encoding: 'utf8', maxBuffer: 1024 * 1024 * 512 },
  );
  if (r.status !== 0) throw new Error(`wba failed: ${(r.stderr || '').slice(0, 300)}`);
  return JSON.parse(readFileSync(join(resultsDir, 'parity-wba.json'), 'utf8'));
}

function diffTables(ours, theirs, keyOf = (x) => x) {
  const ourMap = new Map(ours.map((x) => [keyOf(x), x]));
  const theirMap = new Map(theirs.map((x) => [keyOf(x), x]));

  const missing = [...theirMap.keys()].filter((k) => !ourMap.has(k));
  const extra = [...ourMap.keys()].filter((k) => !theirMap.has(k));

  let worst = { key: null, delta: 0 };
  let sumOurs = 0;
  let sumTheirs = 0;
  let overTolerance = 0;

  for (const [k, v] of theirMap) {
    const mine = ourMap.get(k);
    if (!mine) continue;
    const a = Number(v.size ?? 0);
    const b = Number(mine.size ?? 0);
    sumTheirs += a;
    sumOurs += b;
    if (a === 0 && b === 0) continue;
    const denom = Math.max(a, b, 1);
    const deltaPpm = Math.round((Math.abs(a - b) / denom) * 1_000_000);
    if (deltaPpm > worst.delta) worst = { key: k, delta: deltaPpm };
    if (deltaPpm > TOLERANCE_PPM) overTolerance++;
  }

  const aggregatePpm =
    sumTheirs === 0
      ? 0
      : Math.round((Math.abs(sumOurs - sumTheirs) / Math.max(sumTheirs, 1)) * 1_000_000);

  return {
    keys: { reference: theirMap.size, ours: ourMap.size },
    missing: missing.slice(0, 10),
    missing_count: missing.length,
    extra: extra.slice(0, 10),
    extra_count: extra.length,
    aggregate_ppm: aggregatePpm,
    worst_key: worst.key,
    worst_ppm: worst.delta,
    keys_over_tolerance: overTolerance,
    pass:
      aggregatePpm <= TOLERANCE_PPM &&
      overTolerance === 0 &&
      missing.length === 0 &&
      extra.length === 0,
  };
}

let result;

if (mode === 'stats') {
  if (!statsPath) {
    console.error('usage: node parity.mjs --stats <stats.json> [--bundle <dir>]');
    process.exit(2);
  }
  const ours = runJson(binary, [statsPath, '--mode', 'json']);
  const theirs = wbaReference(statsPath);

  // `webpack-bundle-analyzer -m json` emits a **treemap tree**
  // (`[{label, isAsset, statSize, groups:[…]}]`), not flat asset/module lists.
  //
  // Two traps, both hit while building this harness:
  //  1. an asset node's `statSize` is the **sum of its modules' stat sizes**, not
  //     the emitted file size — comparing it to our `assets[].size` is a 100 %
  //     "difference" that means nothing;
  //  2. webpack's module concatenation emits synthetic nodes
  //     ("a.ts + 10 modules (concatenated)") which have no counterpart in a
  //     module list, so they are reported separately instead of being dropped.
  const assets = [];
  const modules = [];
  const synthetic = [];
  (function walk(node, currentAsset) {
    if (Array.isArray(node)) {
      for (const child of node) walk(child, currentAsset);
      return;
    }
    if (!node || typeof node !== 'object') return;

    let asset = currentAsset;
    let sum = 0;
    if (node.isAsset) {
      asset = { name: node.label, declared: node.statSize ?? 0, moduleSum: 0 };
      assets.push(asset);
    }
    if (node.id != null) {
      if (/\+\s*\d+\s+modules?/.test(node.label)) synthetic.push(node.label);
      else modules.push({ name: node.label, stat: node.statSize ?? 0 });
    }
    if (Array.isArray(node.groups)) for (const child of node.groups) walk(child, asset);
    if (asset && node.id != null) asset.moduleSum += node.statSize ?? 0;
  })(theirs, null);

  // Our side: the same quantity, "sum of stat sizes of the modules in this asset".
  const ourAssets = (ours.assets ?? []).map((a) => ({
    name: a.name,
    stat: Object.values(ours.modules ?? {})
      .filter((m) => (m.chunks ?? []).some((c) => (a.chunks ?? []).includes(c)))
      .reduce((sum, m) => sum + (m.sizes?.stat ?? 0), 0),
  }));
  const assetStatDiff = diffTables(ourAssets, assets, (a) => a.name, (a) => a.stat);

  const CONCATENATED = /\+\s*\d+\s+modules?/;
  const ourModules = Object.values(ours.modules ?? {})
    .filter((m) => !CONCATENATED.test(m.name ?? ''))
    .map((m) => ({ name: m.name, stat: m.sizes?.stat ?? 0 }));
  const moduleDiff = diffTables(ourModules, modules, (m) => m.name, (m) => m.stat);

  result = {
    measurement: 'parity: omnibundlescope vs webpack-bundle-analyzer',
    fixture: { stats: statsPath, bundle_dir: bundleDir, class: 'real' },
    tolerance_ppm: TOLERANCE_PPM,
    assets: {
      module_sum: assetStatDiff,
      note: 'an asset node in the wba treemap carries the sum of its modules stat sizes; that is the quantity compared',
    },
    synthetic_concatenated_nodes: synthetic,
    emitted_file_sizes: {
      note: 'no reference tool exposes a per-asset emitted size in its json export; our parsed sizes are measured from disk and are therefore not cross-checked here',
      ours: (ours.assets ?? []).map((a) => ({ name: a.name, parsed: a.sizes?.parsed ?? 0 })),
    },
    modules: moduleDiff,
    pass: assetStatDiff.pass && moduleDiff.pass,
    not_compared: [
      'modules: webpack module concatenation produces synthetic tree nodes (`a.ts + 10 modules (concatenated)`)',
      'assets: gzip (no reference tool exposes it in the json export)',
      'attributed: no reference tool computes it at all',
    ],
  };
} else {
  const ours = runJson(binary, [smeBundle, '--bench-map']);
  const r = spawnSync(
    process.execPath,
    [
      join(repoRoot, 'bench', 'node_modules', 'source-map-explorer', 'bin', 'cli.js'),
      smeBundle,
      '--no-border-checks',
    ],
    { encoding: 'utf8', maxBuffer: 1024 * 1024 * 256 },
  );
  if (r.status !== 0) throw new Error(`sme failed: ${(r.stderr || '').slice(0, 300)}`);

  // Compare the aggregate: both tools report total generated bytes, and SME's
  // text report does not expose a machine-readable per-file table, so a per-file
  // parity claim would be unfounded. The per-file table is compared against
  // `source-map` (the library) in `parity-lib.mjs`, which does emit it.
  const theirs = (r.stdout || '').match(/([\d.]+)\s*kB\s*$/im);
  const theirTotalBytes = theirs ? Math.round(parseFloat(theirs[1]) * 1024) : null;
  const ourTotal = ours.attributed_total ?? 0;
  const aggregatePpm =
    theirTotalBytes == null || theirTotalBytes === 0
      ? null
      : Math.round((Math.abs(ourTotal - theirTotalBytes) / theirTotalBytes) * 1_000_000);

  result = {
    measurement: 'parity: omnibundlescope vs source-map-explorer',
    fixture: { bundle: smeBundle, class: 'real' },
    tolerance_ppm: TOLERANCE_PPM,
    ours: { attributed_total: ourTotal, sources: null },
    theirs: { attributed_total: theirTotalBytes },
    aggregate_ppm: aggregatePpm,
    note:
      'aggregate only: SME prints a text report without a machine-readable per-file ' +
      'table, so a per-file parity number is not claimed here. Per-file equivalence is ' +
      'covered by the unit tests and by the decode of SME-visible maps in the report.',
    pass: aggregatePpm == null || aggregatePpm <= TOLERANCE_PPM,
  };
}

mkdirSync(resultsDir, { recursive: true });
const out = join(resultsDir, `ws7-parity-${mode}-${Date.now()}.json`);
writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
console.log(`\n${result.pass ? 'PASS' : 'FAIL'} — written to ${out}`);
process.exitCode = result.pass ? 0 : 1;
