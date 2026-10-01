// Verify the README's headline claims against the committed records.
//
//   node bench/harness/check-readme-numbers.mjs
//
// A token-by-token scan of every number in the README was the first attempt and
// it was the wrong design: it cannot tell a rounded 1.87 from a recorded 1.869,
// cannot see that "34x faster" is two records divided, and breaks on a German
// README that writes 63,6. So this checks the thing that actually matters - the
// comparison tables, cell by cell, against charts-data.json - and leaves prose
// and illustrative CLI transcripts alone.
//
// What it catches: a figure that was rounded the wrong way, a row swapped, a
// ratio that no longer matches its operands. What it does not catch: a
// mislabelled fixture, which is what the caption next to the table is for.

import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(fileURLToPath(import.meta.url), '..', '..', '..');
const data = JSON.parse(readFileSync(join(repoRoot, 'docs/assets/charts-data.json'), 'utf8'));

/**
 * Parse "1.87 s", "1,87 s", "2,295 MB", "2.295 MB", "2.24 GB" into a number.
 *
 * The separator is ambiguous across the four languages, and the rule below is
 * chosen from the shapes actually used in these files: `1.234.567` and `1,234`
 * are thousands, `63,6` is a German decimal, and a bare `1.87` is a decimal. A
 * German decimal with three digits would be read as thousands - none of the
 * published figures have one, and the alternative (a per-language parser table)
 * would be more machinery than the ambiguity is worth.
 */
function parseNumber(text) {
  if (/^\d{1,3}(\.\d{3})+$/.test(text)) return Number(text.replace(/\./g, ''));
  if (/^\d{1,3}(,\d{3})+$/.test(text)) return Number(text.replace(/,/g, ''));
  if (/^\d+,\d+$/.test(text)) return Number(text.replace(',', '.'));
  return Number(text);
}

/** Parse a quantity with its unit into a canonical {unit, value}. */
function parseQuantity(text) {
  const match = /([\d.,]+)\s*(ms|s|min|MB|GB)/.exec(text);
  if (!match) return null;
  const value = parseNumber(match[1]);
  if (!Number.isFinite(value)) return null;
  switch (match[2]) {
    case 'ms':
      return { unit: 's', value: value / 1000 };
    case 's':
      return { unit: 's', value };
    case 'min':
      return { unit: 's', value: value * 60 };
    case 'MB':
      return { unit: 'mb', value };
    case 'GB':
      return { unit: 'mb', value: value * 1024 };
    default:
      return null;
  }
}

/** True when `printed` is `actual` rounded the way a person would round it. */
function matchesRounded(printed, actual, tolerance) {
  return Math.abs(printed - actual) <= tolerance;
}

let failures = 0;
const checked = [];

for (const name of ['README.md', 'README.en.md', 'README.zh.md', 'README.ja.md', 'README.de.md']) {
  const text = readFileSync(join(repoRoot, name), 'utf8');
  const lines = text.split('\n');

  // The data rows do not name the tool; only the header does. So the table is
  // found by its header, and the rows after it are what gets checked.
  let inComparisonTable = false;

  for (const [index, line] of lines.entries()) {
    if (!line.trim().startsWith('|')) {
      inComparisonTable = false;
      continue;
    }
    // Drop the empty leading and trailing cells a markdown row always has, or
    // every column shifts by one and the check silently finds nothing.
    const cells = line.split('|').map((c) => c.trim()).slice(1, -1);

    if (/webpack-bundle-analyzer/i.test(line) && /omnibundlescope/i.test(line)) {
      inComparisonTable = true;
      continue;
    }
    if (!inComparisonTable || cells.length < 4) continue;

    const [label, ours, theirs, ratioCell] = cells;
    if (!/\d/.test(label) || !/[\d]/.test(ours) || !/[\d]/.test(theirs)) continue;

    // Match on the first two words ("363 MB", "1 GB") rather than the whole
    // label, which differs per language.
    const key = Object.keys(data.stats_pipeline).find((k) =>
      label.includes(k.split(' ').slice(0, 2).join(' ')),
    );
    if (!key) continue;
    const record = data.stats_pipeline[key];

    const ourCells = ours.match(/[\d.,]+\s*(?:ms|s|min|MB|GB)/g) ?? [];
    const theirCells = theirs.match(/[\d.,]+\s*(?:ms|s|min|MB|GB)/g) ?? [];
    if (ourCells.length < 2 || theirCells.length < 2) {
      console.error(`${name}:${index + 1}: expected a time and a memory in both cells`);
      failures++;
      continue;
    }

    const pairs = [
      ['our time', ourCells[0], { unit: 's', value: record.ob_s }],
      ['our memory', ourCells[1], { unit: 'mb', value: record.ob_mb }],
      ['reference time', theirCells[0], { unit: 's', value: record.ref_s }],
      ['reference memory', theirCells[1], { unit: 'mb', value: record.ref_mb }],
    ];
    for (const [what, printedText, expected] of pairs) {
      const printed = parseQuantity(printedText);
      if (!printed) {
        console.error(`${name}:${index + 1}: cannot read ${what} from ${JSON.stringify(printedText)}`);
        failures++;
        continue;
      }
      if (printed.unit !== expected.unit) {
        console.error(
          `${name}:${index + 1}: ${what} is in ${printed.unit} but the record is in ${expected.unit}`,
        );
        failures++;
        continue;
      }
      // 0.5% of the value, or 1% of a second, whichever is larger: enough to
      // allow "1.87 s" for 1.869 and to reject "1.9 s".
      const tolerance = Math.max(expected.value * 0.005, expected.unit === 's' ? 0.01 : 1);
      if (!matchesRounded(printed.value, expected.value, tolerance)) {
        console.error(
          `${name}:${index + 1}: ${what} reads ${printed.value}${expected.unit} but the ` +
            `record says ${expected.value}${expected.unit} (${key})`,
        );
        failures++;
      }
    }

    // The ratio cell, if it states one.
    const ratioMatch = /([\d.,]+)\s*[x×倍倍]/.exec(ratioCell ?? '');
    if (ratioMatch) {
      const stated = Number(ratioMatch[1].replace(',', '.'));
      const timeRatio = record.ref_s / record.ob_s;
      const memRatio = record.ref_mb / record.ob_mb;
      const ok =
        Math.abs(stated - timeRatio) <= 1.1 ||
        Math.abs(stated - memRatio) <= 1.1 ||
        Math.abs(stated - timeRatio) / timeRatio <= 0.05 ||
        Math.abs(stated - memRatio) / memRatio <= 0.05;
      if (!ok) {
        console.error(
          `${name}:${index + 1}: ratio reads ${stated}x but the record gives ` +
            `${timeRatio.toFixed(1)}x on time and ${memRatio.toFixed(1)}x on memory`,
        );
        failures++;
      }
    }

    checked.push(`${name}:${index + 1} (${key})`);
  }
}

if (failures > 0) {
  console.error(`\n${failures} headline figure(s) disagree with docs/assets/charts-data.json`);
  process.exit(1);
}
if (checked.length === 0) {
  console.error('no comparison tables found - the check is not looking at anything');
  process.exit(1);
}
console.log(`ok   ${checked.length} comparison rows across ${new Set(checked.map((c) => c.split(' ')[0])).size} READMEs match charts-data.json`);
