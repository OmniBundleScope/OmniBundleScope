// One-off: repoint the "Translations:" header line in the EN documents.
//
//   node bench/harness/.fix-translation-links.mjs
//
// The EN documents each linked to their own translation, which does not exist
// yet: 30 dead links, found by check-links.mjs. The header now points at the
// language READMEs, which do exist, and says plainly that the per-document
// translations are still in progress. When a translation lands, the line goes
// back to linking it.
//
// Kept as a script rather than a sed one-liner because it is a deliberate edit
// to ten files and the pattern has to match exactly once in each.

import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
const dir = join(repoRoot, 'docs', 'en');

// Two shapes exist in the wild: the links on their own line, and the same links
// trailing an "Owner: ..." line and wrapped across two. Matching the link
// group alone covers both without depending on the surrounding prose.
const PATTERN =
  /\[ZH\]\(\.\.\/zh\/[0-9a-z-]+\.md\) ·\r?\n?\[JA\]\(\.\.\/ja\/[0-9a-z-]+\.md\) · \[DE\]\(\.\.\/de\/[0-9a-z-]+\.md\)/g;
const REPLACEMENT =
  'Translations: per-document translations are still in progress. ' +
  '[ZH](../zh/README.md) · [JA](../ja/README.md) · [DE](../de/README.md)';

let changed = 0;
for (const name of readdirSync(dir)) {
  if (!name.endsWith('.md') || name === 'GLOSSARY.md') continue;
  const path = join(dir, name);
  const before = readFileSync(path, 'utf8');
  const after = before.replace(PATTERN, REPLACEMENT);
  if (after !== before) {
    writeFileSync(path, after, 'utf8');
    console.log(`updated docs/en/${name}`);
    changed++;
  }
}
console.log(`${changed} file(s) updated`);
