#!/usr/bin/env python3
"""Layout-fidelity metrics: the tier the pixel channel could not provide.

Why this exists
───────────────
`scripts/visual_test.py`'s composite score cannot see layout drift, and its
pixel channel is actively misleading. Measured over the corpus
(docs/layout-fidelity-research-2026-08-17.md):

  * pearson(truth ink coverage, mean_ssim) = -0.913 — SSIM is very nearly a pure
    function of how BLANK the truth page is, not of how well we matched it. A
    1-column→2-column collapse scores 0.602; a benign font swap scores 0.543.
    The ordering is inverted, so the 0.20 SSIM weight rewards the wrong thing.
  * `word_recall` is a `set` of `[A-Za-z]{3,}` — every digit and all ordering is
    discarded before comparison.
  * Drift is endemic INSIDE the passing set (only 4/64 papers fail anything):
    leading >8% in 58/64, top margin >20% in 36/64, text width >5% in 32/64.
    The worst case (2605.22315: text column +43%, left margin moved 63pt, both
    figures dropped) has word_recall 0.923 and `structure_ok: true`.

This module measures GEOMETRY and ORDER, and reports NAMED PROPERTIES rather
than a scalar — a scalar is what let the drift hide in the first place.

Three tiers, all pure enough to test without an engine:

  T0 `anchor_recall`   \\label{} keys in the LaTeX vs `<key>` anchors in the .typ.
                       Needs NO truth PDF, so it is the only layout-adjacent
                       signal available for the ~6/71 papers with no truth render.
  T1 geometry          Per-page margins / column count / font tiers / leading /
                       ink, compared truth-PDF vs output-PDF.
  T2 `ordered_recall`  The text stream compared IN ORDER via an LCS, with
                       ligature + hyphenation normalization.

Module boundary (load-bearing)
──────────────────────────────
`page_profile` and `compare_profiles` take plain span dicts, NOT PDFs. The whole
geometry core is therefore testable under bare `python3` with hand-built spans —
no PDF, no typst, no pymupdf. `extract_spans` is the single function that
imports `fitz`, and it imports it LAZILY, so importing this module pulls in no
third-party package at all (asserted by scripts/tests/layout_metrics_test.py).

PyMuPDF is a DEV-ONLY dependency: `uv run --with pymupdf`. It is AGPL and must
never enter `scripts/requirements.txt` or the shipped Rust binary. Every T1
consumer degrades to `None` when it is absent, exactly as
`visual_test.py:page_image_similarity` degrades without numpy/skimage.

Diagnostic CLI:
  uv run --with pymupdf python scripts/layout_metrics.py truth.pdf output.pdf
"""
from __future__ import annotations

import difflib
import json
import re
import statistics as st
import sys
import unicodedata
from pathlib import Path

# ── tuning constants ────────────────────────────────────────────────────────
# All of these are DETECTION parameters (does the property exist?), not
# TOLERANCES (is the difference acceptable?). Tolerances live in
# scripts/layout_floors.json and are measured, never guessed. Keeping the two
# apart is the point: a corpus percentile is a workload description, not a
# tolerance, and must never become a threshold.

GUTTER_MIN_WIDTH_FRAC = 0.015  # a column gutter is ≥1.5% of page width (~9pt A4)
GUTTER_MAX_FILL = 0.40         # ...and is ≤40%-filled over the page's text height
GUTTER_MIN_FLANK_FILL = 0.50   # ...and BOTH sides of it carry ≥50% of that height
MIN_LINES_FOR_COLUMNS = 10     # below this a page cannot evidence a column split
MAX_PLAUSIBLE_COLUMNS = 4      # no academic layout has more; more means noise
FSIZE_BIN = 0.5                # font sizes are quantized to 0.5pt before binning
SMALL_TIER_DROP = 0.75         # "small" = ≥0.75pt below the doc's OWN modal size
LEADING_MIN, LEADING_MAX = 2.0, 40.0  # plausible baseline deltas (pt)
MARGIN_TRIM_PCT = 2.0          # ignore the outer 2% of span edges (stray marginalia)
FULL_PAGE_LINE_FRAC = 0.5      # a "full" page carries ≥50% of the doc's busiest page


# ═══════════════════════════════════════════════════════════════════════════
# T0 — anchors
# ═══════════════════════════════════════════════════════════════════════════

_TEX_COMMENT_RE = re.compile(r"(?<!\\)%.*?$", re.MULTILINE)
_LABEL_RE = re.compile(r"\\label\s*\{([^}]*)\}")


