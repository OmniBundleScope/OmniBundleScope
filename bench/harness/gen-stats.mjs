// Synthetic webpack stats.json generator (WS-A).
//
// Shape and field names match what `webpack-bundle-analyzer` consumes, and
// what our WS-1 ingest must read: 1,500 assets, 200 chunks, N modules with an
// embedded `source` string (that string is why real stats files get big).
//
// Usage:
//   node gen-stats.mjs <targetBytes> [outFile]
// e.g.
//   node gen-stats.mjs 381000000 /tmp/stats-400mb.json
//
// The file is written incrementally with fs.writeSync so the generator itself
// never holds the whole document in memory — otherwise measuring the parser
// would require more memory than the parser.

import fs from 'node:fs';

const target = Number(process.argv[2] || 381_000_000);
const out = process.argv[3] || 'stats-synthetic.json';

// Asset shape is configurable because the fusion benchmarks need a build that a
// single source map can actually *cover*: with 1,500 assets, one map covers 0.1%
// of the bytes and the tool correctly refuses to call the result ground truth
// (coverage < 0.99). One asset of the bundle's real size is the case B8 means.
const argOf = (flag, dflt) => {
  const i = process.argv.indexOf(flag);
  return i > -1 ? Number(process.argv[i + 1]) : dflt;
};
const N_ASSETS = argOf('--assets', 1500);
const N_CHUNKS = Math.min(200, N_ASSETS);
const ASSET_NAME = process.argv.includes('--asset-name')
  ? process.argv[process.argv.indexOf('--asset-name') + 1]
  : null;
const ASSET_BYTES = argOf('--asset-bytes', 0);

// `--sources-from <map>` names the modules after a source map's `sources`, so the
// pair is coherent: a stats file whose modules are `node_modules/pkgN/...` next to
// a map of `src/module-N.ts` has a 0% coverage *by construction*, and measuring
// B8 on it would measure nothing but the tool correctly refusing to invent a join.
let SOURCE_NAMES = null;
if (process.argv.includes('--sources-from')) {
  const mapPath = process.argv[process.argv.indexOf('--sources-from') + 1];
  const map = JSON.parse(fs.readFileSync(mapPath, 'utf8'));
  // `webpack://fixture/./src/module-0.ts` -> `./src/module-0.ts`, which is how
  // webpack itself names a module in stats.
  SOURCE_NAMES = map.sources.map((s) => {
    const clean = s.replace(/^webpack:\/\/[^/]*\/?/, '');
    return clean.startsWith('.') ? clean : `./${clean.replace(/^\.?\//, '')}`;
  });
  console.error(`naming modules after ${SOURCE_NAMES.length} sources from ${map.file}`);
}

const fd = fs.openSync(out, 'w');
let written = 0;

function w(str) {
  const buf = Buffer.from(str);
  let off = 0;
  while (off < buf.length) off += fs.writeSync(fd, buf, off, buf.length - off);
  written += buf.length;
}

w(`{"version":"5.90.0","hash":"deadbeef","time":100,"builtAt":0,"publicPath":"/","outputPath":"/dist","assets":[`);
for (let i = 0; i < N_ASSETS; i++) {
  w(
    (i ? ',' : '') +
      JSON.stringify({
        type: 'asset',
        name: ASSET_NAME ?? `chunk.${i}.js`,
        size: ASSET_BYTES || 20000 + i * 37,
        chunks: [i % N_CHUNKS],
        chunkNames: [`chunk-${i % N_CHUNKS}`],
        emitted: true,
        info: { javascriptModule: false },
        auxiliaryFiles: [],
      }),
  );
}
w('],"chunks":[');
for (let i = 0; i < N_CHUNKS; i++) {
  const files = [];
  for (let a = 0; a < N_ASSETS; a++) if (a % N_CHUNKS === i) files.push(`chunk.${a}.js`);
  w(
    (i ? ',' : '') +
      JSON.stringify({
        id: i,
        names: [`chunk-${i}`],
        files,
        size: 20000 * files.length,
        entry: false,
        rendered: true,
        initial: true,
        modules: [],
        parents: [],
        siblings: [],
        children: [],
        origins: [],
      }),
  );
}
w('],"modules":[');

const sourceUnit = 'function f(a,b){return a+b*2-1;} '.repeat(60); // ~1.7 KB

// `id` is 1-based here, so the name lines up with the map's source order.
const moduleName = (n) =>
  SOURCE_NAMES
    ? SOURCE_NAMES[(n - 1) % SOURCE_NAMES.length]
    : `./node_modules/pkg${n % 400}/dist/index-${n}.js`;
let first = true;
let id = 1;
while (written < target) {
  const chunk = id % N_CHUNKS;
  w(
    (first ? '' : ',') +
      JSON.stringify({
        id,
        // Identifiers must be unique: they are the join key (unified-graph.md
        // §2), so a generator that repeats them would silently dedupe the graph
        // and make every module count a lie. With `--sources-from` the name comes
        // from the map instead, which is how a real pair is coherent.
        identifier: moduleName(id),
        name: moduleName(id),
        index: id,
        size: 1700 + (id % 900),
        cacheable: true,
        built: true,
        optional: false,
        prefetched: false,
        chunks: [chunk],
        issuer: null,
        issuerId: null,
        issuerName: null,
        failed: false,
        errors: 0,
        warnings: 0,
        assets: [],
        reasons: [],
        usedExports: true,
        providedExports: [],
        optimizationBailout: [],
        depth: 1,
        moduleType: 'javascript/auto',
        profile: { total: 100 },
        source: sourceUnit,
      }),
  );
  first = false;
  id++;
}

w('],"entrypoints":{},"errors":[],"warnings":[]}');
fs.closeSync(fd);

const result = {
  file: out,
  bytes: written,
  MB: +(written / 1048576).toFixed(2),
  modules: id - 1,
  assets: N_ASSETS,
  chunks: N_CHUNKS,
};
console.log(JSON.stringify(result));
