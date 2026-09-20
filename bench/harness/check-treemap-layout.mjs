// Property check for the treemap layout, and an equivalence test against the
// copy inside the shipped report shell.
//
//   node bench/harness/check-treemap-layout.mjs
//
// What this catches, in order of how badly it bit us:
//   1. overlapping rectangles - a squarify that double-places a row still looks
//      fine in a thumbnail, which is how the first README image got 39 overlaps
//   2. rectangles that leave the canvas, or leave a visible hole in it
//   3. areas that do not match the sizes, i.e. a lie about proportions
//   4. the shell's inlined copy drifting from this one
//
// Cases are generated rather than hard-coded, because the failure depends on the
// aspect ratio and the size distribution: a layout that is fine for 10 square
// items in a wide canvas breaks for 40 items with one dominant.

import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { MIN_DIMENSION, squarify } from './treemap-layout.mjs';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

// Deterministic PRNG: a failing case has to be reproducible from its seed.
function rng(seed) {
  let s = seed >>> 0;
  return () => {
    s ^= s << 13; s >>>= 0;
    s ^= s >> 17;
    s ^= s << 5; s >>>= 0;
    return s / 4294967296;
  };
}

function makeItems(n, rand, shape) {
  const items = [];
  for (let i = 0; i < n; i++) {
    let size;
    switch (shape) {
      case 'uniform': size = 1 + rand() * 100; break;
      case 'zipf': size = 1 / (i + 1); break;
      case 'one-huge': size = i === 0 ? 5000 : 1 + rand() * 10; break;
      case 'two-huge': size = i < 2 ? 4000 : 1 + rand() * 10; break;
      case 'tiny': size = 1 + rand() * 0.01; break;
      default: size = 1 + rand() * 1000;
    }
    items.push({ name: `item-${i}`, size });
  }
  return items.sort((a, b) => b.size - a.size);
}

function checkCase(items, w, h, label, failures) {
  const rects = squarify(items, 0, 0, w, h);

  // The layout stops once the remaining strip is thinner than half a pixel:
  // on a 1x1 canvas with 7 items, most cells would be sub-pixel and invisible,
  // so dropping them is correct and a rectangle you cannot see is not worth
  // emitting. What must hold is that the dropped ones are the *smallest* (the
  // tail of the size-sorted list), never an arbitrary subset.
  const totalSize = items.reduce((s, i) => s + i.size, 0);
  const placedNames = new Set(rects.map((r) => r.node.name));
  const firstMissing = items.findIndex((i) => !placedNames.has(i.name));
  if (firstMissing !== -1) {
    const dropped = items.slice(firstMissing);
    const placed = items.slice(0, firstMissing);
    if (placed.length !== rects.length) {
      failures.push(`${label}: placed ${rects.length} rects but the kept prefix has ${placed.length}`);
      return;
    }
    if (dropped.length > 0 && !droppedAreSubPixel(dropped, totalSize, w, h)) {
      failures.push(
        `${label}: dropped ${dropped.length} of ${items.length} items, but at least one could still have been ${MIN_DIMENSION}px thick`,
      );
      return;
    }
  } else if (rects.length !== items.length) {
    failures.push(`${label}: laid out ${rects.length} rectangles for ${items.length} items`);
    return;
  }

  const area = w * h;
  const EPS = 1e-6;

  for (const r of rects) {
    if (r.w < -EPS || r.h < -EPS) {
      failures.push(`${label}: negative rectangle for ${r.node.name} (${r.w}x${r.h})`);
      return;
    }
    if (r.x < -EPS || r.y < -EPS || r.x + r.w > w + EPS || r.y + r.h > h + EPS) {
      failures.push(
        `${label}: ${r.node.name} at (${r.x.toFixed(2)},${r.y.toFixed(2)}) ${r.w.toFixed(2)}x${r.h.toFixed(2)} escapes ${w}x${h}`,
      );
      return;
    }
  }

  for (let i = 0; i < rects.length; i++) {
    for (let j = i + 1; j < rects.length; j++) {
      const a = rects[i];
      const b = rects[j];
      const dx = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
      const dy = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
      if (dx > 1e-6 && dy > 1e-6) {
        failures.push(
          `${label}: ${a.node.name} and ${b.node.name} overlap by ${dx.toFixed(2)}x${dy.toFixed(2)}`,
        );
        return;
      }
    }
  }

  const total = items.reduce((s, i) => s + i.size, 0);
  const covered = rects.reduce((s, r) => s + r.w * r.h, 0);

  // Full coverage is required whenever nothing was dropped.
  if (rects.length === items.length && Math.abs(covered - area) / area > 1e-6) {
    failures.push(
      `${label}: covers ${((covered / area) * 100).toFixed(4)}% of the canvas, expected 100%`,
    );
    return;
  }

  // Areas must be proportional to the sizes: that is the whole promise of a
  // treemap, and a layout bug can satisfy the no-overlap check while lying here.
  for (const r of rects) {
    const expected = (r.node.size / total) * area;
    const actual = r.w * r.h;
    if (expected > 0 && Math.abs(actual - expected) / expected > 1e-6) {
      failures.push(
        `${label}: ${r.node.name} area ${actual.toFixed(3)} but its size implies ${expected.toFixed(3)}`,
      );
      return;
    }
  }
}

