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

const N_ASSETS = 1500;
const N_CHUNKS = 200;

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
        name: `chunk.${i}.js`,
        size: 20000 + i * 37,
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
        // and make every module count a lie.
        identifier: `/repo/node_modules/pkg${id % 400}/dist/index-${id}.js`,
        name: `./node_modules/pkg${id % 400}/dist/index-${id}.js`,
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
