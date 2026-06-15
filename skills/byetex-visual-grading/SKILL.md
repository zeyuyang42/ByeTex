---
name: byetex-visual-grading
description: Grade the VISUAL FIDELITY of a ByeTex conversion against the LaTeX truth — compare truth↔typst page images dimension-by-dimension and emit structured findings. Run `byetex review <paper>` to build the `grading_packet.json` (one command), or use `scripts/visual_test.py` for the corpus, then grade. Use when asked to visually grade/audit how faithfully a `.typ` renders vs the original. NOT for fixing compile errors (use byetex-repair-loop) or warnings (use byetex-using-warnings-json).
---

# byetex: visual fidelity grading

You are grading how faithfully a ByeTex Typst conversion **renders** compared to the original
LaTeX, by looking at page images. This is different from compilation (does it parse?) and from
conversion warnings (unsupported commands). You are the only thing that can see typography and
layout — the structural metrics (word/heading/float recall, SSIM) are blind to title size,
abstract style, citation format, fonts, margins, and float placement.

## Inputs: the grading packet

Build the packet with **`byetex review <paper>`** — one command that renders the converted Typst
to per-page PNGs and, when a truth PDF is available, rasterises the original LaTeX render
alongside. For corpus-wide grading with structural metrics, `scripts/visual_test.py` writes the
same shape into `tests/visual/<id>/`. The `grading_packet.json` contains:

- `detected_class` — the document class (drives nearly all front-matter typography).
- `truth_source` — `provided` | `cached` | `tectonic` | `none`. If `none`, there is no reference
  render: grade the typst pages against the LaTeX **source** instead, and mark truth-relative
  dimensions you couldn't verify as such (don't guess). Pass `--truth <pdf>` to supply one.
- `front_matter.{truth,typst}` — page-1 images (a 200-DPI top crop when from `visual_test.py`).
- `pages[]` — `{page, truth, typst}` per-page raster pairs (`truth`/`typst` may be null if one
  side has fewer pages, or `truth` null throughout when `truth_source` is `none`).
- `warnings` — the conversion-warning summary (`total` + `by_kind`).
- `structure` — inlined metrics (page_ratio, figure/table_ratio, word/heading recall), present
  only from `scripts/visual_test.py`. Use as a cross-check when present.
- `rubric` — points at `docs/fidelity-rubric.md`, the dimension list + severity anchors.

Image paths are **absolute** when the packet comes from `byetex review`, packet-relative when from
`visual_test.py` — either way, open them directly.

**Read `docs/fidelity-rubric.md` first** — it is the authoritative list of dimensions, what
"faithful" means for each, how to spot a gap, the current ByeTex status (HANDLED/PARTIAL/GAP),
and the severity anchors. Grade against it.

## Procedure

1. **Read the rubric** (`docs/fidelity-rubric.md`).
2. **Front matter first** — open `front_matter.truth` and `front_matter.typst` side by side. This
   is where most class fidelity lives (title size/weight/font/rules, author block, abstract
   style/placement, keywords). Grade §1–§2 of the rubric here.
3. **Page sweep** — walk `pages[]`. Read the first ~3–4 pairs in full; for longer papers, sample
   the rest (skim for floats, headings, tables, math). Grade §3–§6 (sectioning/lists/theorems,
   floats, math, page geometry).
4. **Cross-check the metrics when present.** If the packet has a `structure` block (from
   `visual_test.py`), use `page_ratio` for density (§6) and `figure_ratio`/`table_ratio` for
   dropped/extra floats (§4); otherwise judge those visually. Always use `warnings` to inform
   `suspected_cause`. Spend your visual attention on what metrics CANNOT see (typography).
5. **Per dimension**, assign a `verdict` and `severity` using the rubric's anchor. A dimension not
   present in this paper is `na` (e.g. no theorems) — that is valid information, not a miss.
6. **Emit ONLY the findings JSON** (schema below). Record every dimension you could assess; you
   may omit pure `match` rows to keep it short, but ALWAYS include every `minor`/`major`. Be
   concrete in `truth_desc`/`typst_desc` (what you actually see), and point `evidence_image` at
   the packet-relative image that best shows it.

## Output schema (`findings.json`)

```json
{
  "paper": "arxiv:2605.22507",
  "detected_class": "neurips",
  "grader_run": "1",
  "findings": [
    {
      "dimension": "front-matter/title-size",
      "verdict": "minor",
      "severity": 3,
      "truth_desc": "17pt bold centered title under a 4pt rule",
      "typst_desc": "title ~14pt; no bottom rule below it",
      "suspected_cause": "title_rule_below not drawn / title_size mismatch",
      "evidence_image": "pages/frontmatter-typst.png"
    }
  ],
  "summary": { "match": 8, "minor": 3, "major": 1, "na": 2 }
}
```

- `dimension` — use the rubric's `section/slug` form (e.g. `front-matter/abstract-placement`,
  `floats/figure-placement`, `sectioning/heading-size-hierarchy`).
- `verdict` ∈ `match` | `minor` | `major` | `na`. `severity` 1–5 from the rubric anchor (omit/0 for `match`).
- `suspected_cause` — your best guess (a rubric "GAP" status is a strong hint it's an
  unparameterized hole, not a regression). It feeds `docs/fidelity-backlog.md`.

## Calibration

- **Reader impact, not pixel count.** A wrong citation style that misattributes sources outranks
  a 1pt title-size miss. Use the rubric's severity anchors; don't inflate.
- **Cross-engine renders never match perfectly.** Minor sub-pixel/justification/hyphenation
  differences from LaTeX↔Typst are `match`, not findings — only flag what a human reader would
  notice as *wrong for this class*.
- **Attribute to the class.** "The abstract is full-width" is only a gap if `detected_class` is a
  two-column class whose abstract belongs in column 1.
- **Be honest about uncertainty.** If a page is illegible or a dimension can't be assessed, say so
  in `typst_desc` rather than guessing a verdict.