def tex_labels(source_text: str) -> set[str]:
    """Every `\\label{...}` key in the LaTeX source, ignoring `%` comments.

    Pass the output of `visual_test.collect_project_source(toplevel)` — the
    `\\input`-reachable file set. Do NOT pass a `rglob("*.tex")` concatenation:
    corpus papers keep unused drafts beside the real sources, and globbing them
    in invents labels the converter was never asked to emit, silently depressing
    the score. (The tmp/ prototype had exactly this bug.)
    """
    return {m.group(1).strip() for m in _LABEL_RE.finditer(_TEX_COMMENT_RE.sub("", source_text))
            if m.group(1).strip()}


_TYP_RAW_BLOCK_RE = re.compile(r"```.*?```", re.DOTALL)
_TYP_RAW_INLINE_RE = re.compile(r"`[^`\n]*`")
_TYP_STRING_RE = re.compile(r'"(?:[^"\\\n]|\\.)*"')
_TYP_MATH_RE = re.compile(r"\$.*?\$", re.DOTALL)
_TYP_ANCHOR_RE = re.compile(r"(?<!\()<([A-Za-z0-9_:.\-]+)>")


def typ_anchors(typ_text: str) -> set[str]:
    """Every `<key>` LABEL DEFINITION in a Typst document.

    Four things look like `<key>` but are not a definition, and counting any of
    them turns anchor_recall into noise:
      * `#link(<key>)` / `#ref(<key>)` — a REFERENCE (excluded by the `(?<!\\()`
        lookbehind)
      * `<key>` inside raw blocks / inline raw — verbatim listing text
      * `<key>` inside a string literal
      * `a < b > c` inside math — two comparison operators
    """
    t = _TYP_RAW_BLOCK_RE.sub(" ", typ_text)
    t = _TYP_RAW_INLINE_RE.sub(" ", t)
    t = _TYP_MATH_RE.sub(" ", t)
    t = _TYP_STRING_RE.sub(" ", t)
    return {m.group(1) for m in _TYP_ANCHOR_RE.finditer(t)}


def _norm_key(k: str) -> str:
    """A punctuation-free normal form for label-key comparison.

    byetex rewrites LaTeX keys into Typst-safe ones (`sec:intro` may become
    `sec-intro`; PR #452 substitutes `_` through a sentinel). Comparing raw
    strings would score every such rewrite as a LOST label, which is a
    measurement artifact rather than a fidelity loss.
    """
    return re.sub(r"[^a-z0-9]", "", k.lower())


def anchor_recall(labels: set[str], anchors: set[str]) -> dict:
    """Fraction of source `\\label` keys that reached the .typ as an anchor.

    None (not 1.0) when the source declares no labels — nothing was measured,
    and a fabricated 1.0 would dilute every corpus aggregate it enters.
    """
    want = {_norm_key(k) for k in labels if _norm_key(k)}
    have = {_norm_key(k) for k in anchors if _norm_key(k)}
    matched = len(want & have)
    return {
        "anchor_recall": (matched / len(want)) if want else None,
        "anchor_labels_total": len(want),
        "anchor_matched": matched,
    }


# ═══════════════════════════════════════════════════════════════════════════
# T2 — ordered text stream
# ═══════════════════════════════════════════════════════════════════════════

_LIGATURES = {"\ufb00": "ff", "\ufb01": "fi", "\ufb02": "fl", "\ufb03": "ffi",
              "\ufb04": "ffl", "\ufb05": "st", "\ufb06": "st"}
_TOKEN_RE = re.compile(r"[a-z0-9]{2,}")


def normalize_stream(text: str) -> list[str]:
    """Text → an ordered token list, with extraction artifacts normalized away.

    NORMALISE, don't fuzz (the l3build principle): each rule below removes a
    known *rendering* difference that carries no fidelity information, so the
    comparison can then be exact rather than tolerant.

      * NFKC + explicit ligature map — `pdftotext` emits ﬁ/ﬄ inconsistently
        between engines. Roughly HALF of the corpus's measured `word_recall`
        deficit is this artifact and nothing else.
      * soft hyphen (U+00AD) — a discretionary break, never content.
      * `-\\n` — line-break hyphenation is a function of the measure, so it
        differs whenever the column width differs. Text width is measured by T1;
        letting it leak into T2 as well would double-count one cause.

    Digits are KEPT: `word_recall`'s `[A-Za-z]{3,}` throws away every equation
    number, table figure and year before it compares anything.
    """
    t = unicodedata.normalize("NFKC", text)
    for lig, repl in _LIGATURES.items():
        t = t.replace(lig, repl)
    t = t.replace("\u00ad", "")
    t = re.sub(r"-[ \t]*\r?\n[ \t]*", "", t)  # dehyphenate across a line break
    return _TOKEN_RE.findall(t.lower())


