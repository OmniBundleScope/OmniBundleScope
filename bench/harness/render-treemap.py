#!/usr/bin/env python3
"""Render the README treemap: every package in a build, side by side.

    # the committed README figure
    node bench/harness/gen-stats.mjs 20050000 /tmp/treemap-stats.json
    python bench/harness/render-treemap.py \
      --stats /tmp/treemap-stats.json --top 40 \
      --label "synthetic monorepo fixture" \
      --out docs/assets/treemap-large.svg --width 880 --height 440

Why a committed script instead of a screenshot: a screenshot goes stale the
moment the shell changes, and nobody can tell whether the numbers in it were
real. This runs the shipped binary on a pinned fixture, so the image in the
README is reproducible and its labels are the tool's actual output. CI
regenerates it and fails on a diff (`.github/workflows/ci.yml`).

This is the Python half of `bench/harness/make-charts.py`; together they are the
only two things in the repository that draw a figure, and both are gated the same
way. It replaces a Node renderer that could not be checked, and that was wrong:
it read `module.package` when the payload holds `{name, path, version}`, so every
group was keyed by an object identity and the committed image labelled all forty
rectangles "[object Object]: 3 KB" while covering 0.3 % of the canvas.

Two contract choices, both because the alternative was silently wrong:

  * the byte value per module follows `Module::sizes.effective()` - ground
    truth, then measurement, then the bundler's claim - and the footer names the
    dimension that was *used*, not the one requested;
  * the layout is the same squarify as `bench/harness/treemap-layout.mjs`, which
    the property check drives (1,018 cases) and which the report shell keeps an
    inlined copy of. A treemap that double-places a row still looks plausible in
    a thumbnail, so the geometry is verified before anything is written.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]

# Thinnest strip the layout will emit, in pixels. Mirrors `treemap-layout.mjs`;
# below this a rectangle cannot show a label or a border, so the layout stops
# rather than emitting invisible cells.
MIN_DIMENSION = 0.5

# One accent per group, all at the same saturation, with lightness carrying the
# rank so the biggest rectangles are the calmest. Same palette intent as the
# report shell (`assets/report/shell.js`).
PALETTE = [
    "#4c8dff", "#00b3a4", "#f2a33c", "#c86bff", "#ff6b6b",
    "#5ad1a0", "#ffd166", "#7aa2ff", "#e07be0", "#8ecae6",
]
BACKGROUND = "#0e1116"
LABEL_INK = "#0b0e13"
FOOTER_INK = "#8b98a9"

# A group needs at least this room before its name is drawn. An unreadable,
# clipped label is worse than no label - the same rule the HTML shell applies.
LABEL_MIN_W = 74
LABEL_MIN_H = 26
LABEL_ROW_H = 42

# Average advance width of the monospace stack, in em. Used to decide where a
# name has to be ellipsised; over-estimating is the safe direction.
ADVANCE_EM = 0.62


# ------------------------------------------------------------------ layout


def squarify(items: list[dict], x: float, y: float, w: float, h: float) -> list[dict]:
    """Squarified treemap: Bruls, Huizing & van Wijk (2000).

    A line-by-line port of `bench/harness/treemap-layout.mjs`. It is duplicated
    rather than shared because one is JavaScript and one is Python, and the
    property check drives both so the two cannot drift apart unnoticed.
    """
    out: list[dict] = []
    total = sum(item["size"] for item in items)
    if total <= 0 or w <= 0 or h <= 0:
        return out

    scale = (w * h) / total
    rest = list(items)
    rect = [x, y, w, h]

    def worst(row: list[dict], side: float) -> float:
        s = sum(item["size"] * scale for item in row)
        if s <= 0:
            return float("inf")
        mx = max(item["size"] * scale for item in row)
        mn = min(item["size"] * scale for item in row)
        return max((side * side * mx) / (s * s), (s * s) / (side * side * mn))

    while rest:
        # Lay the row along the shorter side.
        vertical = rect[2] >= rect[3]
        side = rect[3] if vertical else rect[2]
        row: list[dict] = []
        best = float("inf")
        while rest:
            candidate = worst(row + [rest[0]], side)
            if row and candidate > best:
                break
            row.append(rest.pop(0))
            best = candidate

        row_area = sum(item["size"] * scale for item in row)
        thickness = row_area / side

        if vertical:
            cursor = rect[1]
            for item in row:
                height = (item["size"] * scale) / thickness
                out.append({"node": item, "x": rect[0], "y": cursor, "w": thickness, "h": height})
                cursor += height
            rect = [rect[0] + thickness, rect[1], rect[2] - thickness, rect[3]]
        else:
            cursor = rect[0]
            for item in row:
                width = (item["size"] * scale) / thickness
                out.append({"node": item, "x": cursor, "y": rect[1], "w": width, "h": thickness})
                cursor += width
            rect = [rect[0], rect[1] + thickness, rect[2], rect[3] - thickness]

        if rect[2] <= MIN_DIMENSION or rect[3] <= MIN_DIMENSION:
            break

    return out


def assert_valid(rects: list[dict], width: float, height: float, name: str) -> None:
    """Refuse to write a treemap that overlaps itself or leaves a hole.

    The same properties `check-treemap-svg.mjs` asserts on the committed file,
    asserted here as well: a generator that writes a bad figure and a checker
    that finds it later is strictly worse than a generator that refuses.
    """
    if not rects:
        raise SystemExit(f"{name}: the layout produced no rectangles")

    eps = 1e-6
    for rect in rects:
        if rect["w"] < -eps or rect["h"] < -eps:
            raise SystemExit(f"{name}: negative rectangle for {rect['node']['name']}")
        if (
            rect["x"] < -eps
            or rect["y"] < -eps
            or rect["x"] + rect["w"] > width + eps
            or rect["y"] + rect["h"] > height + eps
        ):
            raise SystemExit(
                f"{name}: {rect['node']['name']} at ({rect['x']:.1f},{rect['y']:.1f}) "
                f"{rect['w']:.1f}x{rect['h']:.1f} escapes {width:.0f}x{height:.0f}"
            )

    for i, a in enumerate(rects):
        for b in rects[i + 1:]:
            dx = min(a["x"] + a["w"], b["x"] + b["w"]) - max(a["x"], b["x"])
            dy = min(a["y"] + a["h"], b["y"] + b["h"]) - max(a["y"], b["y"])
            if dx > 1e-6 and dy > 1e-6:
                raise SystemExit(
                    f"{name}: {a['node']['name']} and {b['node']['name']} overlap by "
                    f"{dx:.2f}x{dy:.2f} - fix the layout, not the check"
                )

    covered = sum(rect["w"] * rect["h"] for rect in rects)
    if covered / (width * height) < 0.97:
        raise SystemExit(
            f"{name}: the rectangles cover {covered / (width * height):.1%} of the canvas; "
            "a treemap that leaves a quarter of the frame empty is a bug"
        )


# ------------------------------------------------------------------- input


def find_binary(explicit: str | None) -> pathlib.Path:
    """Locate the shipped CLI, honouring the env var the Node harness used."""
    if explicit:
        candidate = pathlib.Path(explicit)
        if not candidate.exists():
            raise SystemExit(f"--binary {explicit} does not exist")
        return candidate

    from_env = os.environ.get("OB_BINARY")
    if from_env:
        return pathlib.Path(from_env)

    stem = REPO / "target" / "release" / "omnibundle"
    for candidate in (stem.with_suffix(".exe"), stem):
        if candidate.exists():
            return candidate
    raise SystemExit(
        "no omnibundle binary found: build it with `cargo build --release`, "
        "or pass --binary / set OB_BINARY"
    )


def run_payload(binary: pathlib.Path, stats: pathlib.Path) -> dict:
    """Ask the tool for its own graph rather than re-parsing the stats file.

    The image must show what the tool reports, not what a second parser thinks
    the stats file means; anything else is a picture of a different program.
    """
    result = subprocess.run(
        [str(binary), str(stats), "--mode", "json", "--dims", "package"],
        capture_output=True,
    )
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip()
        raise SystemExit(f"{binary} exited {result.returncode}: {detail}")
    # `omnibundle --mode json` writes a UTF-8 BOM, which `json.loads` rejects.
    return json.loads(result.stdout.decode("utf-8-sig"))


def size_of(sizes: dict) -> tuple[int, str]:
    """`Module::sizes.effective()`: ground truth, then measurement, then claim.

    Kept identical to `crates/omnibundle-core/src/model.rs` so the picture and
    the product cannot disagree about what a module weighs.
    """
    attributed = sizes.get("attributed")
    if attributed:
        return int(attributed), "attributed"
    parsed = sizes.get("parsed")
    if parsed:
        return int(parsed), "parsed"
    return int(sizes.get("stat") or 0), "stat"


def groups_by_package(payload: dict) -> list[dict]:
    """Sum every module's bytes into its package, across every asset.

    This is the projection a reader brings to a README image: "which dependency
    is costing me". The report's own tree is asset -> group, which is the right
    shape for a UI you click through and the wrong one for a single picture.
    """
    totals: dict[str, int] = {}
    counts: dict[str, int] = {}
    for module in (payload.get("modules") or {}).values():
        package = module.get("package") or {}
        name = package.get("name") or "<app>"
        byte_count, _ = size_of(module.get("sizes") or {})
        totals[name] = totals.get(name, 0) + byte_count
        counts[name] = counts.get(name, 0) + 1

    children = [
        {"name": name, "size": size, "module_count": counts[name]}
        for name, size in totals.items()
        if size > 0
    ]
    # size desc, then name asc: deterministic, so the committed SVG is stable.
    children.sort(key=lambda c: (-c["size"], c["name"]))
    return children


def size_dimension(payload: dict) -> str:
    """The dimension the bytes are in, named rather than assumed.

    `effective()` is per module, but a build has one answer in practice and the
    footer has to say which; drawing `stat` bytes under a `parsed` label is how
    the previous image ended up claiming a measurement it did not have.
    """
    seen = {
        size_of(module.get("sizes") or {})[1]
        for module in (payload.get("modules") or {}).values()
    }
    for candidate in ("attributed", "parsed", "stat"):
        if candidate in seen:
            return candidate
    return "stat"


# ------------------------------------------------------------------ output


def mix_toward_white(hex_colour: str, t: float) -> str:
    n = int(hex_colour[1:], 16)
    channels = ((n >> 16) & 255, (n >> 8) & 255, n & 255)
    mixed = tuple(round(c + (255 - c) * t) for c in channels)
    return "#" + "".join(f"{c:02x}" for c in mixed)


def fmt_bytes(n: float) -> str:
    if n >= 1048576:
        return f"{n / 1048576:.1f} MB"
    return f"{round(n / 1024)} KB"


def esc(text: str) -> str:
    return str(text).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def render(
    children: list[dict],
    payload: dict,
    label: str | None,
    width: int,
    height: int,
    top: int,
    fold: bool,
) -> tuple[str, int, int, str]:
    shown = children if top <= 0 else children[:top]
    if fold and len(children) > len(shown):
        rest = children[len(shown):]
        shown = shown + [
            {
                "name": f"(+{len(rest)} more)",
                "size": sum(c["size"] for c in rest),
                "module_count": sum(c.get("module_count") or 0 for c in rest),
            }
        ]

    rects = squarify(shown, 0, 0, width, height)
    assert_valid(rects, width, height, "treemap (package)")

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" '
        f'font-family="ui-monospace,SFMono-Regular,Menlo,Consolas,monospace">',
        f'<rect width="{width}" height="{height}" fill="{BACKGROUND}"/>',
    ]

    for index, rect in enumerate(rects):
        fill = mix_toward_white(PALETTE[index % len(PALETTE)], min(0.55, index * 0.045))
        node = rect["node"]
        # The 1 px grid is a stroke, not padding: padding shrinks the rectangles,
        # and a checker that sums their areas then reports a 4 % hole - which is
        # why the previous image could not have passed `check-treemap-svg.mjs`
        # however it was drawn.
        parts.append(
            f'<rect x="{rect["x"]:.1f}" y="{rect["y"]:.1f}" '
            f'width="{rect["w"]:.1f}" height="{rect["h"]:.1f}" '
            f'fill="{fill}" fill-opacity="0.92" '
            f'stroke="{BACKGROUND}" stroke-width="2">'
            f'<title>{esc(node["name"])}: {fmt_bytes(node["size"])}</title></rect>'
        )
        if rect["w"] > LABEL_MIN_W and rect["h"] > LABEL_MIN_H:
            name = str(node["name"])
            room = int(rect["w"] / (12 * ADVANCE_EM))
            if len(name) > room:
                name = name[: max(1, room - 1)] + "\u2026"
            parts.append(
                f'<text x="{rect["x"] + 7:.1f}" y="{rect["y"] + 17:.1f}" '
                f'fill="{LABEL_INK}" font-size="12" font-weight="600">{esc(name)}</text>'
            )
            if rect["h"] > LABEL_ROW_H:
                parts.append(
                    f'<text x="{rect["x"] + 7:.1f}" y="{rect["y"] + 31:.1f}" '
                    f'fill="{LABEL_INK}" font-size="11" fill-opacity="0.75">'
                    f'{fmt_bytes(node["size"])}</text>'
                )

    totals = payload.get("totals") or {}
    # The input's own file name is deliberately *not* here. It is the one part
    # that depends on where the file happens to live - `stats.json` locally,
    # `treemap-stats.json` in CI - and a figure that changes when it is
    # regenerated somewhere else is a figure the CI diff gate cannot check.
    footer = " \u00b7 ".join(
        [
            label or "bundle",
            "package",
            f'{totals.get("asset_count", 0)} assets',
            f'{totals.get("module_count", 0)} modules',
            f"{len(children)} groups",
            f"dimension {size_dimension(payload)}",
        ]
    )
    parts.append(f'<text x="12" y="{height - 12}" fill="{FOOTER_INK}" font-size="11">{esc(footer)}</text>')
    parts.append("</svg>")
    return "\n".join(parts) + "\n", len(rects), len(shown), footer


def main() -> int:
    parser = argparse.ArgumentParser(description="Render the OmniBundle README treemap to SVG.")
    parser.add_argument("--stats", required=True, help="stats.json (or metafile.json) to analyse")
    parser.add_argument("--out", required=True, help="SVG to write")
    parser.add_argument("--width", type=int, default=880)
    parser.add_argument("--height", type=int, default=440)
    parser.add_argument("--top", type=int, default=40, help="rectangles to keep; 0 for all")
    parser.add_argument(
        "--fold", action="store_true",
        help="fold the tail into one node, so the picture never implies it shows everything",
    )
    parser.add_argument("--label", default=None, help="what the fixture is, drawn in the footer")
    parser.add_argument("--binary", default=None)
    args = parser.parse_args()

    stats = pathlib.Path(args.stats)
    if not stats.exists():
        raise SystemExit(f"--stats {args.stats} does not exist")

    binary = find_binary(args.binary)
    payload = run_payload(binary, stats)

    children = groups_by_package(payload)
    if not children:
        raise SystemExit(f"no packages with a positive size in {stats}")

    svg, rendered, groups, footer = render(
        children, payload, args.label, args.width, args.height, args.top, args.fold
    )

    out = pathlib.Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    # newline='\n' so the file is identical on Windows and on the Linux CI runner.
    out.write_text(svg, encoding="utf-8", newline="\n")
    print(
        f"wrote {out} ({rendered} rectangles, {groups} of {len(children)} package groups, "
        f"dimension {size_dimension(payload)})"
    )
    print(f"  {footer}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
