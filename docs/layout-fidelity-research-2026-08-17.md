# Layout-Drift Detection: Harness Audit + Tooling Research

**Date:** 2026-08-17 · **Question:** the fidelity harness misses obvious layout drift. What is the
third category of tooling — beyond hand-written structural rules and vision LLMs — and does it fit?

**Method:** five parallel investigations. Everything marked **[M]** was measured on this machine
against real corpus artifacts (typst 0.14.2, tectonic 0.16.9, Apple Silicon). Nothing here is a
literature summary alone.

---

## Part 1 — The current harness, and exactly how it fails

### 1.1 What it is

```
truth:  paper LaTeX --tectonic--> truth.pdf  ─┐
                                              ├─> pdftotext -layout ─> SET of [A-Za-z]{3,}
output: .tex --byetex--> .typ --typst--> pdf ─┘   pdftoppm 100dpi   ─> greyscale SSIM

fidelity_score = 0.35·word_recall + 0.25·heading_recall + 0.20·mean_ssim + 0.20·page_closeness
```

Headings and float counts are **not** read from either PDF — they are regexed from the LaTeX source
and from ByeTex's own `.typ` (`visual_test.py:419-425`). The truth PDF contributes only word tokens,
a page count, and SSIM pixels.

### 1.2 No geometry is compared anywhere **[M]**

Zero hits for `pymupdf` / `fitz` / `pdfplumber` / `pdfminer` / `pdftotext -bbox` / `pdftohtml`
across `scripts/`, `crates/`, `docs/`, `.github/`. `pdftotext -layout` is the only text path
(`visual_test.py:394`) and its spatial output is destroyed one function later by
`re.findall(r"[A-Za-z]{3,}")` into a `set` (`:405-416`).

Consequences of that one line: **every digit in the document is discarded** — table data, equation
numbers, dates, citation numbers. And because it is a `set`, reading order, duplication, and
line structure are invisible.

### 1.3 Three of four channels are dead **[M]**

Across the 65 measured papers in `fidelity_baseline.json`:

| channel | weight | min | med | max | σ |
|---|---|---|---|---|---|
| `word_recall` | 0.35 | 0.673 | 0.886 | 0.986 | 0.071 |
| `heading_recall` | 0.25 | 0.375 | **1.000** | 1.000 | 0.104 |
| `mean_ssim` | 0.20 | 0.394 | 0.562 | 0.803 | 0.089 |
| `page_ratio` | 0.20 | 0.312 | 1.000 | 1.333 | 0.178 |

`heading_recall` is exactly 1.0 for **43/65** papers — a quarter of the score is a constant.

### 1.4 SSIM is anti-correlated with severity **[M]**

Controlled experiment: identical text, only layout varied, scored with a verbatim reimplementation
of `page_image_similarity` (`visual_test.py:935`).

| variant | severity | word_recall | mean_ssim | composite |
|---|---|---|---|---|
| ragged-right vs justified | legit | 1.000 | 1.000 | 1.000 |
| **font family swap** | **legit** | 1.000 | **0.543** | 0.909 |
| 10 → 10.5pt | legit | 1.000 | 0.583 | 0.917 |
| text block shifted 1cm | **real** | 1.000 | 0.743 | 0.949 |
| **margins halved** | **real** | 1.000 | 0.561 | 0.912 |
| **1-col → 2-col collapse** | **real** | 1.000 | 0.602 | 0.920 |

Two results, both worse than "insensitive":

1. **Every variant scores above the 0.827 corpus baseline.** A halved margin and a collapsed column
   layout both pass the fidelity gate comfortably.
2. **The ordering is inverted.** A catastrophic column collapse (0.602) scores *better* than a benign
   font swap (0.543). At weight 0.20, SSIM is worse than noise — it rewards the wrong thing.

Confirmed at corpus scale over all 64 truth/output pairs on disk:

```
pearson(truth ink coverage, mean_ssim)     = -0.913
pearson(|Δ text width|/truth, mean_ssim)   = +0.628
pearson(|Δ leading|/truth,    mean_ssim)   = +0.441
```

r = −0.913 means **SSIM is very nearly a pure function of how blank the truth page is.** Concretely:
`gh-bard-metropolis` (wrong page size, 2× font, +122% text width) scores **0.803**; `2605.22779`
(correct page size, correct 10pt body, text width within +3%) scores **0.394, worst in the corpus** —
solely because it is dense.