def ordered_recall(truth_toks: list[str], out_toks: list[str]) -> dict:
    """Recall + precision of the output stream against truth, IN ORDER.

    One `difflib.SequenceMatcher(autojunk=False)` LCS yields both. `autojunk` is
    off deliberately: its heuristic discards any element appearing in >1% of a
    sequence longer than 200 items, which on a real paper silently drops "the",
    "of", "and" — the exact tokens that pin the alignment.

    Recall and precision separate the two failure modes a single number cannot:
    dropped content lowers recall alone, duplicated/leaked content lowers
    precision alone.
    """
    if not truth_toks or not out_toks:
        return {"ordered_recall": None, "ordered_precision": None}
    sm = difflib.SequenceMatcher(None, truth_toks, out_toks, autojunk=False)
    matched = sum(b.size for b in sm.get_matching_blocks())
    return {
        "ordered_recall": matched / len(truth_toks),
        "ordered_precision": matched / len(out_toks),
    }


def ordered_stream_compare(truth_text: str, out_text: str) -> dict:
    """T2 over two already-extracted text blobs.

    NON-NEGOTIABLE: pass the same `extract_pdf_text()` output that feeds
    `word_recall`. If `ordered_recall` were computed from PyMuPDF spans while
    `word_recall` came from `pdftotext`, the two would differ for EXTRACTION
    reasons — PyMuPDF splits ligatures differently — and someone would read the
    gap as the converter improving. `ordered_extractor` records which one was
    used so that assumption is checkable rather than assumed.
    """
    res = ordered_recall(normalize_stream(truth_text), normalize_stream(out_text))
    res["ordered_extractor"] = "pdftotext"
    return res


# ═══════════════════════════════════════════════════════════════════════════
# T1 — geometry
# ═══════════════════════════════════════════════════════════════════════════

def extract_spans(pdf) -> list[dict] | None:
    """Per-page text spans from a PDF: `[{"bbox","size","text"}, ...]` per page.

    THE ONLY function in this module that touches PyMuPDF, and it imports it
    lazily so the pure core stays importable under bare `python3`. Returns None
    (never raises) when pymupdf is absent or the PDF is unreadable — every
    consumer then reports `layout_pages_compared: None` plus a `skipped` reason,
    mirroring how `visual_test.page_image_similarity` degrades without numpy.
    """
    try:
        import fitz  # noqa: PLC0415  (lazy on purpose — see docstring)
    except ImportError:
        return None
    try:
        doc = fitz.open(str(pdf))
    except Exception:  # noqa: BLE001 — a malformed PDF is data, not a bug
        return None
    pages = []
    with doc:
        for page in doc:
            spans = [
                {"bbox": tuple(s["bbox"]), "size": float(s["size"]), "text": s["text"]}
                for blk in page.get_text("dict")["blocks"] if blk["type"] == 0
                for line in blk["lines"] for s in line["spans"] if s["text"].strip()
            ]
            pages.append({"spans": spans,
                          "page_w": float(page.rect.width),
                          "page_h": float(page.rect.height)})
    return pages


def _pct(values: list[float], q: float) -> float:
    """Linear-interpolated percentile. Trims a single stray marginal note out of
    the margin measurement without needing numpy."""
    if not values:
        return 0.0
    s = sorted(values)
    if len(s) == 1:
        return s[0]
    pos = (q / 100.0) * (len(s) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(s) - 1)
    return s[lo] + (s[hi] - s[lo]) * (pos - lo)


