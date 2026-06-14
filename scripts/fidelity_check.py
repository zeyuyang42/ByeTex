#!/usr/bin/env python3
"""Fidelity regression gate (Layer 1 — deterministic, no agent).

Compares a fresh `scripts/visual_test.py` index.json against a committed
baseline (scripts/fidelity_baseline.json) and fails (exit 1) iff render
fidelity REGRESSES:

  * the corpus-wide `fidelity_score` drops by more than --score-tol, or
  * a paper's `word_recall` drops by more than --recall-tol, or
  * a paper that was `structure_ok` in the baseline no longer is.

Improvements are reported so the baseline can be promoted. Papers absent from
the current run are skipped, so gitignored corpus payloads (a paper missing
locally / in CI) never cause a false regression.

This mirrors the compile acceptance gate (scripts/acceptance_check.py):
compile-rate is the GATE, render fidelity is the DRIVER — this script makes a
fidelity regression visible without coupling it to the compile gate. See
docs/scorecard.md.

Usage:
  # gate a fresh run against the baseline
  python3 scripts/fidelity_check.py --current tests/visual/index.json \
      --baseline scripts/fidelity_baseline.json
  # (re)generate a trimmed, path-free baseline from a fresh index.json
  python3 scripts/fidelity_check.py --current tests/visual/index.json \
      --emit-baseline scripts/fidelity_baseline.json
"""
import argparse
import json
import sys

# Fields kept in the committed baseline: enough to gate + promote, and NO
# machine-specific absolute paths (the index.json `composite` field is dropped).
BASELINE_FIELDS = (
    "status",
    "structure_ok",
    "word_recall",
    "heading_recall",
    "page_ratio",
    "mean_ssim",
)


def load(path):
    with open(path) as fh:
        return json.load(fh)


def trim_for_baseline(index):
    """A committed-baseline view of an index.json: the gate metrics only, no
    absolute paths or timestamps."""
    papers = {
        pid: {k: p.get(k) for k in BASELINE_FIELDS}
        for pid, p in index.get("papers", {}).items()
    }
    return {
        "_comment": (
            "Fidelity baseline for scripts/fidelity_check.py. Per-paper render "
            "metrics + corpus fidelity_score from scripts/visual_test.py. "
            "Regenerate with `./scripts/fidelity_gate.sh --update-baseline` "
            "(or fidelity_check.py --emit-baseline)."
        ),
        "fidelity_score": index.get("fidelity_score"),
        "papers": papers,
    }


def evaluate(current, baseline, score_tol, recall_tol):
    """Return (regressions, improvements) as lists of human-readable strings."""
    regressions, improvements = [], []

    cur_score = current.get("fidelity_score")
    base_score = baseline.get("fidelity_score")
    if base_score is not None and cur_score is not None:
        if cur_score < base_score - score_tol:
            regressions.append(
                f"corpus fidelity_score {cur_score:.3f} < baseline "
                f"{base_score:.3f} (-{base_score - cur_score:.3f}, tol {score_tol})"
            )
        elif cur_score > base_score + score_tol:
            improvements.append(
                f"corpus fidelity_score {cur_score:.3f} > baseline {base_score:.3f}"
            )

    cur_papers = current.get("papers", {})
    for pid, base in baseline.get("papers", {}).items():
        cur = cur_papers.get(pid)
        if cur is None:
            continue  # absent this run; skip (gitignored corpus payload)
        if base.get("structure_ok") and not cur.get("structure_ok"):
            regressions.append(f"{pid}: structure_ok true→false")
        bw, cw = base.get("word_recall"), cur.get("word_recall")
        if bw is not None and cw is not None:
            if cw < bw - recall_tol:
                regressions.append(
                    f"{pid}: word_recall {cw:.3f} < baseline {bw:.3f} "
                    f"(-{bw - cw:.3f}, tol {recall_tol})"
                )
            elif cw > bw + recall_tol:
                improvements.append(f"{pid}: word_recall {cw:.3f} > baseline {bw:.3f}")
    return regressions, improvements


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument("--current", required=True, help="fresh index.json from visual_test.py")
    ap.add_argument("--baseline", help="committed fidelity baseline (gate mode)")
    ap.add_argument(
        "--emit-baseline",
        metavar="PATH",
        help="instead of gating, write a trimmed baseline from --current to PATH",
    )
    ap.add_argument("--score-tol", type=float, default=0.02)
    ap.add_argument("--recall-tol", type=float, default=0.05)
    args = ap.parse_args(argv)

    current = load(args.current)

    if args.emit_baseline:
        with open(args.emit_baseline, "w") as fh:
            json.dump(trim_for_baseline(current), fh, indent=2, sort_keys=True)
            fh.write("\n")
        n = len(current.get("papers", {}))
        print(f"fidelity: wrote baseline ({n} papers) → {args.emit_baseline}")
        return 0

    if not args.baseline:
        ap.error("either --baseline (gate) or --emit-baseline is required")

    baseline = load(args.baseline)
    regressions, improvements = evaluate(current, baseline, args.score_tol, args.recall_tol)

    cs, bs = current.get("fidelity_score"), baseline.get("fidelity_score")
    print(f"fidelity: score {cs} (baseline {bs})")
    if improvements:
        print("  IMPROVED (consider `--update-baseline`):")
        for i in improvements:
            print(f"    + {i}")
    if regressions:
        print("  REGRESSION:", file=sys.stderr)
        for r in regressions:
            print(f"    - {r}", file=sys.stderr)
        print("FAIL: render fidelity regression.", file=sys.stderr)
        return 1
    print("OK: no fidelity regression.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
