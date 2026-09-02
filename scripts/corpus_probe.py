#!/usr/bin/env python3
"""Convert every corpus paper and report where a pattern appears in the output.

The sweep every converter tick needs, with the three mistakes already made:

  * ENTRY FILE comes from the harness (`tests/visual/<id>/summary.json`
    `toplevel_tex`), not from guessing. `find ... | head -1` returns an unsorted
    path — it once converted a paper's `preamble.tex` and produced a 184-byte
    output that read as a lost feature; and 5 corpus papers contain more than one
    file with `\\documentclass`, so even a SORTED pick can convert a supplement.
  * EXIT STATUS is checked per paper and failures are printed. A sweep that
    discards stderr turns a panic into a clean-looking zero: a converter crashing
    on every paper reported as "the feature fired on nothing".
  * COUNTS are per paper AND total, so "51 papers" cannot be mistaken for a delta
    when most of those matches predate the change.

Usage:
    scripts/corpus_probe.py 'header:'                  # which papers emit it
    scripts/corpus_probe.py --count '\\[#strong\\['      # totals, for a before/after
    scripts/corpus_probe.py --json 'byetex-float'
    BYETEX_BIN=/path/to/byetex scripts/corpus_probe.py ...

To measure a DELTA, run it once per revision with each binary built from that
revision — comparing against a stale binary is its own recurring bug.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VISUAL = REPO / "tests" / "visual"
CORPUS = REPO / "corpus"


def resolve_bin() -> Path:
    env = os.environ.get("BYETEX_BIN")
    cand = Path(env) if env else REPO / "target" / "release" / "byetex"
    if not cand.exists():
        sys.exit(f"byetex not found at {cand}; set BYETEX_BIN or `cargo build --release`")
    return cand


def entries() -> list[tuple[str, Path]]:
    """(paper_id, toplevel .tex) for every paper the fidelity harness knows."""
    out = []
    for summary in sorted(VISUAL.glob("*/summary.json")):
        pid = summary.parent.name
        try:
            top = json.loads(summary.read_text()).get("toplevel_tex")
        except (OSError, ValueError):
            continue
        if not top:
            continue
        matches = sorted((CORPUS / pid / "source").rglob(top))
        if matches:
            out.append((pid, matches[0]))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("pattern", help="regex to look for in each emitted main.typ")
    ap.add_argument("--count", action="store_true", help="report occurrence counts, not just papers")
    ap.add_argument("--json", action="store_true", help="emit JSON")
    args = ap.parse_args()

    byetex = resolve_bin()
    rx = re.compile(args.pattern)
    papers = entries()
    if not papers:
        # An empty run reads exactly like a clean one; say which it was.
        sys.exit("no corpus papers resolved — is tests/visual/ populated?")

    hits: dict[str, int] = {}
    failures: list[str] = []
    tmp = Path(tempfile.mkdtemp(prefix="byetex-probe-"))
    try:
        for pid, src in papers:
            out_dir = tmp / pid
            proc = subprocess.run(
                [str(byetex), "convert", "--project", str(src),
                 "--project-out", str(out_dir), "--force", "--no-brief"],
                capture_output=True, text=True,
            )
            typ = out_dir / "main.typ"
            if proc.returncode != 0 or not typ.exists():
                # Never silent: a crash here is the finding, not an absence.
                first = (proc.stderr.strip().splitlines() or ["(no stderr)"])[0]
                failures.append(f"{pid}: {first[:120]}")
                continue
            n = len(rx.findall(typ.read_text(errors="replace")))
            if n:
                hits[pid] = n
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    if args.json:
        print(json.dumps({"pattern": args.pattern, "papers_converted": len(papers),
                          "hits": hits, "total": sum(hits.values()),
                          "failures": failures}, indent=2))
    else:
        for pid, n in sorted(hits.items()):
            print(f"  {pid:26s} {n}" if args.count else f"  {pid}")
        print(f"\npapers converted: {len(papers)}   with a match: {len(hits)}"
              + (f"   total occurrences: {sum(hits.values())}" if args.count else ""))
        for f in failures:
            print(f"  CONVERT FAILED  {f}", file=sys.stderr)
        if failures:
            print(f"\n{len(failures)} paper(s) failed to convert — see above",
                  file=sys.stderr)

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