def _column_bands(spans: list[dict], page_w: float) -> list[tuple[float, float]]:
    """The page's text columns as `[(x_lo, x_hi), ...]`, from VERTICAL whitespace
    bands. A 1-column page returns a single band.

    The prototype gap-scanned a 1-D x-projection of span coverage. That method
    has two failure modes that matter, in opposite directions:

      * a full-width table or display equation crosses the gutter, filling the
        projection, and a genuine 2-column page reads as 1;
      * a table's own inter-cell gaps read as gutters, and a 1-column page reads
        as 3.

    Both are the same mistake — a projection cannot tell a horizontal gap from a
    vertical band. A real gutter is a band that PERSISTS down the page, so
    measure how much of the page's text height each x-slice actually carries:
    an x is gutter-like when it is ≤`GUTTER_MAX_FILL` filled. One table row
    crossing it moves the fill by the table's height fraction only, so the band
    survives; and inter-cell gaps, which carry the full prose column above and
    below, do not qualify.

    An empty band is NECESSARY but not SUFFICIENT, and assuming otherwise is how
    the first version of this failed. Validated against synthetic fixtures it was
    perfect; against the corpus it read SIX columns on single-column pages:

        2605.22507  truth [1,1,1,1,1,1,1,2,1,1]   out [1,1,1,1,2,1,2,1,6,1]

    firing on 58 of 65 papers. The synthetic pages are dense prose; real pages
    are not. A page holding one figure and a two-line caption is empty almost
    everywhere, so almost every x-slice qualifies as "gutter-like" and the
    detector shatters it into columns.

    So a gutter must also be FLANKED: the regions either side of it have to carry
    at least `GUTTER_MIN_FLANK_FILL` of the page's text height, because a column
    gutter by definition separates two columns of text. On a sparse page neither
    side is filled and nothing is detected; on a real two-column page each side
    carries ~65% and the gutter survives. Two cheap guards back that up — a page
    with fewer than `MIN_LINES_FOR_COLUMNS` lines cannot evidence a split at all,
    and more than `MAX_PLAUSIBLE_COLUMNS` bands means the page defeated the
    heuristic, so report one column rather than a fabricated number.

    Fill is accumulated with a difference array over x — O(spans + width), no
    rasterization, so this stays cheap enough for the whole corpus.

    Returning the BANDS rather than just a count is what lets leading and line
    count be measured per column; see `page_profile`.
    """
    if not spans or page_w <= 0:
        return [(0.0, page_w)]
    x_lo = min(s["bbox"][0] for s in spans)
    x_hi = max(s["bbox"][2] for s in spans)
    y_lo = min(s["bbox"][1] for s in spans)
    y_hi = max(s["bbox"][3] for s in spans)
    text_h = y_hi - y_lo
    if text_h <= 0 or x_hi - x_lo <= 0:
        return [(x_lo, x_hi)]

    width = int(page_w) + 2
    diff = [0.0] * (width + 1)
    for s in spans:
        x0, y0, x1, y1 = s["bbox"]
        a, b = max(0, int(x0)), min(width - 1, int(x1) + 1)
        if b > a:
            diff[a] += (y1 - y0)
            diff[b] -= (y1 - y0)

    min_gap = max(1, int(GUTTER_MIN_WIDTH_FRAC * page_w))
    lo, hi = int(x_lo), min(int(x_hi) + 1, width - 1)

    # Per-x coverage height, materialized so flanking regions can be inspected.
    fill = [0.0] * (width + 1)
    acc = 0.0
    for x in range(0, width):
        acc += diff[x]
        fill[x] = acc

    # A page with too few lines cannot evidence a column split; guessing from
    # three lines of caption is what produced the six-column readings.
    n_lines = len({round(s["bbox"][3], 1) for s in spans})
    if n_lines < MIN_LINES_FOR_COLUMNS:
        return [(x_lo, x_hi)]

    cuts: list[tuple[int, int]] = []  # (gutter_start, gutter_end) in x
    run = 0
    for x in range(lo, hi + 1):
        if fill[x] <= GUTTER_MAX_FILL * text_h:
            run += 1
        else:
            # An interior run only — a run touching either edge of the text
            # block is a ragged margin (ragged-right prose leaves exactly such a
            # low-fill band), not a gutter.
            if run >= min_gap and (x - run) > lo:
                cuts.append((x - run, x))
            run = 0
    # a trailing run reaches x_hi, so it is by definition NOT interior

    # Flank test: a gutter separates two COLUMNS, so both sides must actually
    # carry text. Without this, an empty page is all gutter.
    def flanked(g0: int, g1: int) -> bool:
        left = max(fill[lo:g0], default=0.0)
        right = max(fill[g1:hi + 1], default=0.0)
        need = GUTTER_MIN_FLANK_FILL * text_h
        return left >= need and right >= need

    cuts = [c for c in cuts if flanked(*c)]
    if len(cuts) + 1 > MAX_PLAUSIBLE_COLUMNS:
        return [(x_lo, x_hi)]

    bands, start = [], x_lo
    for g0, g1 in cuts:
        bands.append((start, float(g0)))
        start = float(g1)
    bands.append((start, x_hi))
    return bands


