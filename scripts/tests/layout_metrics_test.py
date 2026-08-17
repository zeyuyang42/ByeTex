#!/usr/bin/env python3
# requires: none
"""Unit tests for the pure layout-metric core (scripts/layout_metrics.py).

Run: python3 scripts/tests/layout_metrics_test.py

This file MUST pass under bare `python3` — no pymupdf, no numpy, no Pillow, no
typst, no PDFs. That is the whole point of the module boundary: `page_profile`
and `compare_profiles` take plain span dicts, so the geometry core is testable
without an engine in the loop. No existing test in scripts/tests/ has that
property, and it is why the layout tier can be trusted before it is wired.

Covers, in order:
  * T0 anchors, incl. negative control 8 (false anchors in raw/#link/math/comments)
  * T2 ordered stream, incl. negative control 9 (ligature / soft-hyphen / line-break
    hyphenation normalization must score exactly 1.0)
  * T1 geometry: column detection (the top technical risk, R3), leading on a
    single-line page, font-size tiers, ink, identity
  * import purity: importing the module must not pull in a third-party package
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))  # scripts/
import layout_metrics as lm  # noqa: E402

fails: list[str] = []


def check(cond: bool, desc: str) -> None:
    print(("ok: " if cond else "FAIL: ") + desc)
    if not cond:
        fails.append(desc)


def close(a, b, tol=1e-9) -> bool:
    return a is not None and b is not None and abs(a - b) <= tol


# ═════════════════════════════════════════════════════════════════════════════
# T0 — anchors
# ═════════════════════════════════════════════════════════════════════════════
print("── T0 anchors ──")

TEX = r"""
\section{Intro}\label{sec:intro}
Some prose \ref{sec:intro} and \eqref{eq:rof_dual}.
\begin{equation}\label{eq:rof_dual} x = y \end{equation}
% \label{sec:commented_out} <- inside a comment, must NOT count
\label{tab:main}
"""
labels = lm.tex_labels(TEX)
check(labels == {"sec:intro", "eq:rof_dual", "tab:main"},
      f"tex_labels finds \\label keys and skips % comments, got {sorted(labels)}")

# control 8 — false anchors must not be counted as label DEFINITIONS
TYP = """
= Intro <sec:intro>
Some prose @sec:intro and #link(<eq:rof_dual>)[eq].
$ x < y > z $
`inline raw <tab:notreal>`
```
block raw <tab:alsonotreal>
```
"a string with <tab:stringnotreal>"
#figure(caption: [T])[body] <tab:main>
"""
anchors = lm.typ_anchors(TYP)
check("sec:intro" in anchors and "tab:main" in anchors,
      f"typ_anchors finds real <key> definitions, got {sorted(anchors)}")
check("tab:notreal" not in anchors, "control 8: <key> in inline raw is not an anchor")
check("tab:alsonotreal" not in anchors, "control 8: <key> in a raw block is not an anchor")
check("tab:stringnotreal" not in anchors, "control 8: <key> in a string is not an anchor")
check("eq:rof_dual" not in anchors, "control 8: #link(<key>) is a REFERENCE, not a definition")

# the recall itself, and its key normalization (byetex sanitizes label keys, so
# compare on a punctuation-free normal form or every `:`/`_` rewrite reads as a loss)
r = lm.anchor_recall({"sec:intro", "eq:rof_dual", "tab:main"}, {"sec-intro", "tab:main"})
check(r["anchor_labels_total"] == 3 and r["anchor_matched"] == 2,
      f"anchor_recall counts matched via a punctuation-free normal form, got {r}")
check(close(r["anchor_recall"], 2 / 3), f"anchor_recall = matched/total, got {r['anchor_recall']}")
check(lm.anchor_recall(set(), {"a"})["anchor_recall"] is None,
      "anchor_recall on a paper with zero labels is None, not 1.0 (nothing was measured)")

# ═════════════════════════════════════════════════════════════════════════════
# T2 — ordered stream
# ═════════════════════════════════════════════════════════════════════════════
print("\n── T2 ordered stream ──")

check(lm.normalize_stream("The Quick brown FOX") == ["the", "quick", "brown", "fox"],
      "normalize_stream lowercases and splits on non-alphanumerics")
check("12" in lm.normalize_stream("equation 12 holds"),
      "normalize_stream KEEPS digits (word_recall's [A-Za-z]{3,} throws them away)")

# control 9 — three extraction artifacts that must normalize to the same stream
PLAIN = "the efficient offline workflow was self-evident"
LIGA = "the eﬃcient oﬄine workﬂow was self-evident"  # ffi, ffl, fl
SOFT = "the effi­cient offline workflow was self-evident"      # soft hyphen
BREAK = "the effi-\ncient offline workflow was self-evident"        # line-break hyphenation
base = lm.normalize_stream(PLAIN)
for name, variant in (("ligature", LIGA), ("soft hyphen", SOFT), ("line-break hyphen", BREAK)):
    got = lm.normalize_stream(variant)
    res = lm.ordered_recall(base, got)
    check(close(res["ordered_recall"], 1.0) and close(res["ordered_precision"], 1.0),
          f"control 9: {name} normalizes to an identical stream "
          f"(recall {res['ordered_recall']}, precision {res['ordered_precision']})")

# order is the point: word_recall's set() cannot see a reordering at all
fwd = lm.normalize_stream("alpha beta gamma delta epsilon zeta")
rev = lm.normalize_stream("zeta epsilon delta gamma beta alpha")
check(lm.ordered_recall(fwd, rev)["ordered_recall"] < 0.5,
      "a full reversal craters ordered_recall (a set-based recall would score 1.0)")
check(close(lm.ordered_recall(fwd, fwd)["ordered_recall"], 1.0), "identity stream scores 1.0")

drop = lm.normalize_stream("alpha beta gamma")
res = lm.ordered_recall(fwd, drop)
check(res["ordered_recall"] < 1.0 and close(res["ordered_precision"], 1.0),
      f"dropped content lowers recall but not precision, got {res}")
res = lm.ordered_recall(drop, fwd)
check(close(res["ordered_recall"], 1.0) and res["ordered_precision"] < 1.0,
      f"duplicated/extra content lowers precision but not recall, got {res}")
check(lm.ordered_recall([], [])["ordered_recall"] is None,
      "empty streams yield None, not a fabricated 1.0")

# ═════════════════════════════════════════════════════════════════════════════
# T1 — geometry, on hand-built spans (no PDF anywhere)
# ═════════════════════════════════════════════════════════════════════════════
print("\n── T1 geometry ──")

W, H = 595.0, 842.0  # A4 pt


def span(x0, y0, x1, y1, size=10.0, text="lorem ipsum"):
    return {"bbox": (x0, y0, x1, y1), "size": size, "text": text}


def lines(x0, x1, y_start, n, step=12.0, size=10.0):
    """n text lines stacked at `step` leading."""
    return [span(x0, y_start + i * step, x1, y_start + i * step + 8.0, size) for i in range(n)]


# ── one column ──────────────────────────────────────────────────────────────
one_col = lines(72.0, 523.0, 72.0, 40)
p1 = lm.page_profile(one_col, W, H)
check(p1["ncol"] == 1, f"a single text block is 1 column, got {p1['ncol']}")
check(close(p1["left"], 72.0 / W) and close(p1["top"], 72.0 / H),
      "margins are recorded as page fractions")
check(close(p1["leading"], 12.0), f"leading is the median baseline delta, got {p1['leading']}")

# ── two columns ─────────────────────────────────────────────────────────────
two_col = lines(72.0, 285.0, 72.0, 40) + lines(310.0, 523.0, 72.0, 40)
p2 = lm.page_profile(two_col, W, H)
check(p2["ncol"] == 2, f"a 25pt gutter running the text height is 2 columns, got {p2['ncol']}")

# ── R3, the top technical risk: a full-width table must not fake a gutter ───
# A 1-col page whose table has inter-cell gaps. The gaps exist only over the
# table's y-range, so a 1-D x-projection (the prototype's method) reads them as
# column gutters. The 2-D band test must reject them.
table_rows = []
for i in range(6):
    y = 400.0 + i * 14.0
    table_rows += [span(72.0, y, 180.0, y + 9.0), span(220.0, y, 330.0, y + 9.0),
                   span(380.0, y, 523.0, y + 9.0)]
one_col_table = lines(72.0, 523.0, 72.0, 20) + table_rows + lines(72.0, 523.0, 500.0, 20)
pt = lm.page_profile(one_col_table, W, H)
check(pt["ncol"] == 1,
      f"a full-width TABLE's inter-cell gaps are not column gutters, got {pt['ncol']}")

# ── and the converse: a full-width table crossing a real 2-col gutter must
#    still leave the page detectable as 2 columns ──────────────────────────
spanning = [span(72.0, 400.0, 523.0, 470.0)]  # a float spanning both columns
two_col_span = (lines(72.0, 285.0, 72.0, 25) + lines(310.0, 523.0, 72.0, 25)
                + spanning
                + lines(72.0, 285.0, 500.0, 25) + lines(310.0, 523.0, 500.0, 25))
ps = lm.page_profile(two_col_span, W, H)
check(ps["ncol"] == 2,
      f"a spanning float over part of the height does not collapse a real gutter, got {ps['ncol']}")

# ── a SPARSE page must not shatter into columns (the corpus failure) ────────
# The first detector treated "empty band" as sufficient. A page holding one
# figure and a two-line caption is empty almost everywhere, so nearly every
# x-slice qualified and real papers read as 6 columns:
#   2605.22507  truth [1,1,1,1,1,1,1,2,1,1]  out [1,1,1,1,2,1,2,1,6,1]
# A gutter must be FLANKED by filled columns, and a page with too few lines
# cannot evidence a split at all.
sparse_page = [span(72.0, 700.0, 200.0, 710.0), span(260.0, 700.0, 523.0, 710.0),
               span(72.0, 715.0, 180.0, 725.0)]
check(lm.page_profile(sparse_page, W, H)["ncol"] == 1,
      f"a sparse page (3 lines, big gaps) is 1 column, not shattered — got "
      f"{lm.page_profile(sparse_page, W, H)['ncol']}")

# ── a gutter with an EMPTY side is not a gutter ─────────────────────────────
# Text on the left only, nothing on the right: the band between them is empty,
# but it separates a column from nothing.
one_sided = lines(72.0, 200.0, 72.0, 30)
check(lm.page_profile(one_sided, W, H)["ncol"] == 1,
      f"a band with text on ONE side only is a margin, not a gutter — got "
      f"{lm.page_profile(one_sided, W, H)['ncol']}")

# ── an implausible band count means the heuristic lost, so report 1 ─────────
shredded = []
for i in range(30):
    y = 72.0 + i * 12.0
    for x0 in (72.0, 150.0, 230.0, 310.0, 390.0, 470.0):
        shredded.append(span(x0, y, x0 + 40.0, y + 8.0))
check(lm.page_profile(shredded, W, H)["ncol"] <= lm.MAX_PLAUSIBLE_COLUMNS,
      f"a page that defeats the heuristic reports at most "
      f"{lm.MAX_PLAUSIBLE_COLUMNS}, never a fabricated count — got "
      f"{lm.page_profile(shredded, W, H)['ncol']}")

# ── ragged-right must not read as a gutter (the LEGIT negative control) ─────
import random  # noqa: E402

rnd = random.Random(7)
ragged = [span(72.0, 72.0 + i * 12.0, 523.0 - rnd.uniform(0, 90), 80.0 + i * 12.0)
          for i in range(40)]
pr = lm.page_profile(ragged, W, H)
check(pr["ncol"] == 1, f"ragged-right prose is 1 column, got {pr['ncol']}")

# ── leading on a single-line page: None, never 0 (R3 — a 0 becomes a
#    fabricated ~100% delta once it hits max(lead, 1) in a ratio) ───────────
p_one = lm.page_profile([span(72.0, 72.0, 523.0, 82.0)], W, H)
check(p_one["leading"] is None,
      f"a single-line page has UNDEFINED leading, not 0, got {p_one['leading']}")

# ── font-size tiers ────────────────────────────────────────────────────────
body = lines(72.0, 523.0, 100.0, 30, size=10.0)
small = lines(72.0, 523.0, 500.0, 10, size=8.0)
p_tier = lm.page_profile(body + small, W, H)
check(close(p_tier["body_font"], 10.0),
      f"body_font is the char-weighted MODAL size, not the median span, got {p_tier['body_font']}")
check(0.2 < p_tier["small_tier_share"] < 0.3,
      f"small_tier_share is the char share below the document's own modal size, "
      f"got {p_tier['small_tier_share']}")
p_nosmall = lm.page_profile(body, W, H)
check(close(p_nosmall["small_tier_share"], 0.0), "a uniform page has no small tier")

# ── ink is bbox AREA over page area (R3: the prototype mislabelled a 1-D
#    x-projection as ink, and pearson(ink, ssim) = -0.913 is the headline) ──
p_ink = lm.page_profile([span(0.0, 0.0, W / 2, H / 2)], W, H)
check(close(p_ink["ink"], 0.25, 1e-6),
      f"ink = sum(span bbox area) / page area, got {p_ink['ink']}")

check(lm.page_profile([], W, H) is None, "a page with no text spans profiles as None")

# ═════════════════════════════════════════════════════════════════════════════
# compare_profiles — identity, orthogonality, page-count
# ═════════════════════════════════════════════════════════════════════════════
print("\n── compare_profiles ──")

doc = [lm.page_profile(one_col, W, H) for _ in range(4)]

# ── control 2: identity ─────────────────────────────────────────────────────
ident = lm.compare_profiles(doc, doc)
ratio_keys = [k for k in ident if k.endswith("_ratio") and k != "layout_pages_compared"]
check(len(ratio_keys) >= 6, f"compare emits the ratio family, got {sorted(ident)}")
check(all(close(ident[k], 1.0, 1e-9) for k in ratio_keys),
      "control 2: identity gives EXACTLY 1.0 on every ratio — "
      + ", ".join(f"{k}={ident[k]}" for k in ratio_keys if not close(ident[k], 1.0, 1e-9)))
check(ident["layout_column_mismatch_frac"] == 0.0, "control 2: identity has zero column mismatch")
check(close(ident["layout_fontsize_tier_emd"], 0.0), "control 2: identity has zero tier EMD")
check(close(ident["layout_small_tier_share_delta"], 0.0), "control 2: identity has zero tier delta")

# ── control 7: page-count. Truncating the doc must not move a per-page
#    aggregate. This is the bug that destroyed SSIM (visual_test.py:958-967
#    pairs page i to page i and resizes both to a common size).
trunc = lm.compare_profiles(doc, doc[:2])
check(all(close(trunc[k], 1.0, 1e-9) for k in ratio_keys),
      "control 7: comparing 4 pages against the same 2 pages leaves every "
      "per-page aggregate at 1.0 (page COUNT is a separate signal)")
check(trunc["layout_pages_compared"] == 2,
      f"layout_pages_compared reports the weakness of the evidence, got {trunc['layout_pages_compared']}")

# ── control 6: swap. Same properties fire, ratios invert. ───────────────────
wide = [lm.page_profile(lines(40.0, 555.0, 72.0, 40), W, H) for _ in range(4)]
ab = lm.compare_profiles(doc, wide)
ba = lm.compare_profiles(wide, doc)
check(ab["layout_text_width_ratio"] > 1.0 and ba["layout_text_width_ratio"] < 1.0,
      f"control 6: swapping the arguments inverts the ratio "
      f"({ab['layout_text_width_ratio']} vs {ba['layout_text_width_ratio']})")
check(close(ab["layout_text_width_ratio"] * ba["layout_text_width_ratio"], 1.0, 1e-6),
      "control 6: the two directions are exact reciprocals")

# ── the real signals fire ───────────────────────────────────────────────────
twocol_doc = [lm.page_profile(two_col, W, H) for _ in range(4)]
cm = lm.compare_profiles(doc, twocol_doc)
check(cm["layout_column_mismatch_frac"] == 1.0,
      f"1-col vs 2-col is a total column mismatch, got {cm['layout_column_mismatch_frac']}")
check(close(cm["layout_text_width_ratio"], 1.0, 0.05),
      "control 4/5 orthogonality: a column change alone does not move text WIDTH")

loose = [lm.page_profile(lines(72.0, 523.0, 72.0, 40, step=18.0), W, H) for _ in range(4)]
ld = lm.compare_profiles(doc, loose)
check(close(ld["layout_leading_ratio"], 1.5, 1e-6),
      f"12pt→18pt leading reads as a 1.5 ratio, got {ld['layout_leading_ratio']}")
check(ld["layout_column_mismatch_frac"] == 0.0,
      "control 4/5 orthogonality: a leading change alone does not move the column count")

# ── None-safety: undefined leading on either side must skip, not crash ──────
sparse = [lm.page_profile([span(72.0, 72.0, 523.0, 82.0)], W, H) for _ in range(4)]
sp = lm.compare_profiles(doc, sparse)
check(sp["layout_leading_ratio"] is None,
      f"an undefined leading aggregates to None, not a fabricated number, got {sp['layout_leading_ratio']}")
check(lm.compare_profiles([], doc)["layout_pages_compared"] == 0,
      "comparing against an empty profile is a clean zero, not a crash")

# ═════════════════════════════════════════════════════════════════════════════
# import purity — the module boundary that makes this file runnable
# ═════════════════════════════════════════════════════════════════════════════
print("\n── import purity ──")
THIRD_PARTY = ("fitz", "pymupdf", "numpy", "PIL", "requests", "skimage", "bs4")
leaked = [m for m in THIRD_PARTY if m in sys.modules]
check(not leaked,
      f"importing layout_metrics pulls in NO third-party package (leaked: {leaked})")
check(lm.extract_spans.__module__ == "layout_metrics",
      "extract_spans is the single fitz boundary and lives in this module")

print()
if fails:
    print(f"{len(fails)} FAILED")
    sys.exit(1)
print("all passed")