Mechanical causes, both in `visual_test.py:958-967`: both pages are **resized to a common size**
before comparison (normalizing away exactly the page-size mismatch it should catch), and pages are
paired index-to-index, so one displaced float decorrelates every downstream page.

### 1.5 Drift prevalence on the real corpus **[M]**

PyMuPDF geometry scan over all 64 pairs. **Only 4/64 papers have `status != ok`** — everything below
is inside the passing set.

| drift class | prevalence |
|---|---|
| leading wrong >8% | **58/64 (91%)** |
| top margin wrong >20% | 36/64 (56%) |
| text-measure width wrong >5% | 32/64 (50%) — 23/64 >15% |
| body font size wrong | 24/64 (38%) |
| ink density wrong >20% | 20/64 (31%) |
| column count wrong (both directions) | 7/64 (11%) |
| page trim / aspect ratio wrong | 7/64 (11%) |

Worst single case: **`2605.22315`** — text column +43%, left margin moves 63pt, **both
`\includegraphics` figures dropped** (0 `image()` calls in the `.typ`) — scores `word_recall` 0.923,
`structure_ok: true`, **passes**.

Font-size tier histogram on the same paper (share of characters) **[M]**:

| tier | truth | output |
|---|---|---|
| **8.5pt** | **17.8%** | **1.2%** |
| 10.0pt | 77.5% | 94.2% |
| headings | 12.0 / 15.9 / 20.0 | 14.4 / 16.8 / — |

LaTeX puts 17.8% of all characters at 8.5pt (abstract, captions, footnotes, bibliography); ByeTex
emits 1.2%. **The `\small`/`\footnotesize` tier is essentially missing.** The heading ladder is wrong
in both directions. This is a live converter bug found by the research, not by the harness.

### 1.6 Harness defects independent of what it measures

- **Four computed metrics are unreachable by the gate.** `word_count_ratio`,
  `heading_sequence_score`, `figure_ratio`, `table_ratio` are computed (`visual_test.py:922-925`) and
  written to `index.json`, but are absent from `BASELINE_FIELDS` (`fidelity_check.py:35-42`), so
  `evaluate()` can never compare them. `word_count_ratio` = 1.140 on `2605.31306` (a 654-word
  duplicated block) and nothing reads it.
- **Neither hard gate runs in CI.** `fidelity_gate.sh` is `workflow_dispatch`-only
  (`.github/workflows/ci.yml:90-116`); `acceptance.sh` appears in no workflow at all.
- **The baseline is stale** — 21/64 recorded `page_ratio` values no longer match the artifacts on disk.
- **~half of `word_recall`'s deficit is a `pdftotext` ligature artifact**, not content loss. Truth
  PDFs use CM/Times with `fi`/`ff`/`fl` ligatures that split on extraction; the missing tokens are
  debris — `xed`(fixed), `nite`(finite), `rst`(first), `coe`, `payo`. Artifact-adjusted recall for
  `2605.22315` is 0.978 vs the recorded 0.912. **The real signal is ~6-8 points wide and the gate
  tolerance is 0.05.**
- **6 of 71 baseline papers have no truth render at all** and can never register a regression.

---

## Part 2 — The third category

### 2.1 The taxonomy that decides everything

Any comparison tool sits in one of three classes:

| class | assumes | verdict here |
|---|---|---|
| **A. Byte/object-identical** — qpdf `--json`, pikepdf, sha256 | same producer | **Dead.** pdfTeX and typst share zero object structure. |
| **B. Pixel-registered** — diff-pdf, ImageMagick, odiff, pixelmatch, SSIM, callas, Acrobat, AHRTS | same page count, same line breaks, ±few px | **Dead as a metric.** One extra word on page 1 shifts everything downstream. Useful only for *self*-regression (byetex N vs N+1), where alignment is guaranteed. |
| **C. Content-anchored** — extract with geometry, align on text, compare attributes of aligned tokens | only that the words mostly survive | **The only class that works.** |