def page_profile(spans: list[dict], page_w: float, page_h: float) -> dict | None:
    """The geometry a reader actually perceives, for one page.

    PURE: takes span dicts, not a PDF. Returns None for a page with no text —
    the caller keeps the None so page indices stay aligned.
    """
    spans = [s for s in spans if s["text"].strip()]
    if not spans or page_w <= 0 or page_h <= 0:
        return None

    x0s = [s["bbox"][0] for s in spans]
    x1s = [s["bbox"][2] for s in spans]
    left = _pct(x0s, MARGIN_TRIM_PCT)
    right_edge = _pct(x1s, 100.0 - MARGIN_TRIM_PCT)
    top = min(s["bbox"][1] for s in spans)
    bottom_edge = max(s["bbox"][3] for s in spans)

    # Leading and line count are measured PER COLUMN and then combined.
    #
    # Measuring them from one globally-sorted baseline list is wrong on any
    # multi-column page: the two columns' baselines interleave, so consecutive
    # entries come from *different* columns and the deltas between them are
    # sub-line noise rather than the leading. The synthetic `twocol` fixture
    # showed this as a spurious 11% leading change on a variant whose leading is
    # by construction identical — a false positive on the single most common
    # real-world layout in the corpus.
    #
    # Undefined (None) rather than 0 when no column has two baselines: returning
    # 0 is how the prototype fabricated a ~100% delta, because every ratio
    # downstream guards with max(lead, 1).
    bands = _column_bands(spans, page_w)
    col_leads: list[float] = []
    n_lines = 0
    for x_lo, x_hi in bands:
        mid = lambda s: (s["bbox"][0] + s["bbox"][2]) / 2  # noqa: E731
        col = sorted({round(s["bbox"][3], 1) for s in spans if x_lo <= mid(s) <= x_hi})
        n_lines += len(col)
        deltas = [b - a for a, b in zip(col, col[1:]) if LEADING_MIN < b - a < LEADING_MAX]
        if deltas:
            col_leads.append(st.median(deltas))
    leading = st.median(col_leads) if col_leads else None

    # font-size histogram, weighted by CHARACTERS not spans: one 40-char body
    # span and one 2-char superscript are not equal evidence of "the body size".
    hist: dict[float, int] = {}
    for s in spans:
        n = len(s["text"].strip())
        if n:
            hist[round(s["size"] / FSIZE_BIN) * FSIZE_BIN] = (
                hist.get(round(s["size"] / FSIZE_BIN) * FSIZE_BIN, 0) + n)
    body_font = max(hist.items(), key=lambda kv: (kv[1], -kv[0]))[0]
    total_chars = sum(hist.values())
    small_chars = sum(n for sz, n in hist.items() if sz < body_font - SMALL_TIER_DROP)

    return {
        "page_w": page_w,
        "page_h": page_h,
        "left": left / page_w,
        "right": 1.0 - right_edge / page_w,
        "top": top / page_h,
        "bottom": 1.0 - bottom_edge / page_h,
        "text_width": (right_edge - left) / page_w,
        "ncol": len(bands),
        "body_font": body_font,
        "fsize_hist": hist,
        "small_tier_share": (small_chars / total_chars) if total_chars else 0.0,
        "leading": leading,
        "lines": n_lines,
        # ink is span-bbox AREA over page area. The prototype called a 1-D
        # x-projection "ink"; since pearson(ink, ssim) = -0.913 is the headline
        # finding, the quantity behind that name has to be the real one.
        "ink": sum((s["bbox"][2] - s["bbox"][0]) * (s["bbox"][3] - s["bbox"][1])
                   for s in spans) / (page_w * page_h),
    }


def profile_pdf(pdf) -> list[dict] | None:
    """Per-page profiles for a PDF, or None when pymupdf/the PDF is unavailable."""
    pages = extract_spans(pdf)
    if pages is None:
        return None
    return [page_profile(p["spans"], p["page_w"], p["page_h"]) for p in pages]


def _emd(h1: dict[float, int], h2: dict[float, int]) -> float | None:
    """1-D Wasserstein distance (pt) between two font-size distributions.

    A single number for "the type scale moved" that, unlike comparing modal
    sizes, notices a change confined to one tier — which is exactly the live
    converter bug this tier was built to catch (`\\small`/`\\footnotesize`
    flattened into the body size: truth 17.8% of characters at 8.5pt, output
    1.2%, with the body size itself unchanged).
    """
    t1, t2 = sum(h1.values()), sum(h2.values())
    if not t1 or not t2:
        return None
    keys = sorted(set(h1) | set(h2))
    c1 = c2 = emd = 0.0
    for k, nxt in zip(keys, keys[1:]):
        c1 += h1.get(k, 0) / t1
        c2 += h2.get(k, 0) / t2
        emd += abs(c1 - c2) * (nxt - k)
    return emd


def _mean(vals: list[float]) -> float | None:
    return st.fmean(vals) if vals else None


