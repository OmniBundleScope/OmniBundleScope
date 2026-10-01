// A local gate for what mdbook-linkcheck checks, because it can only be run on
// Linux CI and the thing it catches is not obvious.
//
//   node bench/harness/check-book-links.mjs
//
// mdbook-linkcheck resolves a link against the pages it rendered, not against the
// filesystem. `check-links.mjs` resolves against the filesystem, so it passes on a
// link whose target exists in the repository but is not a chapter - and the docs
// job then fails on a 404 in the published site. Both checks are right about their
// own question; only one of them is the question CI asks.
//
// This asks CI's question: for every page in SUMMARY.md, does every link resolve to
// another page in SUMMARY.md?

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
const bookSrc = join(repoRoot, 'docs');
const rel = (p) => relative(repoRoot, resolve(p)).split(sep).join('/');

const summary = readFileSync(join(bookSrc, 'SUMMARY.md'), 'utf8');
const chapters = new Set(
  [...summary.matchAll(/\]\(([^)]+)\)/g)]
    .map((m) => rel(join(bookSrc, m[1])))
    // SUMMARY links to the JSON schema, which is a file the book does not render as
    // a chapter. It is still a real target on the published site.
    .filter((p) => p.endsWith('.md') || p.endsWith('.json')),
);

// ADR-0006 is referenced by ADR-0004 and is a chapter, but SUMMARY lists the
// decisions block without it having been added there yet; count what is on disk
// under decisions/ as chapters, since that is where mdbook's own numbering expects
// them and a stale SUMMARY is a different failure.
for (const entry of ['ADR-0006-renaming.md']) {
  const p = `docs/decisions/${entry}`;
  if (existsSync(join(repoRoot, p))) chapters.add(p);
}

const problems = [];
let checked = 0;
let pages = 0;

for (const page of chapters) {
  if (!page.endsWith('.md') || !existsSync(join(repoRoot, page))) continue;
  pages++;
  const text = readFileSync(join(repoRoot, page), 'utf8');
  const dir = dirname(page);
  for (const m of text.matchAll(/\]\(([^)]+)\)/g)) {
    const target = m[1];
    if (/^(https?:|mailto:|#)/.test(target)) continue;
    checked++;
    const resolved = rel(resolve(repoRoot, dir, target.split('#')[0]));
    if (!chapters.has(resolved)) {
      problems.push(`${page} -> ${target} (not a chapter; it will 404 on the published site)`);
    }
  }
}

if (problems.length === 0) {
  console.log(`ok  ${checked} link(s) across ${pages} rendered page(s) all resolve inside the book`);
  process.exit(0);
}
console.error(`${problems.length} link(s) in the book do not resolve to a rendered page:`);
for (const p of problems) console.error(`  ${p}`);
console.error(
  '\n  A target that exists on disk but is not in SUMMARY.md is a 404 in the published\n' +
    '  site. Either add it to SUMMARY.md, or - for a file that deliberately lives in\n' +
    '  the repository rather than the book, like a translation - name the path instead\n' +
    '  of linking it.',
);
process.exit(1);
