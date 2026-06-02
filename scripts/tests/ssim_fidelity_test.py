#!/usr/bin/env python3
"""Unit tests for the SSIM fidelity metric in visual_test.py:
  - page_image_similarity: per-page SSIM over rasterized page images
  - aggregate_fidelity_score: single corpus-wide fidelity number

Run: uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/tests/ssim_fidelity_test.py
"""
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import visual_test as vt  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def _page(path: Path, fill: int, box: bool) -> Path:
    img = Image.new("L", (120, 160), color=fill)
    if box:
        ImageDraw.Draw(img).rectangle([20, 30, 90, 120], fill=0)
    img.save(path)
    return path


with tempfile.TemporaryDirectory() as td:
    d = Path(td)
    a = _page(d / "a.png", 255, box=True)
    a_copy = _page(d / "a_copy.png", 255, box=True)   # identical to a
    blank = _page(d / "blank.png", 255, box=False)    # clearly different

    # Identical pages → SSIM == 1.0, all pages compared.
    r = vt.page_image_similarity([a], [a_copy])
    check(r["pages_compared"] == 1, "pages_compared counts the aligned pair")
    check(abs(r["mean_ssim"] - 1.0) < 1e-6, f"identical pages -> mean_ssim==1.0 (got {r['mean_ssim']})")

    # Visibly different pages → SSIM well below 1.0.
    r2 = vt.page_image_similarity([a], [blank])
    check(r2["mean_ssim"] < 0.95, f"different pages -> mean_ssim<0.95 (got {r2['mean_ssim']})")

    # Page-count drift → compare only up to the shorter list.
    r3 = vt.page_image_similarity([a, a], [a_copy])
    check(r3["pages_compared"] == 1, "mismatched page counts -> pages_compared = min(len)")

    # No pages → degrade to 0 pages compared, no crash.
    r4 = vt.page_image_similarity([], [])
    check(r4["pages_compared"] == 0, "empty inputs -> 0 pages compared")


# page_closeness: symmetric 1.0-at-match, penalizing over- AND under-running.
check(vt.page_closeness(1.0) == 1.0, "page_closeness(1.0) == 1.0 (perfect)")
check(abs(vt.page_closeness(1.25) - 0.8) < 1e-9, "page_closeness(1.25) == 0.8 (over-run)")
check(abs(vt.page_closeness(0.8) - 0.8) < 1e-9, "page_closeness(0.8) == 0.8 (under-run, symmetric)")
check(vt.page_closeness(None) is None and vt.page_closeness(0) is None,
      "page_closeness(None/0) -> None")

# aggregate_fidelity_score:
#   0.35*word_recall + 0.25*heading_recall + 0.2*mean_ssim + 0.2*page_closeness
papers_perfect = {
    "a": {"word_recall": 1.0, "heading_recall": 1.0, "mean_ssim": 1.0, "page_ratio": 1.0}
}
check(vt.aggregate_fidelity_score(papers_perfect) == 1.0, "all-1.0 metrics -> fidelity 1.0")

# page_ratio 1.25 -> page_closeness 0.8.
# 0.35*0.8 + 0.25*0.6 + 0.2*0.5 + 0.2*0.8 = 0.28 + 0.15 + 0.10 + 0.16 = 0.69
papers_mixed = {
    "a": {"word_recall": 0.8, "heading_recall": 0.6, "mean_ssim": 0.5, "page_ratio": 1.25}
}
check(vt.aggregate_fidelity_score(papers_mixed) == 0.69, "weighted blend computes correctly (0.69)")

# A missing page_ratio also disqualifies a paper (skipped, not scored 0).
papers_no_pages = {"a": {"word_recall": 1.0, "heading_recall": 1.0, "mean_ssim": 1.0}}
check(vt.aggregate_fidelity_score(papers_no_pages) is None,
      "paper without page_ratio is skipped (no full set)")

# Papers missing any metric are skipped, not counted as 0.
papers_partial = {
    "a": {"word_recall": 1.0, "heading_recall": 1.0, "mean_ssim": 1.0, "page_ratio": 1.0},
    "b": {"word_recall": 0.0, "heading_recall": None, "mean_ssim": None, "page_ratio": 2.0},
}
check(vt.aggregate_fidelity_score(papers_partial) == 1.0, "papers missing metrics are skipped")

check(vt.aggregate_fidelity_score({}) is None, "no papers with full metrics -> None")

if fails:
    print(f"\nTEST FAILED ({len(fails)} assertion(s))")
    sys.exit(1)
print("\nTEST PASSED")