def _ratio(out_vals: list[float], truth_vals: list[float]) -> float | None:
    """out/truth over per-page means. None when either side is undefined or the
    truth side is zero — a fabricated ratio is worse than a missing one."""
    a, b = _mean(out_vals), _mean(truth_vals)
    if a is None or b is None or b == 0:
        return None
    return a / b


def _full_pages(profiles: list[dict | None]) -> list[dict]:
    """The pages worth measuring geometry on: those carrying at least
    `FULL_PAGE_LINE_FRAC` of the busiest page's line count.

    A document's LAST page is typically a stub — in the synthetic fixture, 4
    lines against 44. Its right edge sits wherever the final short line happens
    to end, so its `right` margin reads 0.32 where the body pages read 0.12, and
    including it swamps every aggregate. Selecting by CONTENT rather than by
    position also removes the pairing problem entirely: two documents that
    paginate differently still contribute the same *kind* of page.

    This replaces "drop the final page when there are more than two", which did
    nothing at all for a 2-page document — and 2-page documents are exactly
    where one stub page is half the evidence.
    """
    ps = [p for p in profiles if p is not None]
    if not ps:
        return []
    top = max(p["lines"] for p in ps)
    return [p for p in ps if p["lines"] >= FULL_PAGE_LINE_FRAC * top] or ps[:1]


def compare_profiles(truth: list[dict | None], out: list[dict | None]) -> dict:
    """Truth-vs-output geometry as NAMED PROPERTIES. PURE — no PDFs.

    Ratios (`toward_one`) rather than signed deltas, so a property reads the
    same regardless of page size and so an improvement is visible as |x-1|
    shrinking — two-sided, the Gecko model. A one-sided threshold lets a fix
    overshoot into the opposite error without ever being noticed.

    Aggregation is deliberately split three ways by what each property IS:

      * geometry (margins, widths, leading, ink) is summarized over each side's
        own FULL pages independently, then divided. Index-pairing these would
        make any pagination difference — a duplicated paragraph, a dropped
        figure — read as a margin change, because page 2 of one document is
        then compared against unrelated content in page 2 of the other.
      * type tiers are aggregated over the WHOLE document, because a font tier
        is a document property. Computing them per page makes the small-tier
        share move whenever content reflows onto a different page.
      * column count is genuinely per-page and RELATIONAL, so it alone is
        index-paired.

    Page COUNT is a separate signal (`page_ratio`, already in the harness), kept
    separate on purpose — folding it in here is the mistake that destroyed SSIM,
    which resizes both pages to a common size and then pairs page i to page i.
    """
    t_full, o_full = _full_pages(truth), _full_pages(out)
    t_all = [p for p in truth if p is not None]
    o_all = [p for p in out if p is not None]

    res: dict = {"layout_pages_compared": min(len(t_full), len(o_full))}
    keys = ("page_trim", "left_margin", "right_margin", "top_margin", "text_width",
            "body_font", "leading", "lines_per_page", "ink")
    if not t_full or not o_full:
        res.update({f"layout_{k}_ratio": None for k in keys})
        res.update({"layout_column_mismatch_frac": None,
                    "layout_small_tier_share_delta": None,
                    "layout_fontsize_tier_emd": None})
        return res

    def col(pages: list[dict], field: str) -> list[float]:
        return [p[field] for p in pages if p[field] is not None]

    area = lambda p: p["page_w"] * p["page_h"]  # noqa: E731
    res["layout_page_trim_ratio"] = _ratio([area(p) for p in o_full],
                                           [area(p) for p in t_full])
    for name, field in (("left_margin", "left"), ("right_margin", "right"),
                        ("top_margin", "top"), ("text_width", "text_width"),
                        ("leading", "leading"),
                        ("lines_per_page", "lines"), ("ink", "ink")):
        res[f"layout_{name}_ratio"] = _ratio(col(o_full, field), col(t_full, field))

    # Column count is RELATIONAL and CATEGORICAL — "same or not", no epsilon to
    # pick. That is what makes it the one property gateable without first
    # measuring a tolerance, and so the first slated for promotion.
    pairs = list(zip(t_full, o_full))
    res["layout_column_mismatch_frac"] = (
        sum(t["ncol"] != o["ncol"] for t, o in pairs) / len(pairs) if pairs else None)

    # Font tiers are aggregated to DOCUMENT level before the small-tier share is
    # taken, so "small" is relative to the document's own modal size. An absolute
    # 8.5pt cut would misjudge any document whose body genuinely is 8.5pt.
    th: dict[float, int] = {}
    oh: dict[float, int] = {}
    for pages, dst in ((t_all, th), (o_all, oh)):
        for p in pages:
            for sz, n in p["fsize_hist"].items():
                dst[sz] = dst.get(sz, 0) + n
    res["layout_fontsize_tier_emd"] = _emd(th, oh)

    # Body font is compared as a FRACTION OF THE PAGE, from the DOCUMENT's modal
    # size — not as a mean of per-page modal sizes in absolute points. The naive
    # version was confounded twice over, and manually checking the 10 papers it
    # flagged is what exposed both (the floors doc's promotion criterion (d)):
    #
    #   * Absolute points double-count a page-size change. The four slide decks
    #     read ratio ~2.0 "wrong body size" — but their Typst pages are 2x the
    #     canvas (page_trim 3.4-4.8), so the type is roughly proportional.
    #     Normalising by the page diagonal drops them to 0.88-1.31. The page-size
    #     drift is real and layout_page_trim_ratio already reports it; reporting
    #     it a second time as a font bug sends the fix to the wrong place.
    #
    #   * A mean of per-page modal sizes is not the body size. Six papers read
    #     1.08-1.19 while their document modal was identical (10.0 -> 10.0),
    #     because pages of different composition pull the mean.
    #
    # On documents of equal page size the diagonal cancels and this is exactly
    # the absolute comparison, so nothing is lost for the common case.
    def _doc_modal(hist: dict[float, int]) -> float | None:
        return max(hist.items(), key=lambda kv: (kv[1], -kv[0]))[0] if hist else None

    def _page_scale(pages: list[dict]) -> float | None:
        vals = [(p["page_w"] ** 2 + p["page_h"] ** 2) ** 0.5 for p in pages]
        return _mean(vals)

    t_modal, o_modal = _doc_modal(th), _doc_modal(oh)
    t_scale, o_scale = _page_scale(t_full), _page_scale(o_full)
    if None in (t_modal, o_modal, t_scale, o_scale) or 0 in (t_scale, o_scale, t_modal):
        res["layout_body_font_ratio"] = None
    else:
        res["layout_body_font_ratio"] = (o_modal / o_scale) / (t_modal / t_scale)

    def small_share(h: dict[float, int]) -> float | None:
        tot = sum(h.values())
        if not tot:
            return None
        modal = max(h.items(), key=lambda kv: (kv[1], -kv[0]))[0]
        return sum(n for sz, n in h.items() if sz < modal - SMALL_TIER_DROP) / tot

    ts, os_ = small_share(th), small_share(oh)
    res["layout_small_tier_share_delta"] = None if ts is None or os_ is None else os_ - ts
    return res


