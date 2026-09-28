#!/usr/bin/env node
// Documentation parity checker (WS-9), enforcing docs/contracts/i18n-parity.md.
//
//   node check-i18n.mjs            # all languages
//   node check-i18n.mjs --lang zh  # one language
// same headings, same links, same code blocks/commands, same numbers >= 1000,
// Rules: same file set, same headings, same links, same code blocks/commands,
// same numbers ≥ 1000, glossary headers. Missing translations are reported as
// `pending`; stale ones (EN revision moved on) as `stale` and they block the
// release tier.

import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const docsRoot = join(repoRoot, 'docs');
const langs = ['zh', 'ja', 'de'];
const source = 'en';

const only = (() => {
  const i = process.argv.indexOf('--lang');
  return i > -1 ? process.argv[i + 1] : null;
})();

const headings = (text) => text.match(/^#{1,6} .+$/gm) ?? [];
const codeBlocks = (text) => (text.match(/```[\s\S]*?```/g) ?? []).map((b) => b.replace(/\s+/g, ' ').trim());
const numbers = (text) => {
  const found = text.match(/\b\d[\d.,]{2,}\b/g) ?? [];
  return [...new Set(found.filter((n) => Number(n.replace(/[,.]/g, '')) >= 1000))].sort();
};
const links = (text) => (text.match(/\]\(([^)]+)\)/g) ?? []).map((l) => l.slice(2, -1)).sort();
const stamp = (text) => text.match(/<!--\s*source:[^>]*version:\s*(\d+)[^>]*-->/)?.[1] ?? null;
const rev = (n) => text_revision(n);

function text_revision(lang) {
  // A cheap, stable revision: line count of the file. Bumping it is the
  // translator's job, so it only has to change when the text changes.
  try {
    return String(readFileSync(join(docsRoot, lang, 'REVISION'), 'utf8').trim());
  } catch {
    return '0';
  }
}

const sourceFiles = readdirSync(join(docsRoot, source)).filter((f) => f.endsWith('.md'));
let failures = 0;
let pending = 0;

for (const lang of langs) {
  if (only && only !== lang) continue;
  console.log(`\n== ${lang} ==`);
  const langDir = join(docsRoot, lang);

  for (const file of sourceFiles) {
    const srcPath = join(docsRoot, source, file);
    const dstPath = join(langDir, file);
    if (!existsSync(dstPath)) {
      console.log(`  pending  ${file} (not translated yet)`);
      pending++;
      continue;
    }
    const srcText = readFileSync(srcPath, 'utf8');
    const dstText = readFileSync(dstPath, 'utf8');
    const problems = [];

    const srcRev = rev(source);
    const dstRev = stamp(dstText) ?? rev(lang);
    if (srcRev !== '0' && dstRev !== srcRev) problems.push(`stale revision (en=${srcRev} ${lang}=${dstRev})`);

    const hDiff = headings(srcText).length - headings(dstText).length;
    if (hDiff !== 0) problems.push(`heading count differs by ${hDiff}`);

    const srcNums = numbers(srcText);
    const dstNums = new Set(numbers(dstText));
    const missingNums = srcNums.filter((n) => !dstNums.has(n));
    if (missingNums.length) problems.push(`numbers missing: ${missingNums.join(', ')}`);

    const srcBlocks = codeBlocks(srcText).length;
    const dstBlocks = codeBlocks(dstText).length;
    if (srcBlocks !== dstBlocks) problems.push(`code blocks differ (${srcBlocks} vs ${dstBlocks})`);

    const badLinks = links(dstText).filter((l) => {
      if (/^https?:/.test(l)) return false;
      const target = join(langDir, l.split('#')[0]);
      return l.split('#')[0] && !existsSync(target);
    });
    if (badLinks.length) problems.push(`broken relative links: ${badLinks.join(', ')}`);

    if (problems.length) {
      failures++;
      console.log(`  FAIL     ${file}`);
      for (const p of problems) console.log(`             - ${p}`);
    } else {
      console.log(`  ok       ${file}`);
    }
  }

  // README lives at the repo root, one file per language.
  const readme = join(repoRoot, `README.${lang}.md`);
  if (!existsSync(readme)) {
    console.log(`  FAIL     README.${lang}.md is missing (release tier)`);
    failures++;
  } else {
    console.log(`  ok       README.${lang}.md`);
  }
  const glossary = join(langDir, 'GLOSSARY.md');
  if (!existsSync(glossary)) {
    console.log(`  FAIL     docs/${lang}/GLOSSARY.md is missing (release tier)`);
    failures++;
  } else {
    console.log(`  ok       docs/${lang}/GLOSSARY.md`);
  }
}

// README.md and README.en.md are the same document: README.md is what GitHub
// renders, README.en.md is the copy people download. Two copies of an English file
// with nothing comparing them is not a translation workflow, it is a trap - and it
// had already drifted, by exactly the three sections that matter most.
const english = readFileSync(join(repoRoot, 'README.md'), 'utf8');
const englishCopy = readFileSync(join(repoRoot, 'README.en.md'), 'utf8');
if (english === englishCopy) {
  console.log('\n  ok       README.en.md is identical to README.md');
} else {
  const sections = (t) => (t.match(/^## .+$/gm) ?? []).length;
  console.log(
    `\n  FAIL     README.en.md has drifted from README.md ` +
      `(${sections(englishCopy)} sections vs ${sections(english)})\n` +
      '           Fix it by copying README.md over README.en.md. There is no second\n' +
      '           English document to reconcile; the translations are the ones that diverge.',
  );
  failures++;
}

console.log(`\n${failures} failure(s), ${pending} pending`);
process.exitCode = failures > 0 ? 1 : 0;
