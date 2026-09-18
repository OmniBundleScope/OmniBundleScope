// Render a report treemap to a standalone SVG, for the README and the docs.
//
//   node bench/harness/render-treemap-svg.mjs \
//     --stats bench/fixtures/artifacts/webpack/marked/stats.json \
//     --out docs/assets/treemap-marked.svg --width 880
//
// Why a committed script instead of a screenshot: a screenshot goes stale the
// moment the shell changes, and nobody can tell whether the numbers in it were
// real. This runs the shipped binary on a pinned fixture, so the image in the
// README is reproducible and its labels are the tool's actual output. It also
// means the README shows the *data* rather than a flattering crop of it.
//
// The layout is the same squarified treemap the HTML shell uses, written out
// longhand because SVG needs rectangles, not a canvas.

import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');
const binary = process.env.OB_BINARY ?? join(repoRoot, 'target', 'release', 'omnibundle.exe');

const arg = (flag, dflt = null) => {
  const i = process.argv.indexOf(flag);
  return i > -1 ? process.argv[i + 1] : dflt;
};

const stats = arg('--stats');
const out = arg('--out');
const width = Number(arg('--width', 880));
const height = Number(arg('--height', 440));
const dimension = arg('--dimension', 'package');
const label = arg('--label', null);

if (!stats || !out) {
  console.error('usage: node render-treemap-svg.mjs --stats <stats.json> --out <file.svg>');
  process.exit(2);
}

const run = spawnSync(binary, [stats, '--mode', 'json', '--dims', dimension], {
  encoding: 'buffer',
  maxBuffer: 1 << 30,
});
if (run.status !== 0) {
  console.error(`omnibundle exited ${run.status}: ${run.stderr?.toString()}`);
  process.exit(1);
}
const payload = JSON.parse(run.stdout.toString('utf8'));
const tree = payload.trees.find((t) => t.dimension === dimension)?.tree;
if (!tree) {
  console.error(`no ${dimension} tree in the payload`);
  process.exit(3);
}

// The report's tree is asset -> group, which is the right shape for a UI you
// click through. A README image wants the other projection: every package in
// the build side by side, summed across assets, because "which dependency is
// costing me" is the question a reader brings. Both are real views of the same
// payload; `--tree asset` keeps the report's own shape.
function flattenByPackage() {
  const dim = payload.totals?.sizeDimension ?? 'stat';
  const sizes = new Map();
  for (const m of Object.values(payload.modules ?? {})) {
    const pkg = m.package ?? '(application code)';
    const bytes = m.sizes?.[dim] ?? m.sizes?.stat ?? 0;
    sizes.set(pkg, (sizes.get(pkg) ?? 0) + bytes);
  }
  return [...sizes.entries()]
    .map(([name, size]) => ({ name, size, module_count: 0 }))
    .filter((c) => c.size > 0)
    .sort((a, b) => b.size - a.size);
}

const groupBy = arg('--group-by', dimension);
const children =
  groupBy === 'asset'
    ? [...tree.children].sort((a, b) => b.size - a.size)
    : flattenByPackage();
const dimensionLabel = groupBy === 'asset' ? 'assets' : groupBy;

// Squarified treemap: Bruls/Huizing/van Wijk.
//
// This is the *same* algorithm as `assets/report/shell.js`, and deliberately so:
// the README image should be the product's rendering, not a lookalike. It is a
// copy rather than an import because the report has to stay a single file that
// opens from `file://`, which rules out a module reference. The two are held
// together by `check-treemap-svg.mjs`, which fails on any overlap or uncovered
// canvas area - a squarify that silently double-places a row still *looks*
// plausible in a thumbnail, which is exactly why it needs a geometric gate.
function squarify(items, x, y, w, h) {
  const out = [];
  const total = items.reduce((a, b) => a + b.size, 0);
  if (total <= 0 || w <= 0 || h <= 0) return out;

  const scale = (w * h) / total;
  const rest = items.slice();
  let rect = { x, y, w, h };

  const worst = (row, side) => {
    const s = row.reduce((a, b) => a + b.size * scale, 0);
    if (s <= 0) return Infinity;
    const mx = row.reduce((a, b) => Math.max(a, b.size * scale), 0);
    const mn = row.reduce((a, b) => Math.min(a, b.size * scale), Infinity);
    return Math.max((side * side * mx) / (s * s), (s * s) / (side * side * mn));
  };

  while (rest.length) {
    const vertical = rect.w >= rect.h; // lay the row along the shorter side
    const side = vertical ? rect.h : rect.w;
    const row = [];
    let best = Infinity;
    while (rest.length) {
      const candidate = worst(row.concat([rest[0]]), side);
      if (row.length && candidate > best) break;
      row.push(rest.shift());
      best = candidate;
    }

    const rowArea = row.reduce((a, b) => a + b.size * scale, 0);
    const thickness = rowArea / side;

    if (vertical) {
      let cy = rect.y;
      for (const it of row) {
        const hgt = (it.size * scale) / thickness;
        out.push({ node: it, x: rect.x, y: cy, w: thickness, h: hgt });
        cy += hgt;
      }
      rect = { x: rect.x + thickness, y: rect.y, w: rect.w - thickness, h: rect.h };
    } else {
      let cx = rect.x;
      for (const it of row) {
        const wid = (it.size * scale) / thickness;
        out.push({ node: it, x: cx, y: rect.y, w: wid, h: thickness });
        cx += wid;
      }
      rect = { x: rect.x, y: rect.y + thickness, w: rect.w, h: rect.h - thickness };
    }
    if (rect.w <= 0.5 || rect.h <= 0.5) break;
  }
  return out;
}

