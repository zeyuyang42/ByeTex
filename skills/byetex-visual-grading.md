---
name: byetex-visual-grading
description: Grade the VISUAL FIDELITY of a ByeTex conversion against the LaTeX truth — compare truth↔typst page images dimension-by-dimension and emit structured findings. Use when given a `grading_packet.json` (from `scripts/visual_test.py`) or asked to visually grade/audit how faithfully a `.typ` renders vs the original. NOT for fixing compile errors (use byetex-repair-loop) or warnings (use byetex-using-warnings-json).
---

# byetex: visual fidelity grading

You are grading how faithfully a ByeTex Typst conversion **renders** compared to the original
LaTeX, by looking at page images. This is different from compilation (does it parse?) and from
conversion warnings (unsupported commands). You are the only thing that can see typography and
layout — the structural metrics (word/heading/float recall, SSIM) are blind to title size,
abstract style, citation format, fonts, margins, and float placement.

## Inputs: the grading packet

You are given a `grading_packet.json` (written per paper by `scripts/visual_test.py` into
`tests/visual/<id>/`). It contains, with paths RELATIVE to that dir:

- `detected_class` — the document class (drives nearly all front-matter typography).
- `front_matter.{truth,typst}` — a 200-DPI crop of page-1's top region (title/authors/abstract).
- `pages[]` — `{page, truth, typst}` per-page raster pairs (`truth`/`typst` may be null if one side has fewer pages).
- `structure` — the inlined metrics (page_ratio, figure_ratio, table_ratio, word/heading recall).
- `warnings` — the inlined conversion-warning summary.
- `rubric` — points at `docs/fidelity-rubric.md`, the dimension list + severity anchors.

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
4. **Cross-check, don't re-derive, the metrics.** Use `structure.page_ratio` for density (§6),
   `figure_ratio`/`table_ratio` for dropped/extra floats (§4), and `warnings` to inform
   `suspected_cause`. Spend your visual attention on what the metrics CANNOT see (typography).
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
