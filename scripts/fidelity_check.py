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
# ── the gated set: what a regression here FAILS the build ───────────────────
GATED_FIELDS = (
    "status",
    "structure_ok",
    "word_recall",
    "heading_recall",
    "page_ratio",
    "mean_ssim",
)

# ── the layout tier: measured, baselined, REPORTED — not gated ──────────────
#
# "Driver first, gate later." A tier that can fail a build on day one gets
# `|| true`-d within a week — Chromium demoted its layout-tree dumps for exactly
# that reason. So `--gate-layout` is EMPTY by default and properties are promoted
# one at a time, each with measured floors behind it.
#
# direction:
#   "down"       a drop is drift            (recalls)
#   "up"         any rise is drift          (relational/categorical counts)
#   "toward_one" |x - 1| growing is drift   (ratios, two-sided)
#
# `toward_one` is the Gecko rule and it matters: a one-sided threshold lets a fix
# overshoot past the target into the opposite error and never be noticed, and it
# hides the improvement that should force a re-baseline.
LAYOUT_TOLERANCES = {
    "anchor_recall":                 ("down",       0.05),
    "ordered_recall":                ("down",       0.05),
    "ordered_precision":             ("down",       0.05),
    "layout_column_mismatch_frac":   ("up",         0.00),   # relational — no epsilon
    "layout_text_width_ratio":       ("toward_one", 0.05),
    "layout_leading_ratio":          ("toward_one", 0.10),
    "layout_left_margin_ratio":      ("toward_one", 0.08),
    "layout_top_margin_ratio":       ("toward_one", 0.08),
    "layout_body_font_ratio":        ("toward_one", 0.06),
    "layout_page_trim_ratio":        ("toward_one", 0.02),
    "layout_small_tier_share_delta": ("up",         0.03),
    "layout_ink_ratio":              ("toward_one", 0.15),
}
REPORTED_FIELDS = tuple(LAYOUT_TOLERANCES)

BASELINE_FIELDS_LEGACY = (
    "status",
    "structure_ok",
    "word_recall",
    "heading_recall",
    "page_ratio",
    "mean_ssim",
    # Vintage, not a gated metric. index.json ACCUMULATES across runs and the
    # gate's default measures 5 papers, so a committed baseline is a PATCHWORK
    # of measurements taken at different code versions rather than a snapshot of
    # one. Carrying the version makes that visible; `null` means the entry
    # predates this field and its true vintage is unknown.
    "byetex_version",
)

# Persistence set: the gated metrics, the reported tier, and vintage.
BASELINE_FIELDS = tuple(dict.fromkeys(BASELINE_FIELDS_LEGACY + REPORTED_FIELDS))


def _deviation(field, value):
    """Distance from "no drift", per the field's declared direction."""
    direction, _ = LAYOUT_TOLERANCES[field]
    if direction == "toward_one":
        return abs(value - 1.0)
    if direction == "down":
        return max(0.0, 1.0 - value)
    return abs(value)  # "up"


def evaluate_layout(current, baseline, gated, tols):
    """-> (regressions, notices, improvements).

    Only fields named in `gated` can produce a REGRESSION; everything else is a
    NOTICE. That is the whole design: promotion is per-property and one PR at a
    time, not a global switch, so a gate is green on the day it is turned on and
    fires only on NEW breakage.

    A field that is None on either side is SKIPPED — never counted as passing and
    never as failing. Until a baseline is re-emitted, every field here is absent
    on the baseline side, and reporting that as a mass regression would bury the
    tier before it said anything true.
    """
    regressions, notices, improvements = [], [], []
    cur_papers = current.get("papers", {})
    for pid, base in sorted(baseline.get("papers", {}).items()):
        cur = cur_papers.get(pid)
        if cur is None:
            continue
        for field, (direction, tol) in sorted(tols.items()):
            b, c = base.get(field), cur.get(field)
            if b is None or c is None:
                continue
            db, dc = _deviation(field, b), _deviation(field, c)
            if dc > db + tol:
                msg = (f"{pid}: {field} {c:.3f} vs baseline {b:.3f} "
                       f"({direction}, tol {tol})")
                (regressions if field in gated else notices).append(msg)
            elif dc < db - tol:
                improvements.append(f"{pid}: {field} {c:.3f} vs baseline {b:.3f} (improved)")
    return regressions, notices, improvements


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