def layout_compare(truth_pdf, out_pdf) -> dict:
    """T1 end to end. Degrades to a NAMED skip rather than silent Nones."""
    tp, op = profile_pdf(truth_pdf), profile_pdf(out_pdf)
    if tp is None or op is None:
        res = compare_profiles([], [])
        res["layout_pages_compared"] = None
        res["layout_skipped"] = "install pymupdf for the layout profile"
        return res
    return compare_profiles(tp, op)


# ═══════════════════════════════════════════════════════════════════════════
# drift classes, floors, severity
# ═══════════════════════════════════════════════════════════════════════════

# A `drift_class` is the unit the loop routes on: it names WHAT went wrong in
# words a fix can be aimed at, which a bare property name does not. Several
# properties can evidence one class (a font-tier change shows up in both the
# share delta and the EMD), so the class is the reporting unit and the property
# is the evidence.
DRIFT_CLASSES: dict[str, tuple[str, ...]] = {
    "page_trim":  ("layout_page_trim_ratio",),
    "margin":     ("layout_left_margin_ratio", "layout_right_margin_ratio",
                   "layout_top_margin_ratio"),
    "text_width": ("layout_text_width_ratio",),
    "font_size":  ("layout_body_font_ratio",),
    # Split apart deliberately: the EMD says "the type scale as a whole moved"
    # and the share delta says "a specific tier was lost". Both fire when
    # \small is flattened into the body size, but they route to different fixes,
    # and a class that fires for two unrelated reasons routes to neither.
    "type_scale": ("layout_fontsize_tier_emd",),
    "small_tier": ("layout_small_tier_share_delta",),
    "leading":    ("layout_leading_ratio",),
    # lines-per-page is kept OUT of `leading`: a 2-column page legitimately
    # carries ~2x the lines at identical leading, so folding them together makes
    # every multi-column document look like a leading regression.
    "density":    ("layout_lines_per_page_ratio",),
    "columns":    ("layout_column_mismatch_frac",),
    "ink":        ("layout_ink_ratio",),
    "anchors":    ("anchor_recall",),
    "ordering":   ("ordered_recall", "ordered_precision"),
}

