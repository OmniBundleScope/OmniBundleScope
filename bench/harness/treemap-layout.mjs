// Squarified treemap layout: Bruls, Huizing & van Wijk (2000).
//
// One implementation, used by `check-treemap-layout.mjs`. The report shell keeps
// its own copy because the report has to remain a single file that opens from
// `file://`, which rules out a module reference; `check-treemap-layout.mjs`
// asserts the two agree. The README treemap is drawn by the Python renderer
// (`render-treemap.py`), which ports this function line for line - JavaScript
// cannot be shared with it, so the property check drives the shapes instead.
//
// Why it is worth a module at all: a treemap that lays out two rectangles on top
// of each other still looks plausible in a thumbnail. The first version of the
// README image did exactly that, and only a geometric check caught it.

/**
 * Thinnest strip the layout will emit, in pixels.
 *
 * Below this a rectangle cannot show a pixel of a label or a 1 px border, so
 * the layout stops rather than emitting invisible cells. Exported so the
 * property check can assert *why* items were dropped instead of guessing.
 */
export const MIN_DIMENSION = 0.5;

/**
 * @param {{name: string, size: number}[]} items  sorted by size descending
 * @returns {{node: object, x: number, y: number, w: number, h: number}[]}
 */
export function squarify(items, x, y, w, h) {
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
    if (rect.w <= MIN_DIMENSION || rect.h <= MIN_DIMENSION) break;
  }
  return out;
}