// Fail on mojibake in any tracked text file.
//
// This exists because of a real accident: a shell round-trip read a UTF-8 file
// as the console's code page and wrote it back, and the damage looked like
// plausible text in the diff. For a project whose README is in four languages,
// "the bytes are valid UTF-8 and the text is what was intended" has to be a
// gate, not a hope.
//
//   node bench/harness/check-encoding.mjs
//
// What counts as broken:
//   - U+FFFD REPLACEMENT CHARACTER: text that was decoded lossily at some point
//   - C1 controls: what a Latin-1/UTF-8 mix-up leaves behind
//   - the classic UTF-8-read-as-CP936/CP1252 fragments, matched literally
//   - a UTF-8 BOM in a source file, which Rust and Node both tolerate but which
//     is noise in a diff
//
// Non-ASCII is otherwise fine and expected: the docs are translated, and
// typographic punctuation (em dash, curly quotes, ellipsis, arrows) is
// deliberate throughout.

import { readFileSync, readdirSync, statSync } from 'node:fs';
import { extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
// `repos` holds fetched upstream checkouts: their test corpora legitimately
// contain U+FFFD and their licence headers are not ours to normalise.
const SKIP_DIRS = new Set([
  '.git', 'target', 'node_modules', 'artifacts', 'vendor', 'coverage', 'repos',
]);
// Path prefixes to skip anywhere in the tree, for generated output that is not a
// directory name we can match on (ook/book/).
const SKIP_PREFIXES = ['book/book'];
// Authored files that are allowed to contain the patterns on purpose - a doc
// *about* mojibake, for instance.
const ALLOW = new Set(['bench/harness/check-encoding.mjs']);
const EXTENSIONS = new Set([
  '.rs', '.md', '.mjs', '.js', '.json', '.toml', '.yml', '.yaml', '.html',
  '.css', '.sh', '.ps1', '.txt', '.svg', '.gitignore', '.gitattributes',
]);

// Fragments that only appear when UTF-8 was decoded as a single-byte or CJK
// code page. Written as escapes so this file does not match itself.
const SUSPECT = [
  '\u00EF\u00BF\u00BD', // UTF-8 BOM read as Latin-1
  '\u00E2\u20AC\u201C', // left double quote
  '\u00E2\u20AC', // em/en dash, ellipsis
  '\u00C3\u00A4', // accented latin letter
  '\u00E3\u20AC', // box-drawing / CJK punctuation
  '\u00E5\u00AE', // CJK two-byte fragment
  '\u00E7\u0161', // CJK fragment
  '\u00E9\u0192', // surrogate fragment
];

function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') && entry.name !== '.github') continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      yield* walk(full);
    } else if (EXTENSIONS.has(extname(entry.name)) || entry.name.startsWith('.')) {
      if (statSync(full).size > 2 * 1024 * 1024) continue;
      yield full;
    }
  }
}

const problems = [];
let scanned = 0;

for (const file of walk(repoRoot)) {
  const rel = relative(repoRoot, file).replace(/\\/g, '/');
  if (ALLOW.has(rel)) continue;
  if (SKIP_PREFIXES.some((prefix) => rel.startsWith(prefix))) continue;
  scanned++;
  const bytes = readFileSync(file);
  const text = bytes.toString('utf8');

  if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    problems.push(`${rel}: has a UTF-8 BOM`);
  }
  if (text.includes('\uFFFD')) {
    const line = text.slice(0, text.indexOf('\uFFFD')).split('\n').length;
    problems.push(`${rel}:${line}: U+FFFD replacement character`);
  }
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    // C1 controls, excluding the newline/tab characters that live there.
    if (code >= 0x80 && code <= 0x9f) {
      problems.push(`${rel}: C1 control U+${code.toString(16).padStart(4, '0')}`);
      break;
    }
  }
  for (const suspect of SUSPECT) {
    if (text.includes(suspect)) {
      const line = text.slice(0, text.indexOf(suspect)).split('\n').length;
      problems.push(`${rel}:${line}: mojibake fragment ${JSON.stringify(suspect)}`);
    }
  }
}

if (problems.length === 0) {
  console.log(`ok  ${scanned} files, no encoding damage`);
  process.exit(0);
}
console.error(`${problems.length} encoding problem(s):`);
for (const p of problems.slice(0, 40)) console.error(`  ${p}`);
process.exit(1);