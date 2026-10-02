// Build the real fixtures into the artifacts OmniBundleScope ingests (benchmarks).
//
// We build with our own bundler config rather than each project's release
// and must be reproducible, not depend on whatever a project's CI does this
// month. Two variants per fixture, because they stress different things:
//
//   <name>.map        readable bundle + full source maps  → correctness, parity
//   <name>.min.map    minified bundle + source maps      → realistic sizes
//
// Usage:  node build-fixtures.mjs [--minify-only]

import { build } from 'esbuild';
import { existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repos = join(here, '..', 'repos');
const outDir = join(here, '..', 'artifacts');

// entry, resolved inside the cloned fixture
const targets = [
  { name: 'preact', entry: 'src/index.js' },
  { name: 'marked', entry: 'src/marked.ts' },
  { name: 'chalk', entry: 'source/index.js' },
  { name: 'p-limit', entry: 'index.js' },
  { name: 'mitt', entry: 'src/index.mjs' },
  { name: 'ufo', entry: 'src/index.mjs' },
];

const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');
import { readFileSync } from 'node:fs';

const manifest = [];

for (const t of targets) {
  const dir = join(repos, t.name);
  const entry = join(dir, t.entry);
  if (!existsSync(entry)) {
    console.log(`skip ${t.name}: ${t.entry} not found`);
    continue;
  }
  const out = join(outDir, t.name);
  mkdirSync(out, { recursive: true });

  for (const variant of [
    { suffix: '', minify: false },
    { suffix: '.min', minify: true },
  ]) {
    const js = join(out, `bundle${variant.suffix}.js`);
    try {
      await build({
        entryPoints: [entry],
        bundle: true,
        minify: variant.minify,
        sourcemap: true,
        outfile: js,
        platform: 'neutral',
        target: 'es2022',
        absWorkingDir: dir,
        logLevel: 'silent',
      });
    } catch (err) {
      // A fixture whose own dependencies are not installed yet (or whose
      // entry needs build-time config) is skipped, never fatal: the harness
      // must be runnable with a partial fixture set.
      console.log(`skip ${t.name}${variant.suffix}: ${String(err.message).split('\n')[0]}`);
      continue;
    }
    const map = `${js}.map`;
    const stat = { js: statSync(js).size, map: statSync(map).size };
    manifest.push({
      fixture: t.name,
      entry: t.entry,
      variant: variant.suffix ? 'minified' : 'readable',
      js_bytes: stat.js,
      map_bytes: stat.map,
      js_sha256: sha256(js),
      map_sha256: sha256(map),
      map_path: map,
    });
    console.log(
      `${t.name}${variant.suffix}: js ${(stat.js / 1024).toFixed(0)} KB, map ${(stat.map / 1024).toFixed(0)} KB`,
    );
  }
}

writeFileSync(
  join(outDir, 'artifacts.json'),
  `${JSON.stringify({ built: new Date().toISOString(), artifacts: manifest }, null, 2)}\n`,
);
console.log(`\n${manifest.length} artifacts → ${outDir}`);
