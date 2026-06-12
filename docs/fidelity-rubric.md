# ByeTex Fidelity Rubric

The canonical map of **rendering-fidelity dimensions**: what "faithful to the LaTeX truth" means
for each, how to spot a gap in a truth↔typst image pair, where ByeTex stands today, and a severity
anchor for grading. This doc is **both** the oracle for the visual-grading loop
(`skills/byetex-visual-grading.md`) **and** the discovery scaffold for new fidelity work.

It is deliberately separate from *compilation* (does the `.typ` parse?) and from *conversion-gap
warnings* (`warnings.json`: unsupported command/env/macro). Those are tracked elsewhere. Fidelity
is: **the `.typ` compiles, but does it LOOK like the LaTeX original?**

## How to use this as a grader

For each truth↔typst pair, walk every dimension below and assign:
- **verdict** ∈ `match` | `minor` | `major` | `na` (dimension not present in this paper)
- **severity** 1–5, using the per-dimension anchor (a `match` is severity 0/omitted)

Severity is about *reader impact*, not pixel count: a wrong citation style that misleads attribution
outranks a 1pt title-size miss. The **Status** column tells you whether a gap is a known
unparameterized hole (so a finding is actionable) or already handled (so a finding is a regression).

Status legend: **HANDLED** (class-faithful today — a gap here is a regression) · **PARTIAL** (one
behavior for all classes, or basic support; class-specific nuance missing) · **GAP** (unparameterized;
expected to surface in the audit).

---