**The category the question asks for barely exists as a product.** Every off-the-shelf PDF diff tool
is class B. The one genuine commercial entry is i-net PDFC ("deep structure comparison including
styles, shapes", REST API, closed/Java). The good news: class C is ~200 lines to build.

### 2.2 The three viable oracles

| | coverage | needs | gives | cost |
|---|---|---|---|---|
| **PDF geometry extraction** — `pdftotext -tsv`, PyMuPDF, pdfplumber | **any PDF pair, 100%** | nothing new | bbox, font name/size/flags, drawn rules, page size | trivial |
| **Engine-native introspection** — SyncTeX ↔ `typst query` | ~60% (needs local tectonic recompile) | nothing new | exact element positions from both engines | medium |
| **ML layout oracle** — Docling on both sides | 100% | `pip install docling` | semantic region classes, tables, doc tree | medium |

#### PDF geometry — the universal floor

`pdftotext -tsv` (poppler 26.04.0, **already installed and already shelled out to**) gives
flow→block→line→word with bbox. PyMuPDF `get_text("dict")` adds font name, size, and bold/italic/serif
flags. pdfplumber (MIT) is the only tool exposing `.rects`/`.lines` — i.e. **drawn rules**, so booktabs
`\toprule`/`\midrule` fidelity becomes countable.

Alignment strategy, **[M]** and fast: normalize (NFKC, strip soft hyphens, map ligatures) →
`difflib.SequenceMatcher(autojunk=False)` on word streams → compare geometry of *aligned* pairs only.
**0.2–1.0s per paper, 70–80% alignment coverage** on 14k-word papers.

Median aligned line-height ratio out/truth is **1.11–1.14 with p10 ≈ p90** — a tight, systematic
leading inflation. `2605.22549` is an outlier at **1.407**.

Critically, **[M]** established what *not* to measure: absolute per-token |Δx|/|Δy| has median
28-104pt and p90 237-285pt — pure noise, because one reflow shifts everything downstream.

#### Engine-native introspection — the exact oracle

**Typst side works today, non-invasively [M].** A wrapper file beside `main.typ`:

```typst
#include "main.typ"
#context [#metadata(query(heading).map(h => (
  l: h.level, p: h.location().page(),
  x: h.location().position().x.pt(), y: h.location().position().y.pt())))<btdump>]
```
```bash
typst query probe.typ '<btdump>' --field value --root .
```

- **Byte-identical PDF size** with and without the probe — genuinely zero layout cost.
- 33-page corpus paper → 36 headings + 23 figures + 13 block equations in **0.7s / 3.8KB JSON**.
- **Corpus coverage: 69/71 = 97%.** (Typst HTML export, by contrast, is 35/71 = 49%.)
- Locatable: `heading`, `figure`, `table`, `math.equation`, `par`, `list`, `image`, `footnote`, `ref`.
  **Not locatable:** `text`, `block`, `table.cell`, `linebreak` — so no line- or glyph-level access.
- **`metadata` *is* locatable**, and `#metadata("c11")<a1>` inside individual table cells returns
  exact distinct `{page,x,y}`. Since ByeTex generates the Typst, it can emit source-keyed anchors at
  zero layout cost behind a flag.

**LaTeX side: SyncTeX is a complete box dump, no source modification [M].** `tectonic --synctex`
works out of the box. On a 19-page ACL paper: **41,114 records, ~136 boxes/page**. Records are
`(tag,line:x,y:w,h,d` in scaled points, **y top-origin — same convention as Typst, no axis flip**.
Parses in ~25 lines of pure Python (`gzip` + `re`); no C library, and PyPI has nothing usable.

Page-1 line hboxes come out as `(x=71.1, y=46.1, w=455.2)`, and the distinct x-origins
`{66.8, 71.1, …, 307.3, 334.6}` hand you **the two column origins (71.1pt / 307.3pt) directly**.

Limits, all **[M]**: SyncTeX carries **no text** — you join on `(file, line)` + geometry only. Line
attribution is off by up to one source line (TeX breaks a paragraph only after reading the following
line). Under tectonic many `Input:` entries have empty filenames (116/126 on one paper, ~11% of boxes
unattributable). And it needs a **local recompile**, which the pipeline currently avoids by preferring
downloaded arXiv PDFs — the binding coverage constraint.

**The join is already solved.** ByeTex emits `<key>` in Typst for every `\label{key}` in LaTeX. **[M]**
on `2605.22820`: 50 LaTeX labels ↔ 51 Typst anchors. Exact string join, no fuzzy matching, no
annotation. This is an alignment advantage no published benchmark has.

**Dead ends, all verified:** tagged-PDF symmetry (truth PDFs have `StructTreeRoot: 0`; tectonic's
kernel is frozen at LaTeX2e 2021-11-15, `\DocumentMetadata` undefined, tagpdf doesn't support XeTeX);
XDV (glyph IDs, not Unicode); LuaTeX node lists (wrong engine — tectonic is XeTeX).

#### ML layout oracle — symmetric, so detector noise mostly cancels

Docling (MIT, MPS, ~3-15s/paper after warmup; full 71×2 sweep ≈12 min) run on **both** PDFs. **[M]**
per-class deltas found real bugs:

| paper | class | truth | output |
|---|---|---|---|
| 2605.22776 | **table** | **8** | **1** |
| 2605.22820 | **list_item** | **33** | **5** |
| 2605.22507 | code | 0 | 5 |

Honest caveat: cancellation is only *partial*. `2605.22557` reports 4 section headers in truth vs 18
in output — the truth side is almost certainly under-detected because the LaTeX heading font differs
from what the model expects. **Treat class-count deltas as triage leads, never as a scored gate.**

### 2.3 Metric science worth importing

| metric | verdict |
|---|---|
| **`ordered_recall`** — LCS over the linearized word stream (stdlib `difflib`) | **Adopt.** **[M]** spans 0.593–0.815 (range 0.222) where set-recall spans 0.779–0.847 (range 0.068) — **3.3× the dynamic range**, and it *reverses the ranking*: `2605.22557` looks second-best under set-recall and is unambiguously worst under ordered-recall. `ordered_precision` additionally catches duplication/leakage, which recall of any kind cannot see. |
| **`anchor_recall`** — `\label{}` keys vs `<anchor>`s | **Adopt.** Pure text, zero render, zero deps. **[M]** across 67 papers: mean 0.891, **24 below 0.9**, `gh-pelegs-maths-book` 373 labels → **0** anchors, `2605.31203` 20 labels → 83 anchors with **1** match (key *mangling*, not loss). |
| **`anchor_drift`** — SyncTeX ↔ `typst query`, joined on the label string | **Adopt.** **[M]** end-to-end: `2605.22776` mean progress-drift **0.026**; `2605.22820` **0.133**. **5× separation** between a good and a bad conversion, in seconds, no ML, no pixels. |
| **TEDS / TEDS-Struct** (`pip install table-recognition-metric`, Apache-2.0) | **Adopt as a probe.** **[M]** installs and runs in seconds; correct sensitivity (text typo 0.857 / struct 1.000; spurious column 0.778 / 0.778). Blocked on getting HTML tables from both sides — Typst HTML export is only 49%. Per-table diagnostic, not a corpus gate. |
| **Reading-order metrics** (ARD, Kendall τ, OmniDocBench RO edit distance) | **Do not build.** **[M]** τ = 0.93–0.999 across papers. ByeTex has no block-level reading-order problem. (The *content*-stream divergence — bibliography reordering — is real, but `ordered_recall` captures it and τ does not.) |
| **Layout mAP (COCO)** | **Semantically wrong.** mAP presumes one privileged annotated side. Two legitimate renderings *should* place boxes differently. Use IoU/Hungarian region-set matching instead. |
| **BLEU / METEOR** | Skip — adds nothing over LCS, heavy deps. |
| **CDM** (arXiv:2409.03643) | Not as shipped (needs Node + ImageMagick + Ghostscript + TeX Live, and it compares two *LaTeX* strings). But its *technique* — render both and match characters spatially — is the only viable route to a **cross-engine formula fidelity** metric. Research spike, park it. |
| **LPIPS / DreamSim / CW-SSIM** | Skip. No pixel metric separates "legitimate reflow" from "content loss" — at the pixel level they are the same event. |

### 2.4 Prior art: how layout engines actually test layout

Nobody does cross-engine layout comparison except the cross-browser-incompatibility literature. But
the tolerance-design lessons are decisive:

- **l3build has exactly one numeric tolerance in the entire system** (glue set rounded to 4dp).
  Everything else is a named rewrite rule — `on line <n>` → `on line ...`, units standardised,
  discretionaries folded. **Normalise, don't fuzz.**
- **Gecko's fuzzy ranges are two-sided**: `fuzzy-if(cocoaWidget,1-1,8-8)` fails if the difference is
  *too small* as well as too large — so an improvement forces a visible rebaseline instead of silent
  slack.
- **SILE** compares a golden box-model dump **byte-exact** (`cmp -s`), with rounding to 4dp as the
  entire tolerance model, and handles platform variation by *exclusion* (`KNOWNBAD`, `OS=`) — plus it
  reports "known bad tests that pass".
- **X-PERT / ReDeCheck** are the only validated cross-engine work. They discard absolute coordinates
  entirely in favour of **relational predicates** (left-of, above, overlapping, center-justified).
  `heading is at x=71.1pt` needs a tolerance you cannot pick; `heading starts at the left column
  origin` needs none.
- **Counterweight:** Chromium ranks layout-tree text dumps as its *least*-preferred test type — not
  for accuracy but for baseline churn and semantic over-specification. LibreOffice's answer is "dump
  everything, assert almost nothing" via targeted XPath. **A dump-based design must project down to a
  small set of assertions you actually care about.**
- **Typst's own suite** is intra-engine, exact (per-channel byte delta ≤1, dimensions must match
  exactly), 72dpi, refs capped at 20KiB. Not transferable — it fails on line 1 when page sizes differ.
- **There is essentially no rigorous layout-fidelity validation practice in LaTeX→other-format
  conversion.** LaTeXML/tex4ht comparison studies judge layout by eye. BabelDOC's state-of-the-art
  layout-IoU is **50.0%** — which tells you IoU is a ranking signal, never a gate.

---

## Part 3 — Does it fit the hole?

**Yes, and the fit is unusually clean, for one reason: ByeTex's comparison is *symmetric*.**

Nearly every metric in this literature was built for "model output vs hand-annotated ground truth".
ByeTex has something strictly better — both sides are machine-readable renderings, the "annotation" is
*computed* from the truth PDF per run, detector noise applies to both sides and largely cancels, and
the correspondence is authored by ByeTex itself (`\label` → `<anchor>`). That is why an imperfect
oracle like Docling is usable here in a way the benchmark literature would not suggest.

### 3.1 Coverage of the measured hole

| drift class (prevalence) | caught by | alignment needed? |
|---|---|---|
| leading wrong (91%) | aligned line-height ratio; page-level leading median | ✅ / ❌ |
| top margin wrong (56%) | page-level text-bbox | ❌ |
| text width wrong (50%) | page-level text-bbox | ❌ |
| body font size wrong (38%) | **font-size tier histogram** | ❌ |
| ink density wrong (31%) | page-level coverage | ❌ |
| column count wrong (11%) | x-projection gap scan; SyncTeX column origins | ❌ |
| page trim wrong (11%) | page `MediaBox` | ❌ |
| figures dropped | `image()` count vs truth raster count | ❌ |
| content duplicated | `ordered_precision` | ✅ |
| bibliography reordered | `ordered_recall` | ✅ |
| labels lost/mangled | `anchor_recall` | ❌ (no render) |
| element on wrong page | `anchor_drift` | ❌ (exact join) |
| table structure wrong | TEDS | probe only |

**The largest-prevalence drifts need no alignment at all** — they are page-level aggregates computable
independently on each side. That is the 80/20.

### 3.2 Proof the geometric profile discriminates **[M]**

~60 lines of PyMuPDF span extraction, run on the controlled variants from §1.4:

| variant | kind | left_Δ | right_Δ | ncol_mismatch | fsize_Δ% | lead_Δ% | lines/pg_Δ% |
|---|---|---|---|---|---|---|---|
| font family swap | legit | 0.0 | 0.02 | 0 | 0.0 | 4.6 | 10.3 |
| ragged-right | legit | 0.0 | 0.0 | 0 | 0.0 | 0.0 | 0.0 |
| 10→10.5pt | legit | 0.0 | 0.09 | 0 | 5.0 | 4.6 | 5.1 |
| shifted 1cm | real | **4.76** | **4.76** | 0 | 0.0 | 0.0 | 0.0 |
| margins halved | real | **6.19** | **4.84** | 0 | 0.0 | 0.0 | 12.8 |
| 1→2 column | real | 0.0 | 0.51 | **1** | 0.0 | 11.5 | **97.4** |
| 10→12pt | real | 0.0 | 0.84 | 0 | **20.0** | **19.8** | 7.7 |
| loose leading | real | 0.0 | 0.0 | 0 | 0.0 | **57.3** | 10.3 |

Clean separation, **zero false positives on the legitimate set** — the discrimination SSIM inverts.
(Known rough edge: `bot_Δ` is noisy from ragged last pages; measure only on full pages.)

### 3.3 What does *not* fit

- **Pixel diff tools** (diff-pdf, ImageMagick, odiff, callas, AHRTS) — structurally incapable. Keep
  raster+SSIM only for byetex-vs-byetex self-regression, where alignment *is* guaranteed.
- **Tagged-PDF logical structure comparison** — impossible; the truth side cannot be tagged without
  editing all 59 corpus papers.
- **Reading-order metrics** — measured non-problem.
- **Layout mAP** — wrong semantics for a symmetric comparison.
- **A single scalar "layout score"** — every surviving system (l3build, SILE, Gecko, LibreOffice)
  reports *named property failures*; every system that reports a scalar (BIoU, DocSim) uses it for
  ranking, never for gating.

---

## Part 4 — Recommended build

Ordered by value ÷ effort. Tiers 0-2 need no alignment and no new binary dependency.

**Tier 0 — `anchor_recall` (~1 hour, no render, no deps).** `\label{}` keys vs `<anchor>`s in the
emitted `.typ`. Already found 24/67 papers below 0.9 and two catastrophic bugs. Scope the `.tex` file
set to what was actually compiled (SyncTeX's `Input:` list gives this free) — globbing `*.tex` picks up
unused variants and depresses the score.

**Tier 1 — geometric page profile (~1 day, PyMuPDF via `uv run --with`).** Per page, both sides,
independently: page trim, text-block bbox → margins, column count via x-projection gap scan,
font-size tier histogram, median leading, lines/page, ink coverage. Compare as **ratios and
distributions**, never absolute deltas. This alone covers the six highest-prevalence drift classes.
Emit the per-tier table into the vision agent's packet so its findings become quantitative.

**Tier 2 — `ordered_recall` / `ordered_precision` (~2 hours).** Replace set-based `word_recall`.
3.3× the dynamic range and it re-ranks papers. **Fix the ligature artifact first** (normalize
`fi`/`ff`/`fl` and dehyphenate before tokenizing) or half the signal stays noise.

**Tier 3 — `anchor_drift` (~1 day).** `tectonic --synctex` → parse → `typst query` probe → join on
label → mean/p95 progress drift. 5× separation measured. Additional signal only — SyncTeX coverage is
~60% and the failing papers are plausibly the hard ones.

**Diagnostics, not gates.** Docling per-class deltas; TEDS on the ~49% of papers where Typst HTML
export succeeds; `pdffonts` family inventory.

**Fixes to the existing harness, independent of any of the above:**

1. Remove `mean_ssim` from `FIDELITY_WEIGHTS` — it is anti-correlated with severity. Keep it as a
   per-paper tripwire only.
2. Drop or re-weight `heading_recall` — saturated at 1.0 for 43/65.
3. Add `word_count_ratio`, `figure_ratio`, `table_ratio`, `heading_sequence_score` to
   `BASELINE_FIELDS` so the four already-computed metrics can actually gate.
4. Regenerate the stale baseline (21/64 `page_ratio` values are wrong).
5. Wire `acceptance.sh` and `fidelity_gate.sh` into CI, or document explicitly that they are
   release-time-only.

**Tolerance policy — adopt in this order:**

1. **Normalise away every legitimate difference you can name** (l3build): ligatures, soft hyphens,
   hyphenation, page-number-only text, arXiv side-stamp. A rewrite rule, not an epsilon.
2. **Prefer relational to metric** (ReDeCheck): "starts at the left column origin", "caption below
   figure", "element *i* is not on an earlier page than *i−1*" need no tolerance at all.
3. **Two-sided ranges** (Gecko) so improvements force a visible rebaseline.
4. **Per-property categorical exclusion** (SILE `KNOWNBAD`) rather than a global threshold wide
   enough to cover the worst paper.

Resist a single scalar. Report named property failures that map to an emitter.

---

## Appendix — incidental findings

- **`--no-pdf-tags` may be obsolete.** `typst compile` *without* it succeeded on all four papers
  tried under typst 0.14.2. Worth re-testing corpus-wide (`corpus_sweep.sh:136`,
  `visual_test.py:262`, `dogfood.py:206`).
- **Live converter bug:** the `\small`/`\footnotesize` tier is dropped (17.8% → 1.2% of characters on
  `2605.22315`), and the heading size ladder is wrong in both directions.
- **`typst eval`** (PR #7362, merged 2025-11-17) will eventually make the `#include` probe wrapper
  unnecessary; it is not in 0.14.2.
- **PyPI naming trap:** `pip install teds` is an unrelated schema tool. The TEDS metric is
  `table-recognition-metric`.
- **Licence caution:** PyMuPDF is AGPL-3.0 (fine as a dev-only `uv run` script, never linked into the
  shipped binary); pdfplumber is the MIT fallback; DocLayout-YOLO is AGPL; Surya's licence needs a
  real check (PyPI says Apache-2.0, the repo has historically been GPL-3.0-with-exception).
- **Scratch artifacts** from this research: `tmp/layoutresearch/` (212KB) and `tmp/metric-probe/`.
  Prototype scripts for the §3.2 profile are in the job tmp dir; port before deleting.
