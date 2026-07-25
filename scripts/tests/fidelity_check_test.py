#!/usr/bin/env python3
"""Unit tests for the fidelity-gate comparator (scripts/fidelity_check.py).

Run: python3 scripts/tests/fidelity_check_test.py

Focus: the corpus `fidelity_score` is a weighted mean over the papers a run
measured, so comparing a PARTIAL run (`fidelity_gate.sh --papers a b c`) against
a whole-corpus baseline score compares two different populations. A targeted
subset of below-average papers then fails the gate no matter what the change
did. The per-paper checks are the meaningful signal for a subset run and already
skip papers absent from the run.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import fidelity_check as fc  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def paper(word_recall, structure_ok=True):
    return {
        "word_recall": word_recall,
        "heading_recall": 1.0,
        "mean_ssim": 0.5,
        "page_ratio": 1.0,
        "status": "ok",
        "structure_ok": structure_ok,
    }


BASELINE = {
    "fidelity_score": 0.827,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}


def run(current):
    return fc.evaluate(current, BASELINE, score_tol=0.02, recall_tol=0.05)


# ── a partial run must not be gated on the corpus score ──────────────────────
partial = {"fidelity_score": 0.794, "papers": {"c": paper(0.80)}}
regs, _ = run(partial)
check(
    not any("fidelity_score" in r for r in regs),
    "partial run: corpus score is NOT compared (different populations)",
)
check(regs == [], f"partial run with unchanged per-paper metrics is clean, got {regs}")

# ── but a real per-paper regression in that same partial run still fails ─────
partial_bad = {"fidelity_score": 0.794, "papers": {"c": paper(0.70)}}
regs, _ = run(partial_bad)
check(
    any("word_recall" in r for r in regs),
    "partial run: a per-paper word_recall drop still registers",
)

# ── a FULL run is still gated on the corpus score ────────────────────────────
full_bad = {
    "fidelity_score": 0.700,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}
regs, _ = run(full_bad)
check(
    any("fidelity_score" in r for r in regs),
    "full run: a corpus-score drop still registers",
)

full_good = {
    "fidelity_score": 0.900,
    "papers": {"a": paper(0.95), "b": paper(0.90), "c": paper(0.80)},
}
_, imps = run(full_good)
check(
    any("fidelity_score" in i for i in imps),
    "full run: a corpus-score rise is still reported as an improvement",
)

# ── a run covering every MEASURED baseline paper counts as full even when the
#    baseline also holds unmeasured (truth_render_failed) entries ─────────────
baseline_with_unmeasured = {
    "fidelity_score": 0.827,
    "papers": {
        "a": paper(0.95),
        "b": paper(0.90),
        "c": paper(0.80),
        "u": {"word_recall": None, "status": "truth_render_failed"},
    },
}
regs, _ = fc.evaluate(full_bad, baseline_with_unmeasured, 0.02, 0.05)
check(
    any("fidelity_score" in r for r in regs),
    "unmeasured baseline papers don't make a full run look partial",
)

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
