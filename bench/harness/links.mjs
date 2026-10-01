// The one place the URLs this project does not control yet are written down.
//
//   node bench/harness/links.mjs            report: what is filled in, what is not, where
//   node bench/harness/links.mjs --apply    substitute the values into every tracked file
//   node bench/harness/links.mjs --deny     exit 1 if anything is still a placeholder
//
// Why this exists: a README full of plausible URLs for a repository that does not
// exist is worse than one with visible holes in it. Today every badge on the front
// page is broken and every "report an issue" link goes nowhere, and nothing says
// so, because a dead external link is not a build failure.
//
// --deny is what the release workflow uses. A placeholder should be annoying while
// you are filling them in and impossible to publish with.

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const TABLE = 'repo-links.json';
const args = new Set(process.argv.slice(2));
const apply = args.has('--apply');
const deny = args.has('--deny');

// URLs that must never be hardcoded in the tree. Each one is a plausible-looking
// URL for a repository that does not exist, which is the mistake this table exists
// to make impossible to make twice.
//
// Only enforced while the table still has empty values. Once filled in, these
// strings can be *correct* - someone may legitimately own fastscope.dev or the
// fastscope GitHub org - and a gate that fires on their own choice is worse than
// no gate. The file itself is skipped, or it would match its own needle list.
const INVENTED = [
  'github.com/fastscope/fastscope',
  'fastscope.github.io',
  'fastscope.dev',
  'img.shields.io/github/v/release/fastscope',
];

// Claims that a package can be installed. Worse than a dead link: a reader cannot
// tell whether the command failed or the project is lying, and these are the lines
// people copy. Refused while the matching URL is unset, which is exactly until the
// release workflow has published something.
//
// Split in two on purpose. A command inside a fenced block is an instruction; the
// same words in a sentence can be the opposite - "`npx fastscope` does not work
// yet" is the honest sentence this gate exists to encourage. A badge URL carries no
// such ambiguity, so it is matched anywhere.
//
// Only the front pages are scanned: `docs/` describes the npm one-liner as the
// target shape of the CLI, which is a third kind of sentence again.
const UNAVAILABLE_COMMAND = [
  ['NPM_URL', 'npx fastscope'],
  ['NPM_URL', 'npm i fastscope'],
  ['NPM_URL', 'npm install fastscope'],
  // fastscope-cli, not fastscope: there is no crate called `fastscope`, and an
  // install line that 404s on crates.io is the exact failure this list is for.
  ['CRATES_CORE_URL', 'cargo install fastscope-cli'],
];

const UNAVAILABLE_BADGE = [
  ['NPM_URL', 'img.shields.io/npm/v/'],
  ['CRATES_CORE_URL', 'img.shields.io/crates/v/'],
];

const fenced = (text) => (text.match(/```[\s\S]*?```/g) ?? []).join('\n');

const FRONT_PAGE = /^README(\.[a-z]{2})?\.md$/;

const BINARY = new Set(['.png', '.svg', '.ico', '.woff2', '.pdf', '.zip', '.gz', '.jpg', '.jpeg']);

function trackedFiles() {
  return execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8' })
    .split('\0')
    .filter((f) => f && f !== TABLE && !BINARY.has(f.slice(f.lastIndexOf('.'))));
}

const table = JSON.parse(readFileSync(TABLE, 'utf8'));
const tokens = Object.keys(table.links);
const tokenPattern = new RegExp(`\\{\\{(${tokens.join('|')})\\}\\}`, 'g');

// ---------------------------------------------------------------- report

const files = trackedFiles();
const where = new Map(tokens.map((t) => [t, []]));
const unknown = [];
const inventedHits = [];

for (const file of files) {
  const text = readFileSync(file, 'utf8');
  for (const match of text.matchAll(tokenPattern)) {
    where.get(match[1]).push(file);
  }
  for (const match of text.matchAll(/\{\{([A-Z][A-Z0-9_]*)\}\}/g)) {
    if (!tokens.includes(match[1]) && !unknown.some((u) => u.token === match[1])) {
      unknown.push({ token: match[1], file });
    }
  }
}

const empty = tokens.filter((t) => !table.links[t].value);

// Enforced only while something is unfilled; see the note on INVENTED.
const unavailableHits = [];
if (empty.length > 0) {
  for (const file of files) {
    if (file === 'bench/harness/links.mjs') continue;
    const text = readFileSync(file, 'utf8');
    for (const needle of INVENTED) {
      if (text.includes(needle)) inventedHits.push({ needle, file });
    }
    if (!FRONT_PAGE.test(file)) continue;
    const offered = fenced(text);
    for (const [token, needle] of UNAVAILABLE_COMMAND) {
      if (!table.links[token].value && offered.includes(needle)) {
        unavailableHits.push({ token, needle, file });
      }
    }
    for (const [token, needle] of UNAVAILABLE_BADGE) {
      if (!table.links[token].value && text.includes(needle)) {
        unavailableHits.push({ token, needle, file });
      }
    }
  }
}