/**
 * True when every dropped item provably could not have been `MIN_DIMENSION`
 * thick in either direction.
 *
 * A cell of area A inside a canvas whose longest side is L can only be at least
 * `MIN_DIMENSION` thick in both directions if A >= MIN_DIMENSION * L. So
 * A < MIN * L proves it was sub-pixel somewhere - which is exactly the case the
 * guard is allowed to discard.
 */
function droppedAreSubPixel(dropped, total, w, h, items) {
  const area = w * h;
  const longest = Math.max(w, h);
  const budget = MIN_DIMENSION * longest;
  return dropped.every((item) => {
    const cell = (item.size / total) * area;
    return cell < budget;
  });
}

const failures = [];
let cases = 0;

const shapes = ['uniform', 'zipf', 'one-huge', 'two-huge', 'tiny', 'skewed'];
const counts = [1, 2, 3, 7, 17, 64, 250, 1000];
const canvases = [
  [880, 440],
  [440, 880],
  [100, 100],
  [1000, 20],
  [20, 1000],
  [1, 1],
  [37, 613],
];

for (const shape of shapes) {
  for (const n of counts) {
    for (const [w, h] of canvases) {
      for (const seed of [1, 7, 12345]) {
        const rand = rng(seed);
        const items = makeItems(n, rand, shape);
        cases++;
        checkCase(items, w, h, `${shape}/n=${n}/${w}x${h}/seed=${seed}`, failures);
      }
    }
  }
}

// Degenerate inputs must return nothing rather than throwing or laying out junk.
for (const [items, w, h, why] of [
  [[], 100, 100, 'no items'],
  [[{ name: 'a', size: 0 }], 100, 100, 'all zero'],
  [[{ name: 'a', size: 5 }], 0, 100, 'zero width'],
  [[{ name: 'a', size: 5 }], 100, -1, 'negative height'],
]) {
  const rects = squarify(items, 0, 0, w, h);
  if (rects.length !== 0) {
    failures.push(`degenerate (${why}): returned ${rects.length} rectangles`);
  }
  cases++;
}

/*
 * Equivalence with the shell's inlined copy.
 *
 * The report must stay a single offline file, so `assets/report/shell.js` keeps
 * its own `squarify`. Rather than trust a comment saying "keep these in sync",
 * this extracts that function from the shipped source and compares the layouts
 * it produces against the module's, on real inputs. If someone edits one and not
 * the other, the README image and the product stop agreeing, and this fails.
 */
const shellPath = join(repoRoot, 'assets', 'report', 'shell.js');
const shellSource = readFileSync(shellPath, 'utf8');
const start = shellSource.indexOf('function squarify(');
if (start === -1) {
  failures.push('could not find squarify() in assets/report/shell.js');
} else {
  // Brace-match to the end of the function, so this does not depend on how the
  // rest of the file is formatted.
  let depth = 0;
  let end = -1;
  for (let i = shellSource.indexOf('{', start); i < shellSource.length; i++) {
    if (shellSource[i] === '{') depth++;
    else if (shellSource[i] === '}') {
      depth--;
      if (depth === 0) { end = i + 1; break; }
    }
  }
  const fnSource = shellSource.slice(start, end);
  // eslint-disable-next-line no-new-func
  const shellSquarify = new Function(`${fnSource}; return squarify;`)();

  const realItems = [
    { name: 'pkg-a', size: 4_800_000 },
    { name: 'pkg-b', size: 1_900_000 },
    { name: 'pkg-c', size: 900_000 },
    { name: 'pkg-d', size: 420_000 },
    { name: 'pkg-e', size: 210_000 },
    { name: 'pkg-f', size: 90_000 },
    { name: 'pkg-g', size: 40_000 },
    { name: 'pkg-h', size: 11_000 },
  ];
  for (const [w, h] of [
    [880, 440], [440, 880], [100, 100], [1200, 300], [300, 1200], [613, 37],
  ]) {
    const mine = squarify(realItems, 0, 0, w, h);
    const theirs = shellSquarify(realItems, 0, 0, w, h);
    cases++;
    if (mine.length !== theirs.length) {
      failures.push(`shell copy differs at ${w}x${h}: ${mine.length} vs ${theirs.length} rects`);
      continue;
    }
    for (let i = 0; i < mine.length; i++) {
      for (const k of ['x', 'y', 'w', 'h']) {
        if (Math.abs(mine[i][k] - theirs[i][k]) > 1e-9) {
          failures.push(
            `shell copy differs at ${w}x${h} rect ${i} (${mine[i].node.name}) .${k}: ${mine[i][k]} vs ${theirs[i][k]}`,
          );
        }
      }
    }
  }
}

if (failures.length > 0) {
  console.error(`${failures.length} layout failure(s) out of ${cases} cases:`);
  for (const f of failures.slice(0, 20)) console.error(`  ${f}`);
  process.exit(1);
}
console.log(`ok   ${cases} layout cases: no overlaps, full coverage, areas proportional`);
console.log('ok   assets/report/shell.js squarify matches bench/harness/treemap-layout.mjs');