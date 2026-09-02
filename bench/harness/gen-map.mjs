// Synthetic source map generator (WS-A / WS-S).
//
// Purpose: give `source-map-explorer` and our WS-3 ingest the same input at a
// scale real small projects cannot reach (10k+ sources, a long `mappings`
// string). Real fixtures (preact/marked/chalk/dayjs) cover correctness; this
// covers scale. Results from this generator are always labelled `synthetic`.
//
// Usage:
//   node gen-map.mjs <sources> <outFile> [generatedBytes]
// e.g.
//   node gen-map.mjs 10000 /tmp/map-10k-sources.json 8000000

import fs from 'node:fs';

const N_SOURCES = Number(process.argv[2] || 10_000);
const out = process.argv[3] || 'map-synthetic.json';
const GENERATED_BYTES = Number(process.argv[4] || 8_000_000);

// Base64 VLQ, the encoding the spec requires for `mappings`.
const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
function vlq(value) {
  let v = value < 0 ? ((-value) << 1) | 1 : value << 1;
  let out = '';
  do {
    let digit = v & 31;
    v >>>= 5;
    if (v > 0) digit |= 32;
    out += B64[digit];
  } while (v > 0);
  return out;
}

const sources = [];
const sourcesContent = [];
for (let i = 0; i < N_SOURCES; i++) {
  sources.push(`webpack://fixture/./src/module-${i}.ts`);
  const body = Array.from({ length: 12 }, (_, l) => `export const v${i}_${l} = ${i * l};`).join('\n');
  sourcesContent.push(`${body}\n// padding ${'.'.repeat(120)}\n`);
}

// ~4 mappings per generated "line": every source appears repeatedly, which is
// what makes per-file attribution non-trivial.
//
// VLQ fields per segment (spec order): generatedColumn (delta within the line,
// absolute for the first segment), sourceIndex (delta across the whole map),
// originalLine (delta), originalColumn (delta).
const lines = Math.max(1, Math.floor(GENERATED_BYTES / 80));
const segsPerLine = 4;
let prevSource = 0;
let prevOrigLine = 0;
let prevOrigCol = 0;
const segLines = [];

for (let l = 0; l < lines; l++) {
  const segs = [];
  let genCol = 0;
  for (let s = 0; s < segsPerLine; s++) {
    const dGen = s === 0 ? 0 : 12 + (s % 5);
    genCol += dGen;
    const srcIdx = (l * segsPerLine + s) % N_SOURCES;
    const origLine = (l * segsPerLine + s) % 12;
    const origCol = (l + s) % 20;
    segs.push(
      vlq(dGen) +
        vlq(srcIdx - prevSource) +
        vlq(origLine - prevOrigLine) +
        vlq(origCol - prevOrigCol),
    );
    prevSource = srcIdx;
    prevOrigLine = origLine;
    prevOrigCol = origCol;
  }
  segLines.push(segs.join(','));
}
const mappings = segLines.join(';');

const map = {
  version: 3,
  file: 'bundle.js',
  sources,
  sourcesContent,
  names: [],
  mappings,
};

fs.writeFileSync(out, JSON.stringify(map));
const bytes = fs.statSync(out).size;
console.log(
  JSON.stringify({
    file: out,
    MB: +(bytes / 1048576).toFixed(2),
    sources: N_SOURCES,
    mapping_lines: lines,
    mappings_len: mappings.length,
  }),
);