# Classes measured from PAGE GEOMETRY. The rest (`anchors`, `ordering`) are
# content tiers: a document with different words but identical layout must move
# those two and nothing else, which is the orthogonality control.
LAYOUT_CLASSES = tuple(k for k in DRIFT_CLASSES if k not in ("anchors", "ordering"))

# PROVISIONAL floors — replaced by measured values in scripts/layout_floors.json.
#
# These are deliberately WIDE. They are not "how close is close enough"; they
# are "past here it is definitely not measurement noise". The real floors come
# from three measured sources (instrument repeat noise, self-comparison, and the
# synthetic legitimate-variation set), never from a corpus percentile — 58/64
# corpus papers already drift on leading, so a corpus percentile would encode
# that bug as normal and gate nothing.
DEFAULT_FLOORS: dict[str, float] = {
    "layout_page_trim_ratio":        0.02,
    "layout_left_margin_ratio":      0.08,
    "layout_right_margin_ratio":     0.10,
    "layout_top_margin_ratio":       0.08,
    "layout_text_width_ratio":       0.05,
    "layout_body_font_ratio":        0.06,
    "layout_leading_ratio":          0.10,
    "layout_lines_per_page_ratio":   0.15,
    "layout_column_mismatch_frac":   0.001,  # categorical: ANY mismatch is drift
    "layout_ink_ratio":              0.15,
    "layout_small_tier_share_delta": 0.03,
    "layout_fontsize_tier_emd":      0.60,
    "anchor_recall":                 0.05,
    "ordered_recall":                0.05,
    "ordered_precision":             0.05,
}


def property_deviation(name: str, value) -> float | None:
    """How far a property sits from "no drift", in its own units.

    The convention is carried by the property's NAME suffix so a new property
    cannot be added without declaring how it is read:
      `_ratio`     → |v - 1|, two-sided. An overshoot is drift in the same way
                     an undershoot is; a one-sided check lets a fix sail past
                     the target and never be noticed (the Gecko rule).
      `_delta`     → |v|
      `_frac`/`_emd` → v (already a distance from zero)
      `recall`/`precision` → 1 - v
    """
    if value is None:
        return None
    if name.endswith("_ratio"):
        return abs(value - 1.0)
    if name.endswith("_delta"):
        return abs(value)
    if name.endswith(("_frac", "_emd")):
        return abs(value)
    if name.endswith(("recall", "precision")):
        return max(0.0, 1.0 - value)
    return abs(value)


def fired_properties(metrics: dict, floors: dict | None = None) -> list[dict]:
    """Properties beyond their floor, worst first.

    A property that is None on either side is SKIPPED, never counted as passing.
    """
    floors = floors or DEFAULT_FLOORS
    out = []
    for cls, props in DRIFT_CLASSES.items():
        for name in props:
            if name not in metrics:
                continue
            dev = property_deviation(name, metrics.get(name))
            floor = floors.get(name)
            if dev is None or not floor:
                continue
            if dev > floor:
                out.append({"name": name, "drift_class": cls, "value": metrics[name],
                            "deviation": dev, "floor": floor, "excess": dev / floor})
    return sorted(out, key=lambda d: -d["excess"])


def fired_classes(metrics: dict, floors: dict | None = None) -> set[str]:
    """The `drift_class` set a comparison lands in."""
    return {p["drift_class"] for p in fired_properties(metrics, floors)}


def layout_severity(metrics: dict, floors: dict | None = None) -> float | None:
    """A single number for RANKING ONLY — never for gating.

    Deliberately the MAX excess across properties, not the mean: one badly wrong
    property is a real problem, and averaging it against nine healthy ones is
    precisely how the composite fidelity score hid a paper with a +43% text
    column behind a 0.923 word_recall.

    It is not clipped at the floor, so a value below 1.0 still says how much of
    the floor a legitimate variation consumed. That is what makes the separating
    margin between legitimate and real drift measurable, and a SHRINKING margin
    visible before it inverts.
    """
    floors = floors or DEFAULT_FLOORS
    vals = []
    for props in DRIFT_CLASSES.values():
        for name in props:
            dev = property_deviation(name, metrics.get(name))
            floor = floors.get(name)
            if dev is not None and floor:
                vals.append(dev / floor)
    return max(vals) if vals else None


# ═══════════════════════════════════════════════════════════════════════════
# diagnostic CLI
# ═══════════════════════════════════════════════════════════════════════════

def main(argv=None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if len(argv) != 2:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        print("usage: layout_metrics.py <truth.pdf> <output.pdf>", file=sys.stderr)
        return 2
    res = layout_compare(Path(argv[0]), Path(argv[1]))
    print(json.dumps(res, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
