#!/usr/bin/env python3
"""Generate the performance charts for the README and the docs.

    python bench/harness/make-charts.py

Why a script and not checked-in images: a chart nobody can regenerate is a
picture of a benchmark that used to be true. This reads the same harness records
the docs cite and writes a sidecar JSON next to the SVGs listing every plotted
number with its provenance, so a reader can diff the chart against the record
instead of trusting it.

Two themes are emitted for every chart and the README picks between them with a
`prefers-color-scheme` <picture>, because a chart designed only for light mode is
an unreadable grey block on a dark page.

Design rules, applied rather than assumed:
  - the tool being replaced recedes (slate), the subject is the only saturated
    colour, so the eye lands on the comparison and not on the decoration;
  - every value is labelled directly, so there is no legend to decode and no
    axis to squint at;
  - log axes wherever the values span orders of magnitude, because a linear axis
    would render 46 ms as a hairline next to 562 s and quietly hide the result;
  - the ratio between the pair is stated in words, because that is the claim;
  - the fixture class and the protocol are printed on the figure. A benchmark
    without its provenance is a marketing asset, not a measurement.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.ticker import FuncFormatter  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parents[2]
OUT = REPO / "docs" / "assets"

# ---------------------------------------------------------------- data
# Every number below is transcribed from a committed record under
# bench/results/ or from docs/en/01-evidence.md, which cites the session each
# came from. The sidecar written next to the SVGs repeats this provenance so the
# figure can be audited without reading this script.

# (label, OmniBundle seconds, reference tool seconds, reference name)
STATS = {
    "363 MB stats, 154,379 modules": {
        "ob_s": 1.869,
        "ref_s": 63.617,
        "ref": "webpack-bundle-analyzer 4.10.2",
    },
    "1 GB stats, 445,602 modules": {
        "ob_s": 6.140,
        "ref_s": 176.3,
        "ref": "webpack-bundle-analyzer 4.10.2",
    },
}
STATS_MEM = {
    "363 MB stats, 154,379 modules": {"ob_mb": 126.8, "ref_mb": 2295.0},
    "1 GB stats, 445,602 modules": {"ob_mb": 375.9, "ref_mb": 1437.0},
}

# Source map attribution, by number of sources.
#
# The first point is the real preact build, which has twelve sources - not the
# "1,000" this used to claim. There is no 1,000-source input anywhere in the
# records: the synthetic fixtures start at 10k, and the real ones are 11-12
# sources. The old row put the preact fixture at x=1,000, took its time from a
# value that matched no record (4.6 ms when WS-3 says 7 ms) and its memory from
# the *minified* preact run while taking the time from the readable one. Every
# number below is now a value with a record behind it.
MAP_POINTS = [
    # sources, omnibundle seconds, sme seconds, omnibundle MB, sme MB
    (12, 0.007, 0.252, 4.7, 31.0),
    (10_000, 0.046, 18.380, 6.0, 277.0),
    (50_000, 0.209, 562.269, 67.0, 642.0),
]

# Layout facts per figure, recorded into the sidecar so a checker can assert them
# without re-rendering. The shipped SVGs hold glyphs as paths and therefore
# contain no text at all: measured against them, a checker finds nothing and
# reports success, which is worse than not checking.
figure_stats: dict = {}

PROVENANCE = {    "omni_stats": "bench/results/b3-full-pipeline-2026-09-16.json (median of 3) and b4-full-pipeline-2026-09-16.json (median of 3)",
    "omni_map": "bench/results/ws3-sourcemap-ingest-2026-09-21.json (time, median of 3 for 10k and 50k; memory for 50k) and bench/results/ws3b-sourcemap-memory-2026-09-30.json (memory for the 12-source and 10k points)",
    "wba_363mb": "bench/results/ws1-stats-ingest-2026-09-03.json, 63,617 ms / 2,295 MB on the corrected 363 MB fixture - also quoted in docs/en/01-evidence.md \u00a72b",
    "wba_1gb": "docs/en/01-evidence.md \u00a72, 176.3 s / 1,437 MB - baseline session, same reference machine, not the same run as the 363 MB row",
    "sme": "bench/results/ws-s-sme-baseline-2026-09-03.json. The 12-source row is the real preact build (252 ms / 31 MB, median of 3); 10k is a median of 3; 50k is a single 9.4-minute run, because three runs would have taken half an hour",
    "targets": "docs/en/04-benchmark-plan.md - B3 <= 5 s / <= 200 MB, B4 <= 15 s / <= 400 MB, B5 <= 1 s, B8 < 500 MB",
}

# ---------------------------------------------------------------- themes

THEMES = {
    "light": {
        "bg": "#ffffff",
        "surface": "#f6f8fb",
        "ink": "#0f172a",
        "muted": "#64748b",
        "grid": "#e2e8f0",
        "accent": "#2563eb",
        "accent_soft": "#93b4fb",
        "ref": "#a3b0c2",
        "ref_ink": "#475569",
    },
    "dark": {
        "bg": "#0e1116",
        "surface": "#151a21",
        "ink": "#e6edf3",
        "muted": "#8b98a9",
        "grid": "#222a35",
        "accent": "#4c8dff",
        "accent_soft": "#2b4d80",
        "ref": "#4d5866",
        "ref_ink": "#9aa7b6",
    },
}


def style_axes(ax, theme, *, xgrid=True):
    ax.set_facecolor(theme["bg"])
    ax.xaxis.grid(True, color=theme["grid"], linewidth=0.8, zorder=0)
    ax.yaxis.grid(False)
    ax.set_axisbelow(True)
    for side in ("top", "right", "left"):
        ax.spines[side].set_visible(False)
    ax.spines["bottom"].set_color(theme["grid"])
    ax.tick_params(colors=theme["muted"], labelsize=9, length=0)
    if not xgrid:
        ax.xaxis.grid(False)
        ax.yaxis.grid(True, color=theme["grid"], linewidth=0.8, zorder=0)


def human_time(seconds: float) -> str:
    if seconds >= 60:
        return f"{seconds / 60:.1f} min" if seconds < 600 else f"{seconds / 60:.0f} min"
    if seconds >= 1:
        return f"{seconds:.2f} s".replace(".00 ", " ")
    # One decimal below ten milliseconds: rounding 7.0 ms to "7 ms" is fine, but
    # the same rule has to hold for whatever the fastest measurement becomes, and
    # a value like 4.6 ms would print as a 9 % error without it - on the number a
    # reader is most likely to quote back at you.
    if seconds >= 0.01:
        return f"{seconds * 1000:.0f} ms"
    return f"{seconds * 1000:.1f} ms"


def human_mb(mb: float) -> str:
    if mb >= 1024:
        return f"{mb / 1024:.2f} GB"
    # One decimal below ten megabytes for the same reason human_time keeps one
    # below ten milliseconds: the smallest value in the set is the one a reader
    # quotes back, and rounding 4.7 MB to "5 MB" is a 6 % error on it.
    if mb < 10:
        return f"{mb:.1f} MB"
    return f"{mb:.0f} MB"


def ratio(a: float, b: float) -> str:
    """How many times faster `a` is than `b`."""
    factor = b / a
    if factor >= 10:
        return f"{factor:.0f}x"
    return f"{factor:.1f}x"


def check_formatters() -> None:
    """The SVG embeds glyphs as paths, so a label cannot be read back out of it.

    That makes the label text the one part of the figure no reviewer can check by
    looking, so it is pinned here instead: if someone changes the formatting, this
    fails rather than the chart quietly saying "0.0 s" for a 46 ms measurement.
    """
    cases_time = [
        (0.007, "7.0 ms"),
        (0.046, "46 ms"),
        (0.209, "209 ms"),
        (0.252, "252 ms"),
        (1.869, "1.87 s"),
        (6.140, "6.14 s"),
        (18.380, "18.38 s"),
        (63.617, "1.1 min"),
        (176.3, "2.9 min"),
        (562.269, "9.4 min"),
    ]
    cases_mb = [
        (4.7, "4.7 MB"),
        (6.0, "6.0 MB"),
        (31.0, "31 MB"),
        (67.0, "67 MB"),
        (126.8, "127 MB"),
        (375.9, "376 MB"),
        (642.0, "642 MB"),
        (2295.0, "2.24 GB"),
    ]
    cases_ratio = [(1.869, 63.617, "34x"), (0.209, 562.269, "2690x"), (0.046, 18.380, "400x")]

    for value, expected in cases_time:
        actual = human_time(value)
        assert actual == expected, f"human_time({value}) = {actual!r}, expected {expected!r}"
    for value, expected in cases_mb:
        actual = human_mb(value)
        assert actual == expected, f"human_mb({value}) = {actual!r}, expected {expected!r}"
    for ours, theirs, expected in cases_ratio:
        actual = ratio(ours, theirs)
        assert actual == expected, f"ratio({ours}, {theirs}) = {actual!r}, expected {expected!r}"


# ---------------------------------------------------------------- charts


def chart_pipeline(name: str, theme: dict) -> pathlib.Path:
    """Time and memory against webpack-bundle-analyzer, one row per input.

    Two panels sharing one set of row labels. The earlier version printed the
    category label on both panels, put four value labels plus a ratio on every
    row, and let the log axis choose its own ticks; on screen that is seven
    pieces of text per row and it reads as clutter. Here: the row label appears
    once, the ticks are chosen, and the only words on a row are its two numbers.
    """
    fig, (ax_time, ax_mem) = plt.subplots(
        1,
        2,
        figsize=(9.4, 3.1),
        dpi=200,
        constrained_layout=True,
        sharey=True,
    )
    fig.patch.set_facecolor(theme["bg"])

    rows = list(STATS.items())
    # Short, and identical in both panels: the module count belongs in the
    # caption, not repeated on every row of every panel.
    short = {label: label.split(",")[0] for label in STATS}
    ys = list(range(len(rows)))[::-1]
    height = 0.3

    for y, (label, data) in zip(ys, rows):
        mem = STATS_MEM[label]
        ax_time.barh(y + height / 2, data["ref_s"], height=height, color=theme["ref"], zorder=3)
        ax_time.barh(y - height / 2, data["ob_s"], height=height, color=theme["accent"], zorder=3)
        ax_mem.barh(y + height / 2, mem["ref_mb"], height=height, color=theme["ref"], zorder=3)
        ax_mem.barh(y - height / 2, mem["ob_mb"], height=height, color=theme["accent"], zorder=3)

        # Value labels sit in a column past the longest bar rather than at the end
        # of each bar, so the two numbers on a row line up and the eye compares
        # them instead of hunting.
        ax_time.text(430, y + height / 2, human_time(data["ref_s"]), va="center",
                     ha="right", color=theme["ref_ink"], fontsize=9.5)
        ax_time.text(430, y - height / 2, human_time(data["ob_s"]), va="center",
                     ha="right", color=theme["accent"], fontsize=9.5, fontweight="bold")
        ax_mem.text(7200, y + height / 2, human_mb(mem["ref_mb"]), va="center",
                    ha="right", color=theme["ref_ink"], fontsize=9.5)
        ax_mem.text(7200, y - height / 2, human_mb(mem["ob_mb"]), va="center",
                    ha="right", color=theme["accent"], fontsize=9.5, fontweight="bold")

    for ax, xmax, ticks, formatter in (
        (ax_time, 400.0, [1, 10, 100], human_time),
        (ax_mem, 6000.0, [100, 1000, 4000], human_mb),
    ):
        style_axes(ax, theme)
        ax.set_xscale("log")
        ax.set_xlim(0.7, xmax)
        ax.set_xticks(ticks)
        ax.set_xticklabels([formatter(t) for t in ticks])
        ax.set_yticks(ys)
        ax.set_yticklabels([short[label] for label, _ in rows], fontsize=10, color=theme["ink"])
        ax.set_ylim(-0.62, len(rows) - 0.38)
        ax.tick_params(axis="x", labelsize=9)

    # Shared row labels, once.
    ax_time.tick_params(axis="y", labelleft=True)
    ax_mem.tick_params(axis="y", labelleft=False)

    ax_time.set_title("wall clock", loc="left", fontsize=10.5, color=theme["muted"], pad=14)
    ax_mem.set_title("peak memory", loc="left", fontsize=10.5, color=theme["muted"], pad=14)

    fig.suptitle(
        "One pass: parse stats, measure every asset, fuse the source maps, render the report",
        x=0.006,
        ha="left",
        fontsize=13,
        color=theme["ink"],
        fontweight="bold",
    )
    fig.text(
        0.006,
        -0.02,
        "synthetic fixtures, median of 3 runs on the reference machine  ·  "
        "grey is webpack-bundle-analyzer 4.10.2  ·  log axes",
        ha="left",
        fontsize=9,
        color=theme["muted"],
    )
    return save(fig, name, theme)


def chart_source_maps(name: str, theme: dict) -> pathlib.Path:
    """Attribution time and memory against source-map-explorer, by source count.

    Two log-log panels, no legend and no per-point numbers: each series is
    labelled once at its right-hand end, and only the last point - the one the
    claim is about - carries a value. A log-log chart with a legend, three labels
    per series and two callouts is how a finding gets buried.
    """
    fig, (ax_time, ax_mem) = plt.subplots(
        1, 2, figsize=(9.4, 3.5), dpi=200, constrained_layout=True
    )
    fig.patch.set_facecolor(theme["bg"])

    sources = [p[0] for p in MAP_POINTS]
    ob_s = [p[1] for p in MAP_POINTS]
    sme_s = [p[2] for p in MAP_POINTS]
    ob_mb = [p[3] for p in MAP_POINTS]
    sme_mb = [p[4] for p in MAP_POINTS]

    for ax, ours, theirs, formatter in (
        (ax_time, ob_s, sme_s, human_time),
        (ax_mem, ob_mb, sme_mb, human_mb),
    ):
        ax.plot(sources, theirs, color=theme["ref"], linewidth=2.2, marker="o",
                markersize=5, zorder=3)
        ax.plot(sources, ours, color=theme["accent"], linewidth=2.2, marker="o",
                markersize=5, zorder=4)
        style_axes(ax, theme, xgrid=False)
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.set_xticks(sources)
        ax.set_xticklabels(["12", "10k", "50k"])
        ax.set_xlim(7, 190_000)  # headroom on the right for the series labels
        ax.set_xlabel("sources in the map", fontsize=9, color=theme["muted"], labelpad=6)
        # `formatter` is bound as a default argument on purpose. A lambda written
        # inside the loop closes over the loop variable, so both panels ended up
        # formatted with the *last* one - which is how the time panel shipped
        # with "1000 MB" on its y-axis while plotting seconds.
        ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _, f=formatter: f(v)))
        ax.tick_params(axis="both", labelsize=9)
        ax.spines["bottom"].set_color(theme["grid"])

    # One label per series, at the end of its line, and only the final point is
    # annotated. Everything else is the reader's eye to follow.
    for ax, ours, theirs, formatter in (
        (ax_time, ob_s, sme_s, human_time),
        (ax_mem, ob_mb, sme_mb, human_mb),
    ):
        ax.annotate(
            "OmniBundle",
            xy=(sources[-1], ours[-1]),
            xytext=(9, 4),
            textcoords="offset points",
            color=theme["accent"],
            fontsize=10,
            fontweight="bold",
            ha="left",
            va="center",
        )
        ax.annotate(
            "source-map-explorer",
            xy=(sources[-1], theirs[-1]),
            xytext=(9, -4),
            textcoords="offset points",
            color=theme["ref_ink"],
            fontsize=10,
            ha="left",
            va="center",
        )
        # The number the claim rests on, on the last point of each series only.
        ax.annotate(
            formatter(ours[-1]),
            xy=(sources[-1], ours[-1]),
            xytext=(-6, -13),
            textcoords="offset points",
            ha="right",
            color=theme["accent"],
            fontsize=10,
            fontweight="bold",
        )
        ax.annotate(
            formatter(theirs[-1]),
            xy=(sources[-1], theirs[-1]),
            xytext=(-6, 10),
            textcoords="offset points",
            ha="right",
            color=theme["ref_ink"],
            fontsize=10,
        )

    ax_time.set_ylim(0.003, 3000)
    ax_mem.set_ylim(3, 1400)

    ax_time.set_title("attribution time", loc="left", fontsize=10.5, color=theme["muted"], pad=14)
    ax_mem.set_title("peak memory", loc="left", fontsize=10.5, color=theme["muted"], pad=14)

    # One callout, in genuinely empty space, saying the finding in words.
    ax_time.annotate(
        "5x the sources,\n30x the time",
        xy=(10_000, 18.38),
        xytext=(1_250, 240),
        color=theme["ref_ink"],
        fontsize=10,
        ha="center",
        arrowprops=dict(arrowstyle="-", color=theme["ref"], linewidth=1),
    )

    fig.suptitle(
        "Source map attribution, by the number of sources",
        x=0.006,
        ha="left",
        fontsize=13,
        color=theme["ink"],
        fontweight="bold",
    )
    fig.text(
        0.006,
        -0.02,
        "the 12-source point is the real preact build; 10k and 50k are synthetic  ·  "
        "median of 3 except the 9.4-minute reference run  ·  log-log axes",
        ha="left",
        fontsize=9,
        color=theme["muted"],
    )
    return save(fig, name, theme)


def chart_budget(name: str, theme: dict) -> pathlib.Path:
    """Peak memory against input size, with the published ceiling drawn in."""
    fig, ax = plt.subplots(figsize=(6.6, 3.3), dpi=200, constrained_layout=True)
    fig.patch.set_facecolor(theme["bg"])

    points = [
        # input MB, measured MB, ceiling MB, label
        (363.0, 126.8, 200.0, "363 MB"),
        (1049.0, 375.9, 400.0, "1 GB"),
        (1049.0, 137.0, 500.0, "1 GB + 36.5 MB map"),
    ]
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]

    ax.plot(xs, ys, color=theme["accent"], linewidth=2.2, marker="o", markersize=7, zorder=4)
    for x, y, ceiling, label in points:
        ax.annotate(
            f"{human_mb(y)}\n{label}",
            (x, y),
            textcoords="offset points",
            xytext=(0, -34),
            ha="center",
            color=theme["ink"],
            fontsize=9,
        )
        ax.plot([x, x], [y, ceiling], color=theme["grid"], linewidth=1.4, linestyle=(0, (3, 3)), zorder=2)
        ax.plot([x], [ceiling], marker="_", markersize=18, color=theme["ref_ink"], zorder=3)

    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xticks([363, 1049])
    ax.set_xticklabels(["363 MB", "1 GB"])
    ax.set_xlabel("input size", fontsize=9, color=theme["muted"])
    ax.yaxis.set_major_formatter(FuncFormatter(lambda v, _: human_mb(v)))
    ax.set_ylim(80, 700)
    style_axes(ax, theme, xgrid=False)

    ax.annotate(
        "the dash is the published ceiling\nfor that target, not the measurement",
        xy=(1049, 500),
        xytext=(430, 610),
        color=theme["muted"],
        fontsize=9,
        style="italic",
        arrowprops=dict(arrowstyle="-", color=theme["muted"], linewidth=0.9),
    )

    fig.suptitle(
        "Peak memory stays under the budget as the input grows",
        x=0.008,
        ha="left",
        fontsize=12.5,
        color=theme["ink"],
        fontweight="bold",
    )
    fig.text(
        0.008,
        -0.04,
        "median of 3, sampled every 25 ms · working set includes file-backed pages of the "
        "input, so the figure moves about 15% between runs",
        ha="left",
        fontsize=8.5,
        color=theme["muted"],
    )

    return save(fig, name, theme)


def theme_name(theme: dict) -> str:
    return "light" if theme["bg"] == THEMES["light"]["bg"] else "dark"


def assert_readable(svg: str, name: str, display_width_px: int = 880) -> None:
    """The smallest label must survive being shown at README width.

    A chart is authored at some physical size and then displayed at whatever
    width the page gives it, so a 9 pt label on an 11.6-inch figure arrives at
    about 9 px - technically present, practically squinting. This converts the
    font size through the figure's own width into the size a reader actually
    gets, which is the only number that says whether the chart is comfortable or
    merely correct.
    """
    width_match = re.search(r'<svg[^>]*\bwidth="([\d.]+)(pt|px)"', svg)
    if not width_match:
        raise SystemExit(f"{name}: cannot read the figure width from the SVG")
    width_value = float(width_match.group(1))

    sizes = [float(m) for m in re.findall(r"font-size:\s*([\d.]+)px", svg)]
    if not sizes:
        sizes = [float(m) for m in re.findall(r'font-size="([\d.]+)"', svg)]
    if not sizes:
        raise SystemExit(f"{name}: the layout probe contains no font sizes")

    # Font sizes are in points and the figure width is in points, so the 96/72
    # factor that turns each into CSS pixels cancels out: what the reader
    # actually gets is `font_pt x (displayed width / figure width)`. Getting
    # that wrong by 1.333 is exactly how a comfortable chart reads as cramped.
    smallest = min(sizes) * (display_width_px / width_value)
    if smallest < 9.0:
        raise SystemExit(
            f"{name}: the smallest label renders at {smallest:.1f} px at README width "
            f"({min(sizes):.1f} pt on a {width_value:.0f} pt wide figure) - under the 9 px floor.\n"
            "  Make the figure narrower or the type larger; do not shrink the font."
        )


def save(fig, name: str, theme: dict) -> tuple[pathlib.Path, pathlib.Path]:
    """Write the SVG (for GitHub) and a PNG, and refuse to write a bad one.

    Two defects are invisible to a script that only writes files, and both are
    what makes a chart look amateur:

    - **clipped ink.** matplotlib lays out inside the figure box, so anything
      that overflows simply disappears at the edge.
    - **colliding text.** Two labels on top of each other, or a label over a
      data point, reads as clutter even when every number is correct.

    The first is checked on the rendered PNG: the outermost pixel ring must be
    pure background. The second is checked on a *second* render with
    `svg.fonttype = 'none'`, which keeps text as real `<text>` elements with
    their coordinates - the shipped SVG uses glyph outlines so it renders
    identically on every machine, which also means its text cannot be read back
    to find a collision. Two renders of the same figure, same layout, one of them
    measurable.
    """
    svg_path = OUT / f"chart-{name}-{theme_name(theme)}.svg"
    png_path = OUT / f"chart-{name}-{theme_name(theme)}.png"

    # The measurable render first: it shares every layout decision with the
    # shipped one and only differs in how the glyphs are serialised.
    with matplotlib.rc_context({"svg.fonttype": "none"}):
        measure_path = OUT / ".layout-probe.svg"
        fig.savefig(measure_path, format="svg", facecolor=theme["bg"], bbox_inches="tight")
        probe = measure_path.read_text(encoding="utf-8")
        measure_path.unlink(missing_ok=True)

    collisions = find_text_collisions(probe)
    assert_readable(probe, f"{name}-{theme_name(theme)}")
    figure_stats.setdefault(name, {})[theme_name(theme)] = measure_probe(probe)

    fig.savefig(svg_path, format="svg", facecolor=theme["bg"], bbox_inches="tight")
    fig.savefig(png_path, format="png", facecolor=theme["bg"], bbox_inches="tight", dpi=200)
    plt.close(fig)

    svg_path.write_text(stable_svg(svg_path.read_text(encoding="utf-8")), encoding="utf-8")

    assert_not_clipped(png_path, theme)
    if collisions:
        listed = "\n".join(f"    {c}" for c in collisions[:8])
        raise SystemExit(
            f"{name}-{theme_name(theme)}: {len(collisions)} overlapping text label(s):\n"
            f"{listed}\n"
            "  Fix the layout, not the font size."
        )
    return svg_path, png_path


def stable_svg(text: str) -> str:
    """Make one SVG byte-identical across runs, so `git diff` after a
    regeneration means something.

    matplotlib makes two runs of the same figure differ in two ways, neither of
    which is a change to the chart:

    - `<dc:date>` inside the metadata records when it was drawn.
    - generated element ids are random (`pc626e149fb`, `m581a584697`), so every
      `clip-path="url(#...)"` and `<use xlink:href="#..."` differs even when the
      geometry is untouched.

    Without this, the CI gate that regenerates the charts and fails on a diff
    fails on every run, and a gate that always fails is a gate nobody reads.
    Ids are renumbered in order of first appearance, which is stable because the
    draw order is.
    """
    text = re.sub(r"<dc:date>.*?</dc:date>", "<dc:date>1970-01-01T00:00:00</dc:date>", text)
    ids: dict[str, str] = {}
    # matplotlib ids are a one or two letter prefix plus hex digits.
    generated = r"([a-z]{1,2}[0-9a-f]{10})"

    def rename_id(raw: str) -> str:
        return ids.setdefault(raw, f"ob{len(ids)}")

    text = re.sub(rf"#{generated}\b", lambda m: "#" + rename_id(m.group(1)), text)
    return re.sub(rf'\bid="{generated}"', lambda m: 'id="' + rename_id(m.group(1)) + '"', text)


def measure_probe(probe: str) -> dict:
    """Layout facts about a figure, recorded so they can be asserted later.

    These live in the sidecar rather than being re-derived from the shipped SVG
    because that file stores glyphs as paths and therefore contains no text at
    all: a checker pointed at it finds nothing, reports success, and proves
    nothing.
    """
    width = re.search(r'<svg[^>]*width="([\d.]+)(pt|px)"', probe)
    if not width:
        raise SystemExit("the layout probe has no width; the figure was not written")
    width_pt = float(width.group(1))
    sizes = [float(m.group(1)) for m in re.finditer(r"font-size:\s*([\d.]+)px", probe)]
    labels = [m.group(1).strip() for m in re.finditer(r">([^<>]{2,})</text>", probe)]
    if not sizes or not labels:
        raise SystemExit(
            f"the layout probe found {len(sizes)} font sizes and {len(labels)} labels; "
            "svg.fonttype is not 'none', so nothing can be measured"
        )
    return {
        "figure_width_pt": round(width_pt, 1),
        "labels": len(labels),
        "font_pt_min": round(min(sizes), 2),
        "font_pt_max": round(max(sizes), 2),
        # Points against points: the pt-to-px conversion cancels.
        "smallest_px_at_880px_wide": round(min(sizes) * (880 / width_pt), 2),
    }


def find_text_collisions(svg: str) -> list[str]:
    """Text labels whose estimated bounding boxes overlap.

    Width is estimated from the font size and the character count rather than
    measured, because a glyph-accurate measurement is a lot of machinery for a
    check whose job is to catch the obvious case. Over-estimating is the right
    direction: it makes the check stricter than a reader's eye, not laxer.
    """
    pattern = re.compile(
        r'<text[^>]*\bx="([\d.-]+)"[^>]*\by="([\d.-]+)"[^>]*font-size:?\s*([\d.]+)'
        r'[^>]*>([^<]*)</text>',
        re.IGNORECASE,
    )
    # A second pattern for the attribute order matplotlib sometimes emits.
    pattern_size_first = re.compile(
        r'<text[^>]*font-size:?\s*([\d.]+)[^>]*\bx="([\d.-]+)"[^>]*\by="([\d.-]+)"[^>]*>([^<]*)</text>',
        re.IGNORECASE,
    )

    boxes = []
    for match in pattern.finditer(svg):
        x, y, size, text = float(match[1]), float(match[2]), float(match[3]), match[4]
        boxes.append((x, y, size, text))
    for match in pattern_size_first.finditer(svg):
        size, x, y, text = float(match[1]), float(match[2]), float(match[3]), match[4]
        boxes.append((x, y, size, text))

    # 0.62 em is a generous average advance width for a UI sans face; the
    # leading is not in the box because vertical collisions between text on
    # different rows are expected.
    def box(item):
        x, y, size, text = item
        width = len(text) * size * 0.62
        return (x, y - size * 0.8, x + width, y + size * 0.25)

    collisions = []
    for i, a in enumerate(boxes):
        for b in boxes[i + 1 :]:
            ax0, ay0, ax1, ay1 = box(a)
            bx0, by0, bx1, by1 = box(b)
            overlap_x = min(ax1, bx1) - max(ax0, bx0)
            overlap_y = min(ay1, by1) - max(ay0, by0)
            if overlap_x > 1.0 and overlap_y > 1.0:
                collisions.append(f"{a[3]!r} overlaps {b[3]!r}")
    return collisions


def assert_not_clipped(png_path: pathlib.Path, theme: dict) -> None:
    """Fail if ink reaches the edge of the rendered image."""
    import numpy as np
    from matplotlib import image as mpimg

    data = mpimg.imread(png_path)
    if data.ndim == 2:
        data = np.stack([data] * 3, axis=-1)
    rgb = data[..., :3]

    background = rgb[0, 0]
    ring = np.concatenate([rgb[0, :, :], rgb[-1, :, :], rgb[:, 0, :], rgb[:, -1, :]])
    distance = np.abs(ring - background).max(axis=-1)
    # Anti-aliasing means the background itself varies by a hair between pixels.
    clipped = int((distance > 0.02).sum())
    if clipped > 0:
        raise SystemExit(
            f"{png_path.name}: {clipped} pixels of ink in the outermost ring - "
            "something is clipped. Fix the layout rather than the crop."
        )


CHARTS = {
    "pipeline": chart_pipeline,
    "source-maps": chart_source_maps,
    "memory-budget": chart_budget,
}


def main() -> int:
    # First: the label formatters. The SVG stores glyphs as paths, so a label
    # cannot be read back out of the figure and nothing else would notice a
    # formatting change that turned 46 ms into "0.0 s".
    check_formatters()

    OUT.mkdir(parents=True, exist_ok=True)
    written = []
    for name, fn in CHARTS.items():
        for key, theme in THEMES.items():
            svg_path, png_path = fn(name, theme)
            written.append(svg_path)
            print(
                f"wrote {svg_path.relative_to(REPO)} "
                f"({svg_path.stat().st_size // 1024} KB) + png"
            )

    sidecar = {
        "generated_by": "bench/harness/make-charts.py",
        "note": "Every number plotted, with where it came from. The README cites this "
        "file so a reader can check the figure against the records instead of "
        "trusting the picture.",
        "provenance": PROVENANCE,
        "figures": figure_stats,
        "stats_pipeline": {
            label: {**data, **STATS_MEM[label]} for label, data in STATS.items()
        },
        "source_maps": [
            {
                "sources": s,
                "omnibundle_seconds": ob,
                "source_map_explorer_seconds": sme,
                "omnibundle_mb": obmb,
                "source_map_explorer_mb": smemb,
            }
            for s, ob, sme, obmb, smemb in MAP_POINTS
        ],
    }
    sidecar_path = OUT / "charts-data.json"
    sidecar_path.write_text(json.dumps(sidecar, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {sidecar_path.relative_to(REPO)}")
    print("ok   no clipped pixels in any chart")
    return 0


if __name__ == "__main__":
    sys.exit(main())