def measurable_baseline_papers(baseline):
    """Baseline papers that CAN be gated: they carry metrics and their truth
    render succeeded. Papers whose truth render failed can never appear in a run.
    """
    return {
        pid
        for pid, base in baseline.get("papers", {}).items()
        if base.get("word_recall") is not None
        and base.get("status") != "truth_render_failed"
    }


def unchecked_papers(current, baseline):
    """Gateable baseline papers this run did NOT measure — the gate's blind spot.

    Distinct from UNMEASURED (no truth render, unmeasurable by anyone): these are
    papers the gate COULD have checked and simply did not, because the run did
    not include them. `visual_test.py --papers` defaults to the 5 PINNED papers,
    so a bare `./scripts/fidelity_gate.sh` — the documented pre-release command —
    checks 5 papers against a 71-paper baseline and skips the corpus-score
    comparison entirely. That is a defensible default for a quick check and a
    dangerous one for a release gate, so it has to be impossible to miss: a gate
    that silently covers 7% of the corpus reads exactly like one that covers all
    of it. Use `--all` to measure the whole corpus.
    """
    return sorted(measurable_baseline_papers(baseline) - set(current.get("papers", {})))


def covers_whole_corpus(current, baseline):
    """True when `current` measured every paper the baseline has metrics for."""
    return not unchecked_papers(current, baseline)


def baseline_vintages(baseline):
    """{version: [paper_ids]} over the baseline — how many code versions it mixes."""
    out = {}
    for pid, base in baseline.get("papers", {}).items():
        out.setdefault(base.get("byetex_version") or "unknown (pre-dates stamping)",
                       []).append(pid)
    return {v: sorted(ids) for v, ids in sorted(out.items())}