## 1. Front matter

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| title size | matches the class's `\maketitle` size (article \LARGE≈1.728em, NeurIPS 1.7em, ICML 1.4em, IEEE \Huge 2.4em, LNCS 1.44em) | title visibly larger/smaller than truth, or same size across classes that should differ | HANDLED (`StyleProfile.title_size`, `preamble.rs` flush_title_block) | 2 |
| title weight/smallcaps | article regular, NeurIPS/ICML/LNCS bold, ICLR small-caps | bold where truth is regular (or vice-versa); no small-caps on ICLR | HANDLED (`title_bold`/`title_smallcaps`) | 2 |
| title rules (bars) | NeurIPS 4pt-top/1pt-bottom, ICML 1pt/1pt rules around the title | missing or extra horizontal rule above/below the title | HANDLED (`title_rule_above/below`) | 2 |
| title font family | class body/title font (acmart Libertine→Libertinus; most → Computer Modern) | obviously different letterforms (serif vs the truth's family) | PARTIAL (body_font only; no distinct title family) | 2 |
| author block | author names/affiliation/email layout & grouping matches truth (columns, superscript affil markers, equal-contribution) | authors stacked vs inline; affiliations un-grouped; missing email/ORCID line | PARTIAL (`materialize_authors`; no per-class column/footnote-affiliation geometry) | 3 |
| abstract style | per-class heading + body (article \small+centered; NeurIPS/ICML 1.2em bold; ICLR small-caps; IEEE run-in `*Abstract*—`; LNCS run-in `*Abstract.*`) | grey box / wrong heading weight / body not small where truth is small | HANDLED (`AbstractStyle`, `render_abstract_block`) | 2 |
| abstract placement | two-column classes (ICML/IEEE/acmart) put the abstract IN column 1; others full-width centered | abstract spans full width on a 2-col paper, or vice-versa | HANDLED (`abstract_in_columns`) | 3 |
| keywords / "Index Terms" | rendered in the class's position & style | missing, or wrong position relative to abstract | PARTIAL (emitted; no per-class styling) | 2 |

## 2. Citations & bibliography

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| in-text cite form | textual `Author (Year)` for \citet, parenthetical for \citep, author/year-only forms | `[1]` where truth shows `Author (year)`; prose vs parenthetical swapped | HANDLED (`CiteMode`, `emit_citation`) | 3 |
| cite bracket/separator style | brackets & separators match (e.g. `[1, 2]` vs `(Smith, 2024; Lee, 2023)`) | numeric where truth is author-year, or wrong delimiter | PARTIAL (follows the resolved CSL style) | 3 |
| bibliography list style | numbered vs author-year reference list matching the class | numeric reference list where truth is author-year | HANDLED (`resolve_bib_style` → `#bibliography(style:)`) | 3 |
| bib entry ordering / fields | order (appearance vs alphabetical) and fields (DOI/URL/pages) per the style | references in a different order; missing/extra fields | PARTIAL (whatever the CSL style dictates; no explicit control) | 2 |

## 3. Sectioning, lists, theorems

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| heading numbering | `1`, `1.1`, `1.1.1` (or unnumbered where the class is) | numbered where truth is unnumbered, or wrong depth format | HANDLED (`#set heading(numbering)` + starred detection) | 2 |
| heading size hierarchy | per-class section/subsection sizes & spacing | section headings all one size, or not matching the class's scale/spacing | **PARTIAL → GAP** (one global `#show heading.where(level…)` in `build_neutral_preamble`, NOT per-class) | 3 |
| list markers | itemize bullet style; enumerate numbering/label format; nested-level markers | wrong/zero indent; flat bullets where truth nests; wrong enumerate label | **GAP** (`emit_simple_list` writes a raw `"{marker} {body}"` prefix, not Typst `#list`/`#enum`) | 3 |
| list spacing | item spacing matches truth (tight vs loose) | items far more/less spaced than truth | GAP (no list spacing control) | 2 |
| theorem/definition styling | per-class theorem head (bold/italic), body (italic), numbering | plain text where truth italicizes the body; missing/!=numbering; no per-class color/border | PARTIAL (`emit_theorem_env` renders + numbers; no per-class visual styling) | 3 |
| proof / QED | `Proof.` run-in + end-of-proof QED box | missing QED; proof not set off | PARTIAL (`emit_proof_env` → `*Proof.* body`; no QED symbol) | 2 |
| footnotes | rendered as bottom-of-page footnotes | footnote text inlined or dropped | GAP (no `\footnote` handler in `emit/`; falls through to unsupported-command) | 4 |

## 4. Floats: figures & tables

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| figure presence/count | every truth figure renders | blank/missing figure; image not found | HANDLED for resolvable paths (asset plan); else dropped | 4 |
| figure placement | float lands top/bottom per `[t]/[b]`-style intent (not jammed inline) | figure sits mid-paragraph where truth floats it to page top | **GAP** (`emit_figure` emits `#figure(...)` with no `placement:` arg) | 3 |
| figure sizing | width/scale relative to column matches truth | figure much larger/smaller; overflows column | PARTIAL (relative widths converted; raw-dim edge cases) | 3 |
| figure caption position | below the image (LaTeX default) | caption above; caption detached | PARTIAL | 2 |
| subfigure grid | sub-panels laid out in a grid with `(a)(b)` sub-labels & sub-numbering | panels stacked vertically; missing sub-labels | HANDLED (`#subpar.grid`) | 3 |
| table presence/count | every truth table renders | dropped/duplicated table | HANDLED (D1 fix) | 4 |
| table rules/booktabs | top/mid/bottom rules; no vertical rules where truth uses booktabs | heavy gridlines where truth is clean booktabs, or missing rules | PARTIAL (`emit/tables.rs`) | 3 |
| table cell alignment/padding | column l/c/r alignment + cell padding/row height | columns mis-aligned; cramped/loose cells | PARTIAL (alignment handled; padding not class-driven) | 2 |
| table caption position | above the table (LaTeX convention) | caption below | PARTIAL | 2 |

## 5. Math

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| display vs inline | `$…$` inline, `\[…\]`/equation display centered on its own line | inline math broken onto its own line, or display math inlined | HANDLED | 3 |
| equation numbering | numbered display eqs get `(N)`; starred/`\notag` unnumbered | every eq numbered, or none; wrong format | PARTIAL (`#set math.equation(numbering)` when used) | 2 |
| math fonts | `\mathcal`/`\mathbb`/`\mathfrak`/`\mathbf` render in the right script | calligraphic/blackboard letters render as plain italics | PARTIAL (`emit/math.rs`, `math_symbols.rs`) | 3 |
| operator/sub-superscript spacing | spacing & script positioning match | cramped/loose operators; mis-placed sub/superscripts | PARTIAL | 2 |

## 6. Page geometry & global style

| Dimension | Faithful means | How to spot a gap | Status | Sev anchor |
|---|---|---|---|---|
| column count | 1-col vs 2-col matches the class | single column where truth is two-column | HANDLED (`is_two_column`, `#columns(2)`) | 4 |
| margins / text block | page margins & text width match the class | noticeably wider/narrower text block; different margins | PARTIAL (IEEE special-cased; most → neutral 1in) | 3 |
| page density / count | typst page count ≈ truth (page_ratio≈1) | document runs noticeably longer/shorter (cross-check `structure.page_ratio`) | HANDLED (10pt + indent-only par spacing; 2-col residual) | 3 |
| two-column balance | final-page columns balanced like truth | last page one full + one near-empty column | GAP (no balancing) | 1 |
| hyperlink / cross-ref color | hyperref link/cite/ref color (often blue) matches truth | links/refs black where truth colors them | **GAP** (no link-color show-rule) | 1 |
| text color / colorboxes | `\textcolor`/`\colorbox` render in color | colored text rendered black | PARTIAL (`emit_textcolor`; dropped in math mode) | 2 |
| header/footer, page numbers | running header/footer & page-number format match | missing running header where truth has one | GAP | 1 |

---

## Notes for graders
- **Cross-check the metrics, don't re-derive them.** The grading packet inlines `structure` (page_ratio, figure_ratio, table_ratio, word/heading recall) and `warnings`. Use page_ratio for §6 density, figure/table_ratio for §4 counts, and warnings for `suspected_cause` — then spend your visual attention on §1–§5 typography, which the metrics CANNOT see.
- **Front matter first.** Most class fidelity lives on page 1; read the front-matter crop pair before the page sweep.
- **A `na` is information.** "This paper has no theorems" is a valid verdict and keeps the dimension's denominator honest.
- Each **GAP** row is a candidate future fix; the audit (`docs/fidelity-backlog.md`) ranks which ones actually bite, by frequency × severity, and names the `emit/` site to change.
