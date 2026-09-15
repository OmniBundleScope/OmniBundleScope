// One-off probe: what do the stats and the map call the same source?
// (Written as a file because the quoting for a multi-line `node -e` in
// PowerShell is not worth fighting.)
import fs from 'node:fs';

const statsPath = process.argv[2];
const mapPath = process.argv[3];

const fd = fs.openSync(statsPath, 'r');
const buf = Buffer.alloc(600_000);
const n = fs.readSync(fd, buf, 0, buf.length, 0);
fs.closeSync(fd);
const head = buf.subarray(0, n).toString('latin1');

for (const key of ['identifier', 'name']) {
  const re = new RegExp(`"${key}":"[^"]+"`, 'g');
  const found = head.match(re) ?? [];
  console.log(`${key}: ${found.slice(0, 3).join('  ')}  (${found.length} in the first 600 KB)`);
}

const map = JSON.parse(fs.readFileSync(mapPath, 'utf8'));
console.log(`map.file: ${map.file}`);
console.log(`map.sources: ${map.sources.length}  e.g. ${map.sources.slice(0, 2).join(', ')}`);
