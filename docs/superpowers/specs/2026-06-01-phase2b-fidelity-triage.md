# Phase 2b — structural fidelity triage (ranked defect list)

**Date:** 2026-06-01. Drives Phase 2c (fix in TDD slices). Built from the full 26-paper
baseline (`scripts/visual_test.py --truth-source tectonic --no-truth-download`) plus
source-level root-causing.

## Baseline coverage

26 papers swept. Status tally:
- **9 `ok`** (typst compiled + structure measured, passed thresholds)
- **7 `structure_failed`** (compiled, but a structure metric tripped a threshold)
- **10 `truth_render_failed`** — **tectonic cannot compile the source LaTeX** (driver
  conflicts, undefined control sequences, missing pkgs). These are a **truth-source
  limitation, not ByeTex defects**; they can only be scored against the arXiv canonical
  PDF (drop `--no-truth-download`, needs network). Corpus fidelity score over the
  16 measured = **0.699** (0.4·word_recall + 0.3·heading_recall + 0.3·ssim).

So the actionable signal is the **16 measured papers** (full per-paper metrics in
`.fidelity_all/<id>/structure.json`).

## CRITICAL triage caveat — the heading metric is unreliable on math-heavy papers

The lowest column is `heading_recall`/`heading_sequence_score` (22579 0.12, 22728 0.22,
22584 0.30, …). **Investigated: this is mostly a MEASUREMENT artifact, not a ByeTex
defect.** Reading the actual heading lists from `structure.json`:
- **22584** (hrec 0.30): ByeTex's emitted headings are *excellent and correctly ordered*
  (`introduction, setting, discretization…, conclusions, bibliography`). The **truth**
  extraction is the polluted one — `pdftotext` + the heading regex pulled equation
  fragments (`p,q,r,s=1 pq p r s q`), author names, and mangled lines *as headings*. Low
  recall because byetex's clean headings don't substring-match the truth noise.
- **22579 / 22728**: BOTH truth and typst heading lists are equation-soup — the heading
  regex misfires on dense-math papers, on both sides.
- **22765** (hrec 0.55): byetex headings look complete and correct; truth is again
  polluted with equation fragments.

**Implication:** do NOT send Phase 2c chasing "lost headings" based on this metric. The
metric's `extract_pdf_headings` heuristic has a high false-negative rate on math papers.
**This is itself a defect — but a METRIC defect (Phase 2a tooling), not a converter
defect.** Fixing the heading extractor (or replacing it with a structure-aware source-side
heading list) is a prerequisite to trusting heading fidelity numbers.

## Ranked defect list

### D1 — Tables emitted as "Figure", and `\input`-ed tabulars dropped (CONFIRMED, high impact)
**Evidence:** 22776 table_ratio 0.12 (8 source tables → 1 emitted), 22817 table_ratio 0.22
(9 → 2). Root-caused at source level:
1. **No `kind: table`.** byetex wraps every float as a bare `#figure(...)`; Typst defaults
   `kind: image` → caption "Figure N". So even tables byetex *does* emit are labelled
   "Figure", not "Table" — directly explains the low table_ratio AND inflates figure_ratio
   (tables counted as figures). Fix: set `kind: table` on the `#figure` when the body is a
   `table(...)`. Emitter site: `emit.rs` float handler ~6770–6784.
2. **`\input`-ed tabulars dropped.** 7 of 8 tables in 22776 use
   `\begin{table}…{\input{results_c_index}}…\end{table}` — the tabular lives in a separate
   file. The float emitter (`emit.rs:6751–6766`) scans AST children for a `tabular`
   `generic_environment` and finds none (the `\input` is an unexpanded `latex_include`
   node at that point) → "figure has no \includegraphics or tabular body" warning → table
   dropped entirely. Fix: resolve `\input` inside a float body before the tabular scan
   (or scan into `latex_include` children). Warning site: `emit.rs:6786`.

D1 is the **highest-value, best-understood** defect: two concrete emitter fixes, both
testable with small fixtures, both directly move table_ratio/figure_ratio on real papers.

### D2 — Figure over-count (PARTIALLY confirmed, mixed cause)
**Evidence:** figure_ratio > 1 on many papers — 22728 (2.40), 22817 (2.17), 22776 (1.88),
22765 (1.70), 22549 (1.50), 22779 (1.25). Causes are mixed:
- Part of it is **D1 spillover** (tables mislabelled as figures inflate the figure count) —
  fixing D1 will reduce figure_ratio on table-heavy papers (22776, 22817).
- Part is **truth-side under-count**: 22728 has 5 figure envs / 5 includegraphics / 0
  tables, yet typst=12 figures vs truth=5. The truth `pdftotext` "Figure N" caption count
  is suspect (same extraction noise as headings). Needs the truth-side caption extraction
  validated before trusting the ratio.
Action: re-measure D2 AFTER D1 lands and after the caption-extraction reliability check
(see D4). Likely shrinks substantially.

### D3 — Content-volume drift (LOW, watch) 
**Evidence:** word_count_ratio is healthy on most (0.90–1.14), with one outlier: **22814
wcnt 1.29** (29% more text than truth). 22814 was a Phase-1 bug paper (the section-label
fix). Worth a look for residual duplication/leak, but low priority — single paper, modest.

### D4 — Metric reliability: heading + caption extraction noise (CONFIRMED metric defect)
**Evidence:** the heading-list pollution above; likely the same for figure/table caption
counting on math-heavy papers. This is Phase-2a tooling debt that *blocks trusting* D2 and
any heading-fidelity number. Fix options: (a) make `extract_pdf_headings` reject
equation-fragment lines more aggressively; (b) better — derive the *truth* heading/float
list from the **source LaTeX** (`\section`/`\caption`/`\begin{table|figure}`) rather than
from `pdftotext` of the rendered PDF, since we have the source. (b) makes the metric
source-anchored and far less noisy.

## Recommended Phase 2c sequence

1. **D1 first** (two emitter fixes: `kind: table` + `\input`-in-float). Highest confidence,
   clear fixtures, moves real numbers. One PR (or two slices).
2. **D4 next** (source-anchored truth heading/float extraction) — unblocks trusting the
   rest of the metric and removes the false heading signal.
3. **Re-measure, then D2** (figure over-count) on the cleaned metric.
4. **D3** opportunistically.

Each Phase-2c PR: TDD fixture + re-run `visual_test.py`, must hold/improve the committed
fidelity number AND keep compile-rate 25/25 (the gate).

## Note on the 10 truth_render_failed papers
Not ByeTex defects. To expand fidelity coverage to them, run with the arXiv canonical PDF
as truth (`--truth-source arxiv` / drop `--no-truth-download`) — a separate measurement
task, not a converter fix.
