#!/usr/bin/env python3
"""Acceptance gate comparator (Phase-3 oracle).

Reads `corpus_sweep.sh --with-oracle` verdict lines from stdin and compares them
against a committed baseline (scripts/acceptance_baseline.json). Enforces the
scorecard's decision rule — compile-rate is the GATE:

  * REGRESSION (exit 1): a `known_pass` paper now emits Typst that fails to
    compile (BYETEX_FAIL). This is the only hard-fail condition.
  * FIXED (reported): a `known_fail` paper now PASSES — promote it in the
    baseline.
  * NEW FAIL (reported): a paper not in the baseline now BYETEX_FAILs — likely a
    freshly-harvested paper; triage and add to known_fail (or fix it).

Only papers actually present in the sweep are checked, so gitignored corpus
payloads (a paper absent locally / in CI) never cause a false regression.

Usage:
  ./scripts/corpus_sweep.sh --with-oracle | \
      python3 scripts/acceptance_check.py --baseline scripts/acceptance_baseline.json
"""
import argparse
import json
import re
import sys

VERDICT_RE = re.compile(r"^(PASS|BYETEX_FAIL|INPUT_BROKEN|UNATTRIBUTED|SKIP)\s+(\S+)")


def parse_verdicts(lines):
    """Return {verdict: set(paper_id)} from sweep output lines."""
    out = {}
    for line in lines:
        m = VERDICT_RE.match(line.strip())
        if not m:
            continue
        verdict, paper = m.group(1), m.group(2).rstrip(":")
        out.setdefault(verdict, set()).add(paper)
    return out


def evaluate(verdicts, baseline):
    """Compute (regressions, fixed, new_fail) sets from verdicts + baseline."""
    known_pass = set(baseline.get("known_pass", []))
    known_fail = set(baseline.get("known_fail", []))
    passed = verdicts.get("PASS", set())
    byetex_fail = verdicts.get("BYETEX_FAIL", set())

    regressions = sorted(known_pass & byetex_fail)
    fixed = sorted(known_fail & passed)
    new_fail = sorted(byetex_fail - known_pass - known_fail)
    return regressions, fixed, new_fail


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", required=True)
    args = ap.parse_args(argv)

    with open(args.baseline) as fh:
        baseline = json.load(fh)
    verdicts = parse_verdicts(sys.stdin)
    regressions, fixed, new_fail = evaluate(verdicts, baseline)

    n_pass = len(verdicts.get("PASS", set()))
    n_fail = len(verdicts.get("BYETEX_FAIL", set()))
    print(f"acceptance: PASS={n_pass} BYETEX_FAIL={n_fail} "
          f"(baseline known_pass={len(baseline.get('known_pass', []))})")

    if fixed:
        print(f"  FIXED (promote to known_pass): {', '.join(fixed)}")
    if new_fail:
        print(f"  NEW BYETEX_FAIL (triage / add to known_fail): {', '.join(new_fail)}")

    if regressions:
        print(f"  REGRESSION — known-passing papers now fail to compile: "
              f"{', '.join(regressions)}", file=sys.stderr)
        print("FAIL: compile regression in known_pass set.", file=sys.stderr)
        return 1
    print("OK: no compile regression in known_pass set.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
