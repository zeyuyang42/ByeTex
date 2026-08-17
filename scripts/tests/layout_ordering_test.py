#!/usr/bin/env python3
# requires: pymupdf numpy scikit-image Pillow
"""The ordering property — the one SSIM violates.

Run: uv run --with pymupdf --with numpy --with scikit-image --with Pillow \
         python scripts/tests/layout_ordering_test.py
Skips cleanly (prints SKIP, exits 0) without typst / pdftotext / pdftoppm /
pymupdf / numpy / scikit-image.

A fidelity signal is only useful if it ranks REAL drift above LEGITIMATE
variation. That is a weaker requirement than accuracy and a much stronger one
than correlation, and it is exactly the property the harness's pixel channel
fails:

    pearson(truth ink coverage, mean_ssim) = -0.913

SSIM is very nearly a pure function of how blank the truth page is. Measured on
this same fixture family, a 1-column -> 2-column collapse scores 0.602 while a
benign font swap scores 0.543 — the ordering is INVERTED, so the 0.20 SSIM
weight in FIDELITY_WEIGHTS actively rewards the wrong thing.

This file asserts the property for `layout_severity` AND asserts its absence for
`mean_ssim`, so the motivating measurement lives on as an executable test rather
than a paragraph in a research document. If a future change makes SSIM order
correctly, the control fails and says so.
"""
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _layout_fixtures as fx  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import layout_metrics as lm  # noqa: E402

missing = fx.missing_tools(("pdftoppm",))
for mod in ("numpy", "skimage", "PIL"):
    try:
        __import__(mod)
    except ImportError:
        missing.append(mod)
if missing:
    print(f"SKIP: layout_ordering — missing {', '.join(missing)}")
    sys.exit(0)

import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402
from skimage.metrics import structural_similarity as ssim  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


# ── a verbatim reimplementation of visual_test.page_image_similarity ─────────
# Deliberately verbatim, including the page-i-to-page-i pairing and the resize
# to a common size. The point is to measure what the harness ACTUALLY computes,
# not an improved version of it.
def rasterize(pdf: Path, out_dir: Path, stem: str) -> list[Path]:
    subprocess.run(["pdftoppm", "-r", "110", "-png", str(pdf), str(out_dir / stem)],
                   check=True, capture_output=True)
    return sorted(out_dir.glob(f"{stem}-*.png"))


def mean_ssim(truth_pages: list[Path], typst_pages: list[Path]) -> float:
    n = min(len(truth_pages), len(typst_pages))
    per_page = []
    for i in range(n):
        a = Image.open(truth_pages[i]).convert("L")
        b = Image.open(typst_pages[i]).convert("L")
        w, h = min(a.width, b.width), min(a.height, b.height)
        per_page.append(float(ssim(np.asarray(a.resize((w, h))), np.asarray(b.resize((w, h))))))
    return round(sum(per_page) / len(per_page), 3) if per_page else 0.0


def separates(scores: dict[str, float], legit: list[str], real: list[str], higher_is_worse: bool):
    """Is there ANY threshold splitting legit from real? Returns (bool, margin)."""
    lo = [scores[n] for n in legit]
    hi = [scores[n] for n in real]
    if higher_is_worse:
        return max(lo) < min(hi), min(hi) - max(lo)
    return min(lo) > max(hi), min(lo) - max(hi)


manifest = fx.load_manifest()
print(f"building {len(manifest['variants']) + 2} PDFs …")
pdfs = fx.build_all(manifest)
base = pdfs["base"]

# The legit side is variants that are benign in EVERY respect. `different_words`
# is kind: legit only in the layout sense — its whole job is to change content —
# so it is the orthogonality control, not a benign baseline.
legit = [v["name"] for v in manifest["variants"]
         if v["kind"] == "legit" and not v["expect"].get("must_fire")]
real = [v["name"] for v in manifest["variants"] if v["kind"] == "real"]
print(f"legit: {legit}\nreal:  {real}\n")

severity = {n: lm.layout_severity(fx.metrics_for(base, pdfs[n])) for n in legit + real}

print("rasterizing for SSIM …")
with tempfile.TemporaryDirectory(dir=fx.WORK_DIR) as td:
    tmp = Path(td)
    base_png = rasterize(base, tmp, "base")
    ssim_scores = {n: mean_ssim(base_png, rasterize(pdfs[n], tmp, n)) for n in legit + real}

print(f"\n{'variant':22s} {'kind':6s} {'severity':>10s} {'mean_ssim':>10s}")
for n in legit + real:
    print(f"{n:22s} {'legit' if n in legit else 'real':6s} "
          f"{severity[n]:10.2f} {ssim_scores[n]:10.3f}")

# ═════════════════════════════════════════════════════════════════════════════
# the property
# ═════════════════════════════════════════════════════════════════════════════
print("\n── layout_severity orders real drift above legitimate variation ──")
for r in real:
    for lg in legit:
        check(severity[r] > severity[lg],
              f"severity({r}) {severity[r]:.2f} > severity({lg}) {severity[lg]:.2f}")

ok, margin = separates(severity, legit, real, higher_is_worse=True)
check(ok, f"a SEPARATING THRESHOLD exists: max legit {max(severity[n] for n in legit):.2f} "
          f"< min real {min(severity[n] for n in real):.2f}")
print(f"\n   separating margin = {margin:.2f} severity units "
      f"({min(severity[n] for n in real) / max(max(severity[n] for n in legit), 1e-9):.2f}x)")
print("   ^ watch this number: a SHRINKING margin is the warning that arrives")
print("     before the ordering inverts, which is how SSIM failed silently.")

# ═════════════════════════════════════════════════════════════════════════════
# the control that makes this test worth its weight
# ═════════════════════════════════════════════════════════════════════════════
print("\n── control: mean_ssim does NOT have this property ──")
ssim_ok, ssim_margin = separates(ssim_scores, legit, real, higher_is_worse=False)
check(not ssim_ok,
      f"control: mean_ssim CANNOT separate real drift from legitimate variation "
      f"(min legit {min(ssim_scores[n] for n in legit):.3f} is not above "
      f"max real {max(ssim_scores[n] for n in real):.3f}; margin {ssim_margin:+.3f})")

inversions = [(r, lg) for r in real for lg in legit if ssim_scores[r] > ssim_scores[lg]]
check(bool(inversions),
      f"control: {len(inversions)} real/legit pairs are INVERTED under mean_ssim "
      f"(real scores MORE similar than legitimate variation), e.g. "
      + ", ".join(f"{r} {ssim_scores[r]:.3f} > {lg} {ssim_scores[lg]:.3f}"
                  for r, lg in inversions[:3]))

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