def evaluate(current, baseline, score_tol, recall_tol):
    """Return (regressions, improvements) as lists of human-readable strings."""
    regressions, improvements = [], []

    cur_papers_all = current.get("papers", {})
    # The corpus `fidelity_score` is a weighted mean over the papers a run
    # measured, so it is only comparable when the run covers the same
    # population as the baseline. `fidelity_gate.sh --papers a b c` — the way a
    # targeted change is checked — measures a handful, and comparing that score
    # against the whole-corpus number fails the gate whenever the chosen papers
    # sit below the corpus average, regardless of what the change did. The
    # per-paper checks below are the meaningful signal for a subset run.
    is_full_run = covers_whole_corpus(current, baseline)

    cur_score = current.get("fidelity_score")
    base_score = baseline.get("fidelity_score")
    if base_score is not None and cur_score is not None and is_full_run:
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
    ap.add_argument(
        "--gate-layout", default="", metavar="FIELD[,FIELD...]",
        help="layout-tier properties that may FAIL the build. Default: EMPTY — the "
             "tier reports only. `all` promotes everything (escape hatch). Promote "
             "one property per PR, each backed by measured floors.",
    )
    ap.add_argument("--max-notices", type=int, default=10, metavar="N",
                    help="cap on reported layout notices (default 10)")
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
    unchecked = unchecked_papers(current, baseline)
    if unchecked:
        n_cur = len(current.get("papers", {}))
        n_gateable = len(measurable_baseline_papers(baseline))
        print(
            f"  ⚠ PARTIAL RUN — measured {n_cur} of {n_gateable} gateable papers; "
            f"{len(unchecked)} NOT CHECKED ({100 * len(unchecked) / max(n_gateable, 1):.0f}% "
            "of the corpus is invisible to this run)."
        )
        print(
            "    The corpus fidelity_score is a mean over the papers measured, so it is NOT "
            "compared against the whole-corpus baseline. Per-paper checks below still gate, "
            "but only for the papers above."
        )
        print(f"    Re-run with `--all` to gate the whole corpus. Skipped: "
              f"{', '.join(unchecked[:8])}{' …' if len(unchecked) > 8 else ''}")

    # Honesty surface: papers whose TRUTH render failed have no metrics (word_recall=None),
    # so they contribute nothing to the score and CANNOT register a regression — the gate is
    # blind to them. Name them explicitly so a green gate is never mistaken for "all papers
    # are fine" (health-check P2). These are typically book/thesis classes tectonic can't build.
    unmeasured = sorted(
        pid
        for pid, base in baseline.get("papers", {}).items()
        if base.get("word_recall") is None or base.get("status") == "truth_render_failed"
    )
    if unmeasured:
        print(f"  UNMEASURED (not gated — no truth render, {len(unmeasured)}):")
        for pid in unmeasured:
            print(f"    ? {pid}")

    vintages = baseline_vintages(baseline)
    if len(vintages) > 1:
        print(f"  ⚠ MIXED-VINTAGE BASELINE — entries come from {len(vintages)} different "
              "byetex versions, so 'regression vs baseline' compares against an "
              "inconsistent reference:")
        for ver, ids in vintages.items():
            print(f"    {ver}: {len(ids)} paper(s)")
        print("    Fix with a single whole-corpus `--all` run, then --update-baseline.")

    if improvements:
        print("  IMPROVED (consider `--update-baseline`):")
        for i in improvements:
            print(f"    + {i}")
    # ── layout tier ─────────────────────────────────────────────────────────
    gated = (set(LAYOUT_TOLERANCES) if args.gate_layout.strip() == "all"
             else {f.strip() for f in args.gate_layout.split(",") if f.strip()})
    unknown = gated - set(LAYOUT_TOLERANCES)
    if unknown:
        ap.error(f"--gate-layout: unknown field(s) {sorted(unknown)}; "
                 f"choose from {sorted(LAYOUT_TOLERANCES)} or `all`")
    l_regs, l_notices, l_imps = evaluate_layout(current, baseline, gated, LAYOUT_TOLERANCES)

    measured = sum(1 for p in current.get("papers", {}).values()
                   if p.get("anchor_recall") is not None or p.get("ordered_recall") is not None)
    n_cur = len(current.get("papers", {}))
    if n_cur and measured * 2 < n_cur:
        # A blind tier must be LOUD about being blind. Silently returning None on
        # every paper is indistinguishable from "nothing to report".
        print(f"  LAYOUT: not computed on {n_cur - measured}/{n_cur} papers "
              "— re-run visual_test.py (and `uv run --with pymupdf` for the "
              "geometric profile).")
    if l_notices or l_regs:
        label = ("LAYOUT DRIFT (report-only — promote with --gate-layout)"
                 if not gated else f"LAYOUT DRIFT (gated: {', '.join(sorted(gated))})")
        print(f"  {label}:")
        for n in l_notices[:args.max_notices]:
            print(f"    ~ {n}")
        if len(l_notices) > args.max_notices:
            print(f"    … {len(l_notices) - args.max_notices} more")
    if l_imps:
        print(f"  LAYOUT IMPROVED ({len(l_imps)}; consider `--update-baseline`):")
        for i in l_imps[:args.max_notices]:
            print(f"    + {i}")

    if regressions or l_regs:
        print("  REGRESSION:", file=sys.stderr)
        for r in l_regs:
            print(f"    - {r}", file=sys.stderr)
        for r in regressions:
            print(f"    - {r}", file=sys.stderr)
        print("FAIL: render fidelity regression.", file=sys.stderr)
        return 1
    print("OK: no fidelity regression.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
