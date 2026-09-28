// Verify every relative link in the repository's Markdown resolves.
//
//   node bench/harness/check-links.mjs
//
// Broken links in a README are the most visible possible mistake in a project
// whose whole argument is that its claims can be checked, and they appear the
// moment a file is renamed. This walks the Markdown, resolves each relative
// target against the file it appears in, and reports the ones that dangle.
//
// Skipped deliberately: absolute URLs (checked by mdbook-linkcheck in CI for the
// book, and by the network otherwise), bare anchors, and images inside HTML that
// mdbook rewrites.

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
const SKIP_DIRS = new Set(['.git', 'target', 'node_modules', 'repos', 'artifacts', 'book']);
const SKIP_FILES = new Set(['docs/en/GLOSSARY.md', 'docs/zh/GLOSSARY.md', 'docs/ja/GLOSSARY.md', 'docs/de/GLOSSARY.md']);

function* markdownFiles(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') && entry.name !== '.github') continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      yield* markdownFiles(full);
    } else if (entry.name.endsWith('.md')) {
      if (statSync(full).size > 4 * 1024 * 1024) continue;
      yield full;
    }
  }
}

// [text](target) and ![alt](target), plus <img src="target"> in the raw HTML the
// README uses for its light/dark charts.
const LINK = /!?\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g;
const IMG_SRC = /<img[^>]+src="([^"]+)"/g;
const SUMMARY = /\[[^\]]*\]\(([^)]+)\)/g;

let broken = 0;
let checked = 0;

for (const file of markdownFiles(repoRoot)) {
  const rel = relative(repoRoot, file).replace(/\\/g, '/');
  if (SKIP_FILES.has(rel)) continue;
  const text = readFileSync(file, 'utf8');
  const base = dirname(file);

  const targets = [
    ...[...text.matchAll(LINK)].map((m) => m[1]),
    ...[...text.matchAll(IMG_SRC)].map((m) => m[1]),
    // mdbook's SUMMARY.md lists chapters without markdown link syntax being
    // meaningful for the file it sits in, but the paths are still real.
    ...(rel === 'docs/SUMMARY.md' ? [...text.matchAll(SUMMARY)].map((m) => m[1]) : []),
  ];

  for (const raw of targets) {
    if (!raw) continue;
    if (/^(https?:|mailto:|#)/.test(raw)) continue;
    // A double-brace token is an unfilled URL from repo-links.json, not a path in
    // this repository. bench/harness/links.mjs is the gate that owns those, and it
    // reports them per token instead of 17 identical "does not exist" lines.
    if (raw.includes('{{')) continue;
    const target = raw.split('#')[0];
    if (!target) continue;
    checked++;
    const resolvedPath = target.startsWith('/')
      ? join(repoRoot, target)
      : resolve(base, target);
    if (!existsSync(resolvedPath)) {
      const line = text.slice(0, text.indexOf(raw)).split('\n').length;
      console.error(`${rel}:${line}: ${raw} -> does not exist`);
      broken++;
    }
  }
}

if (broken > 0) {
  console.error(`\n${broken} broken link(s) out of ${checked} relative links`);
  process.exit(1);
}
console.log(`ok   ${checked} relative links resolve`);