// Cap the rectangles so the image stays legible. The fold node says how many
// groups it stands for, so the picture never implies it shows everything.
const fold = process.argv.includes('--fold');
const top = children.slice(0, 40);
if (fold && children.length > top.length) {
  const rest = children.slice(top.length);
  top.push({
    name: `(+${rest.length} more)`,
    size: rest.reduce((s, c) => s + c.size, 0),
  });
}
const rects = squarify(top, 0, 0, width, height);

// One accent per group, all at the same saturation, with lightness carrying
// the rank so the biggest rectangles are the calmest. Same palette intent as
// `assets/report/shell.js`.
const PALETTE = [
  '#4c8dff', '#00b3a4', '#f2a33c', '#c86bff', '#ff6b6b',
  '#5ad1a0', '#ffd166', '#7aa2ff', '#e07be0', '#8ecae6',
];
const lighten = (hex, t) => {
  const n = parseInt(hex.slice(1), 16);
  const r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  const mix = (c) => Math.round(c + (255 - c) * t);
  return `#${((mix(r) << 16) | (mix(g) << 8) | mix(b)).toString(16).padStart(6, '0')}`;
};

const esc = (s) =>
  String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
const fmt = (n) => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.round(n / 1024)} KB`);

const parts = [];
parts.push(
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" font-family="ui-monospace,SFMono-Regular,Menlo,Consolas,monospace">`,
  `<rect width="${width}" height="${height}" fill="#0e1116"/>`,
);
rects.forEach((r, i) => {
  const base = PALETTE[i % PALETTE.length];
  const fill = lighten(base, Math.min(0.55, i * 0.045));
  const pad = 1;
  parts.push(
    `<rect x="${(r.x + pad).toFixed(1)}" y="${(r.y + pad).toFixed(1)}" width="${Math.max(0, r.w - pad * 2).toFixed(1)}" height="${Math.max(0, r.h - pad * 2).toFixed(1)}" fill="${fill}" fill-opacity="0.92"><title>${esc(r.node.name)}: ${fmt(r.node.size)}</title></rect>`,
  );
  // Label only where it fits: an unreadable clipped label is worse than none,
  // which is the same rule the HTML shell applies.
  const room = r.w > 74 && r.h > 26;
  if (room) {
    const name = r.node.name.length > Math.floor(r.w / 7.2) ? r.node.name.slice(0, Math.max(1, Math.floor(r.w / 7.2) - 1)) + '…' : r.node.name;
    parts.push(
      `<text x="${(r.x + 7).toFixed(1)}" y="${(r.y + 17).toFixed(1)}" fill="#0b0e13" font-size="12" font-weight="600">${esc(name)}</text>`,
    );
    if (r.h > 42) {
      parts.push(
        `<text x="${(r.x + 7).toFixed(1)}" y="${(r.y + 31).toFixed(1)}" fill="#0b0e13" font-size="11" fill-opacity="0.75">${fmt(r.node.size)}</text>`,
      );
    }
  }
});
parts.push(
  `<text x="12" y="${height - 12}" fill="#8b98a9" font-size="11">${esc(
    `${label ?? tree.name} 路 ${dimension} 路 ${payload.target} 路 ${payload.totals.module_count} modules 路 dimension ${payload.sizeDimension ?? 'stat'}`,
  )}</text>`,
  `</svg>`,
);

mkdirSync(dirname(resolve(out)), { recursive: true });
writeFileSync(out, `${parts.join('\n')}\n`);
console.log(`wrote ${out} (${rects.length} rectangles, ${tree.children.length} groups)`);
