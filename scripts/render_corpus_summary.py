#!/usr/bin/env python3
"""Render `tests/corpus/report.json` as a markdown block and optionally
update the README in-place between HTML-comment sentinels.

Usage:
    python3 scripts/render_corpus_summary.py \\
        --report tests/corpus/report.json \\
        --update README.md

    # Dry-run: print the block without touching any file.
    python3 scripts/render_corpus_summary.py \\
        --report tests/corpus/report.json \\
        --print
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path

START_MARKER = "<!-- corpus-summary:start -->"
END_MARKER = "<!-- corpus-summary:end -->"


def _short_sha() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except Exception:
        return "unknown"


def render_block(report: dict) -> str:
    total = report["total"]
    clean = report["clean"]
    warnings = report["warnings"]
    parse_err = report["parse_error"]
    by_cat: dict[str, int] = report.get("by_category", {})

    passable = clean + warnings
    pct = round(100 * passable / total) if total else 0
    today = date.today().isoformat()
    sha = _short_sha()

    lines: list[str] = []
    lines.append(f"_Last updated: {today} (commit {sha})_")
    lines.append("")
    lines.append(
        f"Corpus pass-rate (clean + warnings): **{pct}%** — {passable}/{total} files."
    )
    lines.append("")

    # Bucket table
    lines.append("| Bucket | Count |")
    lines.append("|---|---:|")
    lines.append(f"| Total | {total} |")
    lines.append(f"| Clean | {clean} |")
    lines.append(f"| Warnings (≥1, no parse error) | {warnings} |")
    lines.append(f"| Parse errors | {parse_err} |")
    lines.append("")

    # Per-category table (exclude parse_error — already in bucket table above)
    cats = sorted(
        ((k, v) for k, v in by_cat.items() if k != "parse_error"),
        key=lambda kv: (-kv[1], kv[0]),
    )
    if cats:
        lines.append("| Warning category | Count |")
        lines.append("|---|---:|")
        for kind, count in cats:
            lines.append(f"| `{kind}` | {count} |")

    return "\n".join(lines)


def update_readme(readme_path: Path, block: str) -> bool:
    """Replace content between the sentinel markers. Returns True if changed."""
    text = readme_path.read_text(encoding="utf-8")
    if START_MARKER not in text or END_MARKER not in text:
        print(
            f"error: markers not found in {readme_path}\n"
            f"  Expected: {START_MARKER!r} and {END_MARKER!r}",
            file=sys.stderr,
        )
        sys.exit(1)

    pattern = re.compile(
        re.escape(START_MARKER) + r".*?" + re.escape(END_MARKER),
        re.DOTALL,
    )
    replacement = f"{START_MARKER}\n{block}\n{END_MARKER}"
    new_text = pattern.sub(replacement, text)
    if new_text == text:
        return False
    readme_path.write_text(new_text, encoding="utf-8")
    return True


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--report", required=True, type=Path, help="Path to report.json")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--update", metavar="README", type=Path, help="Update file in-place")
    mode.add_argument("--print", dest="print_only", action="store_true", help="Print to stdout")
    args = ap.parse_args()

    report = json.loads(args.report.read_text(encoding="utf-8"))
    block = render_block(report)

    if args.print_only:
        print(f"{START_MARKER}\n{block}\n{END_MARKER}")
    else:
        changed = update_readme(args.update, block)
        if changed:
            print(f"updated {args.update}")
        else:
            print(f"no change in {args.update}")


if __name__ == "__main__":
    main()
