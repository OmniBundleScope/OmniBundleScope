// Build a realistic vite dist/ layout, for the README transcript and for
// checking that folder ingest handles what vite actually emits.
//
//   node bench/harness/make-fixture.mjs <dir>
//
// One generated line per bundle with several segments on it, each advancing one
// source index, because that is the shape a minified build's map has: one long
// line, not one segment per line. A map whose `;` structure disagrees with the
// generated file attributes everything to whichever source owns position 0,
// which looks like a parser bug and is not one.

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) {
  console.error('usage: node make-fixture.mjs <dir>');
  process.exit(2);
}

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
function vlq(value) {
  let v = value < 0 ? ((-value << 1) | 1) : value << 1;
  let out = '';
  do {
    let digit = v & 31;
    v >>>= 5;
    if (v) digit |= 32;
    out += B64[digit];
  } while (v);
  return out;
}

const segment = (columnDelta, sourceIndexDelta) =>
  vlq(columnDelta) + vlq(sourceIndexDelta) + vlq(0) + vlq(0);

function sourceMap(file, sources, columns) {
  let previousColumn = 0;
  let previousSource = 0;
  const mappings = columns
    .map((column, index) => {
      const one = segment(column - previousColumn, index - previousSource);
      previousColumn = column;
      previousSource = index;
      return one;
    })
    .join(',');
  return JSON.stringify({
    version: 3,
    file,
    sources,
    sourcesContent: sources.map(() => 'export default 1;'.repeat(80)),
    names: [],
    mappings,
  });
}

const appSources = [
  '../src/main.ts',
  '../src/api/client.ts',
  '../src/ui/list.tsx',
  '../src/ui/row.tsx',
  '../src/util/format.ts',
  '../src/config.ts',
];
const vendorSources = [
  '../node_modules/react-dom/index.js',
  '../node_modules/react/index.js',
  '../node_modules/scheduler/index.js',
];

const appBody = 'a'.repeat(21_400);
const vendorBody = 'b'.repeat(231_000);
const cssBody = 'c'.repeat(6_210);

mkdirSync(join(dir, 'assets'), { recursive: true });
writeFileSync(join(dir, 'assets/index-BqP1xK.js'), appBody);
writeFileSync(
  join(dir, 'assets/index-BqP1xK.js.map'),
  sourceMap('index-BqP1x.js', appSources, [0, 4000, 8000, 11_000, 15_000, 20_000]),
);
writeFileSync(join(dir, 'assets/vendor-Dk9mZ2.js'), vendorBody);
writeFileSync(
  join(dir, 'assets/vendor-Dk9mZ2.js.map'),
  sourceMap('vendor-Dk9mZ.js', vendorSources, [0, 70_000, 150_000]),
);
writeFileSync(join(dir, 'assets/index-Cc3nQ8.css'), cssBody);
writeFileSync(join(dir, 'index.html'), '<!doctype html>'.padEnd(540, ' '));

console.log(
  `fixture written to ${dir}: ${appBody.length + vendorBody.length + cssBody.length + 540} bytes of output`,
);
