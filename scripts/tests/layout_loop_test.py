#!/usr/bin/env python3
# requires: none
"""Unit tests for the layout loop surface (scripts/layout_loop.py).

Run: python3 scripts/tests/layout_loop_test.py

Bare `python3` — `rank_layout_offenders` is pure, taking a papers dict and a
floors dict, so the ranking that drives the autonomous loop is testable without
a corpus, a PDF, or pymupdf.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import layout_loop as ll  # noqa: E402
import layout_metrics as lm  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


FLOORS = {"properties": {
    "layout_text_width_ratio": {"proposed_threshold": 0.05},
    "layout_leading_ratio": {"proposed_threshold": 0.07},
    "layout_body_font_ratio": {"proposed_threshold": 0.075},
}}

PAPERS = {
    "clean":      {"layout_text_width_ratio": 1.00, "layout_leading_ratio": 1.00},
    "mild":       {"layout_text_width_ratio": 1.10, "layout_leading_ratio": 1.00},
    "bad":        {"layout_text_width_ratio": 1.45, "layout_leading_ratio": 1.60},
    "unmeasured": {"word_recall": 0.9},  # the tier never ran on this paper
}

ranked = ll.rank_layout_offenders(PAPERS, FLOORS)
by_id = {r["paper_id"]: r for r in ranked}

print("── ranking ──")
check([r["paper_id"] for r in ranked][:3] == ["bad", "mild", "clean"],
      f"papers order by severity descending, got {[r['paper_id'] for r in ranked]}")
check(by_id["clean"]["severity"] == 0.0 and by_id["clean"]["reason"] == "within floor",
      f"a clean paper is severity 0, not omitted, got {by_id['clean']}")
# Worst = largest EXCESS OVER ITS OWN FLOOR, not largest raw deviation. `bad` has
# leading off by 0.60 and text width off by 0.45, but their floors are 0.07 and
# 0.05, so text width is 9.0x over versus leading's 8.6x. Ranking by raw
# deviation would let a property with a naturally wide floor always win and send
# every tick to the same file.
check(by_id["bad"]["worst_property"] == "layout_text_width_ratio",
      f"the WORST property is the largest excess over ITS OWN floor, "
      f"got {by_id['bad']['worst_property']}")
check(by_id["bad"]["drift_class"] == "text_width",
      f"...and carries its drift_class for routing, got {by_id['bad']['drift_class']}")
check(by_id["bad"]["properties_out_of_floor"][0]["excess"]
      > by_id["bad"]["properties_out_of_floor"][1]["excess"],
      "fired properties are ordered by excess so the first is the lead")

# ── the departure from dogfood.rank_candidates that matters ────────────────
# A paper whose tier did not run must SURFACE, not vanish. The books and theses
# whose truth render fails carry the corpus's worst geometry; a ranking that
# silently omits them reports a healthier corpus than exists.
check("unmeasured" in by_id, "a paper the tier never ran on is NOT dropped from the ranking")
check(by_id["unmeasured"]["severity"] is None
      and by_id["unmeasured"]["reason"] == "unmeasured",
      f"...it is surfaced as severity None / 'unmeasured', never as 0 "
      f"(which would read as CLEAN), got {by_id['unmeasured']}")
check(ranked[-1]["paper_id"] == "unmeasured",
      "unmeasured papers sort last so they never crowd out real offenders")

# ── purity and determinism ─────────────────────────────────────────────────
print("\n── purity ──")
snapshot = {k: dict(v) for k, v in PAPERS.items()}
ll.rank_layout_offenders(PAPERS, FLOORS)
check(PAPERS == snapshot, "ranking does not mutate its input")
check(ll.rank_layout_offenders(PAPERS, FLOORS) == ranked, "ranking is deterministic")
check(ll.rank_layout_offenders({}, FLOORS) == [], "an empty corpus ranks to an empty list")
check(all(r["severity"] is None for r in ll.rank_layout_offenders(PAPERS, {})),
      "with NO measured floors nothing is 'within floor' — every paper is unmeasured, "
      "because a threshold-free comparison has no meaning")

# ── routing: the table is what makes a tick actionable ─────────────────────
print("\n── routing ──")
missing = [c for c in lm.DRIFT_CLASSES if c not in ll.DRIFT_CLASS_ROUTES]
check(not missing,
      f"every drift_class routes to a suspected site — unrouted: {missing}. "
      "A class with no route turns the backlog into a column of numbers.")
check(all(isinstance(v, tuple) and len(v) == 2 and v[0] and v[1]
          for v in ll.DRIFT_CLASS_ROUTES.values()),
      "each route names a site AND a reason, so it reads as a lead rather than a verdict")

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
