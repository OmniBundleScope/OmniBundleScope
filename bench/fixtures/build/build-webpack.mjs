// Real webpack build for the fixtures (parity needs a real graph).
//
// Our synthetic stats fixture is fine for scale benchmarks but useless for
// parity: its module names and sizes are ours, so comparing it against
// webpack-bundle-analyzer only proves the harness runs. This builds the actual
// fixture sources with webpack 5, which gives a genuine stats.json, a genuine
// bundle and a genuine source map.
//
//   node build-webpack.mjs <fixtureName>

import webpack from 'webpack';
import { mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repos = join(here, '..', 'repos');
const outDir = join(here, '..', 'artifacts', 'webpack');

const ENTRIES = {
  preact: { repo: 'preact', entry: './src/index.js' },
  marked: { repo: 'marked', entry: './src/marked.ts' },
  chalk: { repo: 'chalk', entry: './source/index.js' },
};

const name = process.argv[2] ?? 'preact';
const target = ENTRIES[name];
if (!target) {
  console.error(`usage: node build-webpack.mjs <${Object.keys(ENTRIES).join('|')}>`);
  process.exit(2);
}

const context = join(repos, target.repo);
const out = join(outDir, name);
mkdirSync(out, { recursive: true });

const config = {
  mode: 'production',
  context,
  entry: target.entry,
  output: {
    path: out,
    filename: '[name].js',
    sourceMapFilename: '[name].js.map',
    clean: true,
  },
  devtool: 'source-map',
  optimization: { minimize: true },
  // webpack's own stats export: the exact shape our ingest and wba both read
  stats: { all: true, source: true, reasons: true, chunks: true, modules: true },
  performance: { hints: false },
  experiments: { outputModule: false },
};

const compiler = webpack(config);

compiler.run((err, stats) => {
  if (err) {
    console.error(err);
    process.exit(3);
  }
  const info = stats.toJson({ all: true, source: false });
  writeFileSync(join(out, 'stats.json'), JSON.stringify(info));

  const { size } = statSync(join(out, 'main.js'));
  console.log(
    JSON.stringify({
      fixture: name,
      out,
      bundle_bytes: size,
      modules: info.modules?.length ?? 0,
      assets: info.assets?.length ?? 0,
      has_source_map: Boolean(info.assets?.some((a) => a.name.endsWith('.map'))),
    }),
  );
  compiler.close(() => {});
});
