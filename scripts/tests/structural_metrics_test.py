#!/usr/bin/env python3
"""Unit tests for the Phase-2a structural fidelity metrics in visual_test.py:
  - word_count_ratio: typst/truth token COUNT ratio (catches deletion/duplication
    that the set-based word_recall/word_jaccard miss)
  - heading_sequence_score: ordered (LCS) heading alignment (catches reordered or
    flattened structure that the set-based heading_recall misses)
  - float_count_ratio: figures/tables caption-count ratio (catches dropped floats
    invisible to word/heading metrics)

Run: uv run python scripts/tests/structural_metrics_test.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def approx(a, b, eps=1e-6) -> bool:
    return a is not None and b is not None and abs(a - b) < eps


# ── word_count_ratio ──────────────────────────────────────────────────────────
# Set-based recall is blind to deletion when the deleted words appear elsewhere;
# a COUNT ratio is not. Tokenization matches tokenize_words (letters, len>=3).
truth = "alpha beta gamma alpha beta gamma"          # 6 tokens
check(approx(vt.word_count_ratio(truth, truth), 1.0), "identical text -> count ratio 1.0")

# Half the body deleted -> ratio ~0.5, even though every unique word still present.
half = "alpha beta gamma"                            # 3 tokens, same word SET
check(approx(vt.word_count_ratio(truth, half), 0.5), "deleted half body -> count ratio 0.5 (set recall would be 1.0)")
# Sanity: set-based recall really IS blind here (documents WHY we need the count).
check(approx(vt.word_recall_set(truth, half), 1.0), "set recall is 1.0 for the same case (blind spot)")

# Duplicated body (e.g. leaked author block) -> ratio ~2.0.
dup = truth + " " + truth
check(approx(vt.word_count_ratio(truth, dup), 2.0), "duplicated body -> count ratio 2.0")

# Empty truth -> ratio None (undefined), never a crash / div-by-zero.
check(vt.word_count_ratio("", "anything here") is None, "empty truth -> None")
check(vt.word_count_ratio("", "") is None, "both empty -> None")


# ── heading_sequence_score ──────────────────────────────────────────────────────
# Ordered alignment via LCS, using the same _heading_match as heading_recall.
a = ["introduction", "method", "results", "conclusion"]
check(approx(vt.heading_sequence_score(a, a), 1.0), "identical heading order -> 1.0")

# Same SET but reordered -> recall stays 1.0, sequence score drops.
reordered = ["method", "introduction", "conclusion", "results"]
check(vt.heading_recall(a, reordered) == 1.0, "heading_recall blind to reorder (1.0)")
seq = vt.heading_sequence_score(a, reordered)
check(seq is not None and seq < 1.0, f"sequence score penalizes reorder (<1.0, got {seq})")

# A dropped middle heading -> LCS keeps the in-order survivors.
dropped = ["introduction", "results", "conclusion"]   # lost "method"
check(approx(vt.heading_sequence_score(a, dropped), 0.75), "one dropped heading -> 3/4 in-order")

# Empty truth headings -> 1.0 (nothing to recall), matching heading_recall convention.
check(approx(vt.heading_sequence_score([], ["x"]), 1.0), "no truth headings -> 1.0")


# ── float_count_ratio ───────────────────────────────────────────────────────────
# Counts "Figure N" / "Table N" caption mentions in PDF-extracted text.
truth_txt = "See Figure 1 and Figure 2. Table 1 summarizes. Also Figure 3."
# 3 figures, 1 table in truth.
same = "Figure 1 here, Figure 2 there, Figure 3 yonder; Table 1 below."
r = vt.float_count_ratio(truth_txt, same)
check(approx(r["figure_ratio"], 1.0), f"all floats present -> figure_ratio 1.0 (got {r})")
check(approx(r["table_ratio"], 1.0), f"all floats present -> table_ratio 1.0 (got {r})")

# Drop one figure in the typst render -> figure_ratio < 1.0.
missing = "Figure 1 here. Table 1 below."   # only 1 distinct figure
r2 = vt.float_count_ratio(truth_txt, missing)
check(r2["figure_ratio"] is not None and r2["figure_ratio"] < 1.0,
      f"dropped figures -> figure_ratio < 1.0 (got {r2})")

# No floats in truth -> ratios None (undefined), not a crash.
r3 = vt.float_count_ratio("plain prose only", "Figure 1")
check(r3["figure_ratio"] is None and r3["table_ratio"] is None, "no truth floats -> None ratios")


if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
