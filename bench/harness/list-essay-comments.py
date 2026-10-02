"""Report every comment block that reads like an essay, in every file.

The earlier passes were keyword-driven, which catches "nobody" and "honest" and
nothing else. What is left is structural: a block of six or more lines where the
first sentence states a fact and the rest defends it. Those need reading, not a
regex, so this only lists them - ranked worst first, so the next passes are driven
by the list rather than by a pattern.

    python bench/harness/list-essay-comments.py
"""

import pathlib
import re
import subprocess

# Long blocks, and blocks that open with a justification rather than a fact.
JUSTIFY = re.compile(
    r"^\s*(//|#|/\*|\*|<!--)?\s*"
    r"(why|because|so that|so a|the reason|purpose:|what this catches|this exists|"
    r"without this|without that|otherwise|which is why|the point of)\b",
    re.IGNORECASE,
)

SKIP_SUFFIX = {".png", ".svg", ".ico", ".woff2", ".lock", ".proptest-regressions"}
SKIP_NAME = re.compile(r"LICENSE|Cargo\.lock|package-lock")

pattern_for = {
    ".rs": re.compile(r"^\s*(///|//!|//[^\[!])"),
    ".py": re.compile(r"^\s*#(?![!\[])"),
    ".yml": re.compile(r"^\s*#(?![!\[])"),
    ".yaml": re.compile(r"^\s*#(?![!\[])"),
    ".js": re.compile(r"^\s*(//|/\*|\*)"),
    ".mjs": re.compile(r"^\s*(//|/\*|\*)"),
    ".sh": re.compile(r"^\s*#"),
    ".ps1": re.compile(r"^\s*#"),
    ".css": re.compile(r"^\s*/\*"),
    ".html": re.compile(r"^\s*<!--"),
    ".toml": re.compile(r"^\s*#"),
    ".txt": re.compile(r"^\s*#"),
}


def blocks(path: pathlib.Path):
    pattern = pattern_for.get(path.suffix)
    if pattern is None:
        return
    lines = path.read_text(encoding="utf-8", errors="replace").split("\n")
    i = 0
    while i < len(lines):
        if pattern.match(lines[i]):
            j = i
            while j < len(lines) and pattern.match(lines[j]):
                j += 1
            if j - i >= 6:
                yield i + 1, j - i, lines[i].strip()
            i = j
            continue
        i += 1


files = [
    f
    for f in subprocess.run(["git", "ls-files"], capture_output=True, text=True, check=True).stdout.split()
    if pathlib.Path(f).suffix not in SKIP_SUFFIX and not SKIP_NAME.search(f)
]

rows = []
for name in files:
    for line_no, length, first in blocks(pathlib.Path(name)):
        rows.append((length, name, line_no, first, bool(JUSTIFY.match(first))))

rows.sort(key=lambda r: (-r[0], r[1]))
print(f"{len(rows)} comment block(s) of 6+ lines, longest first\n")
for length, name, line_no, first, justifies in rows:
    tag = " <- opens by justifying" if justifies else ""
    print(f"  {length:3}  {name}:{line_no}{tag}")
    print(f"       {first[:88]}")
