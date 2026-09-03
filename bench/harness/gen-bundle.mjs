// Emit a generated bundle next to a synthetic source map, so tools that expect
// a JS entry point (source-map-explorer reads the bundle and follows
// `sourceMappingURL`) can be measured on the same input.
//
//   node gen-bundle.mjs <mapFile> <generatedBytes> [outJs]
//
// Example:
//   node gen-bundle.mjs /tmp/ob-map-10k.json 8000000 /tmp/ob-map-10k.bundle.js

import { readFileSync, writeFileSync, statSync } from 'node:fs';
import { basename } from 'node:path';

const mapFile = process.argv[2];
const generatedBytes = Number(process.argv[3] || 8_000_000);
const outJs = process.argv[4] || mapFile.replace(/\.json$/, '.bundle.js');

const map = JSON.parse(readFileSync(mapFile, 'utf8'));
// Line count implied by the mappings, so offsets in the JS and the map agree.
const lines = map.mappings.split(';').length;
const perLine = Math.max(1, Math.floor(generatedBytes / lines / 24));
const unit = 'var m=1;function f(a){return a*2+m}';
const parts = [`// generated fixture bundle for ${basename(mapFile)}\n`];
for (let i = 0; i < lines; i++) {
  parts.push(`${unit};`.repeat(Math.max(1, Math.floor(perLine / unit.length))));
}
parts.push(`\n//# sourceMappingURL=${basename(mapFile)}\n`);

writeFileSync(outJs, parts.join('\n'));
console.log(
  JSON.stringify({
    js: outJs,
    mb: +(statSync(outJs).size / 1048576).toFixed(2),
    lines,
    map: mapFile,
  }),
);
