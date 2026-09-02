#!/usr/bin/env python3
"""Detect Typst overflow PILE-UPS in rendered PDFs.

Typst does not break an oversized unbreakable block (a `#figure`, a `scale()`d
table) across pages -- it CLAMPS the overflow, painting every excess line at the
same y. The result is an illegible stack of overprinted words at the page bottom.

This is invisible to every other gate we have:

  * it compiles, rc=0, with no warning
  * the PDF TEXT LAYER stays complete, so `word_recall` scores it 100%
  * pymupdf and `pdftotext -bbox` both clip reported coordinates to the page
    box, so "content past the page edge" detectors report a clean corpus

The signal that does work is words sharing an identical (x, y): normal text
never overprints, so a position carrying several distinct words is a pile-up.

Usage:
    uv run --with pymupdf python3 scripts/pileup_check.py 'tests/visual/*/typst.pdf'
    uv run --with pymupdf python3 scripts/pileup_check.py 'out/*.pdf' --json

Exit status is 1 when any pile-up is found, so it can gate a build.

Compare against the LaTeX truth before filing a bug: a paper whose truth also
piles up is an input problem, not a converter one. When this was first run, all
5 flagged corpus papers had clean truths (see PR #515).
"""

import argparse
import collections
import glob
import json
import os
import sys

try:
    import pymupdf
except ImportError:  # pragma: no cover - dev-only dependency
    sys.exit("pymupdf is required: uv run --with pymupdf python3 scripts/pileup_check.py ...")

# Words at one spot before it counts. Two can legitimately coincide (a rule
# glyph over a letter, a combining accent); four never do.
DEFAULT_THRESHOLD = 4


def pileups(pdf, threshold=DEFAULT_THRESHOLD):
    """Return [(count, page, y, x, sample_words)], worst first."""
    hits = []
    with pymupdf.open(pdf) as doc:
        for pno, page in enumerate(doc, start=1):
            at = collections.defaultdict(set)
            for w in page.get_text("words"):
                x0, y0, text = w[0], w[1], w[4]
                if text.strip():
                    at[(round(y0), round(x0))].add(text)
            for (y, x), words in at.items():
                if len(words) >= threshold:
                    hits.append((len(words), pno, y, x, sorted(words)[:4]))
    hits.sort(reverse=True)
    return hits


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pattern", help="glob of PDFs to check (quote it)")
    ap.add_argument("--threshold", type=int, default=DEFAULT_THRESHOLD,
                    help=f"distinct words at one (x, y) to flag (default {DEFAULT_THRESHOLD})")
    ap.add_argument("--json", action="store_true", help="emit JSON instead of a table")
    args = ap.parse_args()

    paths = sorted(glob.glob(args.pattern))
    if not paths:
        # An empty run reads exactly like a clean one; say which it was.
        sys.exit(f"no PDFs matched {args.pattern!r} -- nothing was checked")

    report = {}
    for pdf in paths:
        hits = pileups(pdf, args.threshold)
        if hits:
            name = os.path.basename(os.path.dirname(pdf)) or os.path.basename(pdf)
            report[name] = [
                {"words": n, "page": p, "y": y, "x": x, "sample": s} for n, p, y, x, s in hits
            ]

    if args.json:
        print(json.dumps({"checked": len(paths), "pileups": report}, indent=2))
    else:
        for name, hits in report.items():
            w = hits[0]
            print(f"{name:24s} worst={w['words']:3d} words overprinted "
                  f"at p{w['page']} y={w['y']} :: {w['sample']}")
        print(f"\nchecked {len(paths)} PDF(s); {len(report)} with pile-ups")

    return 1 if report else 0


if __name__ == "__main__":
    sys.exit(main())
