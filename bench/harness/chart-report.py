"""Print how each chart will read at README width.

    python bench/harness/chart-report.py

Reads the layout facts the generator recorded in `charts-data.json`. It does not
parse the SVGs: those store glyphs as paths and contain no text, so a checker
pointed at them finds nothing, prints nothing, and exits 0 - which is the worst
kind of check, because it looks like it passed.
"""

import json
import pathlib
import sys

DATA = pathlib.Path("docs/assets/charts-data.json")

if not DATA.exists():
    print(f"{DATA} is missing - run bench/harness/make-charts.py first")
    sys.exit(2)

figures = json.loads(DATA.read_text(encoding="utf-8")).get("figures")
if not figures:
    print("charts-data.json has no `figures` section; the generator did not record any")
    sys.exit(1)

FLOOR = 9.0
failures = 0
for chart, themes in sorted(figures.items()):
    for theme, facts in sorted(themes.items()):
        smallest = facts["smallest_px_at_880px_wide"]
        bad = smallest < FLOOR
        failures += bool(bad)
        print(
            f"{'SMALL' if bad else 'ok  '} {chart + '-' + theme:<28} "
            f"{facts['figure_width_pt']:>6.0f}pt wide  "
            f"{facts['labels']:>3} labels  "
            f"fonts {facts['font_pt_min']}-{facts['font_pt_max']}pt  "
            f"smallest {smallest}px"
        )

sys.exit(1 if failures else 0)