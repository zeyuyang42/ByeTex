#!/usr/bin/env python3
"""Unit tests for the corpus-ingestion truth gate (scripts/corpus_add_local.py).

The gate decides whether a paper may be accepted into the corpus based on whether its
original LaTeX renders a truth PDF (the fidelity DRIVER's reference). Truth-first rule:
no silent unmeasured "passes".

Run: python3 scripts/tests/corpus_truth_gate_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import corpus_add_local as cal  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


# (render_ok, reason, tectonic_present, allow_no_truth) -> (accept, status, error)
check(
    cal.decide_truth_gate(True, None, True, False) == (True, "ok", None),
    "truth renders → accept, status ok",
)
check(
    cal.decide_truth_gate(False, "font X not found", True, False)
    == (False, "failed", "font X not found"),
    "truth fails + no override → REJECT with reason",
)
check(
    cal.decide_truth_gate(False, "font X not found", True, True)
    == (True, "failed", "font X not found"),
    "truth fails + --allow-no-truth → accept but record failed + reason",
)
check(
    cal.decide_truth_gate(False, "boom", False, False) == (True, "unverified", None),
    "tectonic absent → can't gate, accept as unverified",
)

print(f"\n{len(fails)} failure(s)" if fails else "\nall passed")
sys.exit(1 if fails else 0)