const problems = [];

// A value whose shape differs from its example is usually a half-finished edit: a
// URL pasted where a bare package name belongs, or a scheme left off. Checked
// against the example rather than a hand-written pattern, because two of these are
// legitimately schemeless and three legitimately have a scheme.
for (const [token, entry] of Object.entries(table.links)) {
  if (!entry.value) continue;
  const exampleIsUrl = entry.example.startsWith('https://');
  const valueIsUrl = entry.value.startsWith('https://');
  const unfinished =
    entry.value.includes('{{') ||
    entry.value.trim() !== entry.value ||
    valueIsUrl !== exampleIsUrl ||
    entry.value.length === 0;
  if (unfinished) {
    problems.push(
      `${token}: "${entry.value}" does not look like the example (${entry.example}) - ` +
        'a URL needs a scheme, a package name or a slug needs none',
    );
  }
}

// The package names are also declared in the manifests. If the badge says one
// thing and the published package is called another, the badge is decoration.
const MISMATCH = [
  ['NPM_PACKAGE', 'npm/fastscope/package.json', (m) => m.name],
  ['CRATES_CORE_PACKAGE', 'crates/fastscope-core/Cargo.toml', (t) => /^\s*name\s*=\s*"([^"]+)"/m.exec(t)?.[1]],
];
for (const [token, file, read] of MISMATCH) {
  const value = table.links[token].value;
  if (!value) continue;
  const actual = read(readFileSync(file, 'utf8'));
  if (actual && actual !== value) {
    problems.push(`${token} is "${value}" but ${file} declares "${actual}" - the badge would point at a package that does not exist under that name`);
  }
}

if (apply) {
  if (empty.length) {
    console.error(
      `refusing to apply: ${empty.length} value(s) are still empty:\n` +
        empty.map((t) => `  ${t.padEnd(20)} e.g. ${table.links[t].example}`).join('\n'),
    );
    process.exit(1);
  }
  let changed = 0;
  for (const file of files) {
    const before = readFileSync(file, 'utf8');
    const after = before.replace(tokenPattern, (whole, token) => table.links[token].value);
    if (after !== before) {
      writeFileSync(file, after, 'utf8');
      changed++;
      console.log(`  filled ${file}`);
    }
  }
  console.log(`\napplied to ${changed} file(s). The tokens are gone; commit and push.`);
  // Filling in is the whole job; there is nothing left to report.
  process.exit(problems.length || inventedHits.length || unavailableHits.length ? 1 : 0);
}

// ---------------------------------------------------------------- report mode

console.log(`links declared in ${TABLE}\n`);
for (const token of tokens) {
  const entry = table.links[token];
  const hits = where.get(token);
  const count = hits.length;
  const state = entry.value ? entry.value : 'NOT SET';
  console.log(`  ${token.padEnd(22)} ${state}`);
  console.log(`  ${' '.repeat(22)} ${count} file(s) still carry the token${count ? `: ${[...new Set(hits)].join(', ')}` : ''}`);
  if (!entry.value) console.log(`  ${' '.repeat(22)} example: ${entry.example}`);
  console.log(`  ${' '.repeat(22)} ${entry.note}\n`);
}

if (unknown.length) {
  console.error('unknown tokens in the tree, which means a typo that would never be filled in:');
  for (const { token, file } of unknown) console.error(`  {{${token}}} in ${file}`);
  console.error('');
}

if (inventedHits.length) {
  console.error('URLs for a repository that does not exist are still in the tree:');
  for (const { needle, file } of inventedHits) console.error(`  ${needle} in ${file}`);
  console.error('');
}

if (unavailableHits.length) {
  console.error('the front page offers an install that does not exist yet:');
  for (const { token, needle, file } of unavailableHits) {
    console.error(`  "${needle}" in ${file} - ${token} is unset, so this is a 404`);
  }
  console.error('');
}

if (problems.length) {
  console.error('values that look unfinished:');
  for (const p of problems) console.error(`  ${p}`);
  console.error('');
}

const broken = empty.length + unknown.length + inventedHits.length + unavailableHits.length + problems.length;

if (deny && broken) {
  console.error(`${broken} placeholder problem(s); a release must not ship with these.`);
  process.exit(1);
}
console.log(
  broken
    ? `${broken} placeholder problem(s) outstanding - see repo-links.json`
    : 'every URL is filled in',
);
