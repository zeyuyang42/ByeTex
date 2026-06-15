#!/usr/bin/env python3
"""Findings regression diff (Layer 2 — agent-graded fidelity).

After the `byetex-visual-grading` skill regrades a paper into a findings.json,
diff it against a committed baseline findings.json (or a directory of them) and
flag any dimension that WORSENED — a verdict moving match/minor → major, or a
same-verdict severity jump of at least --sev-bump. New worse findings are the
regression signal; resolved ones are reported as improvements.

Unlike Layer 1 (deterministic structural metrics in fidelity_check.py), this
catches typography / layout regressions that only a vision grader can see. It
is intended to be run on-demand around a fidelity-affecting change, not on
every push. See skills/byetex-visual-grading.md for the findings.json schema.

Usage:
  # single paper
  python3 scripts/findings_diff.py --baseline base.json --current new.json
  # a directory of per-paper findings (matched by filename)
  python3 scripts/findings_diff.py --baseline-dir docs/fidelity-baseline \
      --current-dir tests/visual/findings
"""
import argparse
import json
import os
import sys

# Verdict severity ranking — `na` is "not applicable", same weight as `match`.
RANK = {"match": 0, "na": 0, "minor": 1, "major": 2}


def load(path):
    with open(path) as fh:
        return json.load(fh)


def index_findings(doc):
    """Map dimension -> (verdict, severity) for one findings.json."""
    out = {}
    for f in doc.get("findings", []):
        out[f.get("dimension")] = (f.get("verdict", "na"), f.get("severity", 0) or 0)
    return out


def diff_one(paper, base, cur, sev_bump):
    regressions, improvements = [], []
    b, c = index_findings(base), index_findings(cur)
    for dim, (cv, cs) in c.items():
        bv, bs = b.get(dim, ("match", 0))
        worse_verdict = RANK.get(cv, 0) > RANK.get(bv, 0)
        worse_severity = cv == bv and (cs - bs) >= sev_bump
        if worse_verdict or worse_severity:
            regressions.append(f"{paper}/{dim}: {bv}({bs}) → {cv}({cs})")
    for dim, (bv, bs) in b.items():
        cv, cs = c.get(dim, ("match", 0))
        if RANK.get(cv, 0) < RANK.get(bv, 0):
            improvements.append(f"{paper}/{dim}: {bv}({bs}) → {cv}({cs})")
    return regressions, improvements


def collect_pairs(args):
    if args.baseline and args.current:
        return [("paper", load(args.baseline), load(args.current))]
    pairs = []
    for name in sorted(os.listdir(args.current_dir)):
        if not name.endswith(".json"):
            continue
        base = os.path.join(args.baseline_dir, name)
        if os.path.isfile(base):
            pairs.append((name[:-5], load(base), load(os.path.join(args.current_dir, name))))
    return pairs


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline")
    ap.add_argument("--current")
    ap.add_argument("--baseline-dir")
    ap.add_argument("--current-dir")
    ap.add_argument(
        "--sev-bump",
        type=int,
        default=2,
        help="same-verdict severity increase counted as a regression",
    )
    args = ap.parse_args(argv)
    if not ((args.baseline and args.current) or (args.baseline_dir and args.current_dir)):
        ap.error("give --baseline/--current or --baseline-dir/--current-dir")

    all_reg, all_imp = [], []
    for paper, base, cur in collect_pairs(args):
        reg, imp = diff_one(paper, base, cur, args.sev_bump)
        all_reg += reg
        all_imp += imp

    if all_imp:
        print("IMPROVED:")
        for i in all_imp:
            print(f"  + {i}")
    if all_reg:
        print("REGRESSION (visual fidelity worsened):", file=sys.stderr)
        for r in all_reg:
            print(f"  - {r}", file=sys.stderr)
        print("FAIL: visual fidelity regression.", file=sys.stderr)
        return 1
    print("OK: no visual fidelity regression.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
