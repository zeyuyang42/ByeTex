# ByeTex Fidelity Backlog

Ranked, concrete rendering-fidelity issues discovered by the vision-grading loop
(`skills/byetex-visual-grading.md` + `docs/fidelity-rubric.md`), graded against the
LaTeX truth on a representative paper per class. Ranked by **frequency × peak severity**.
Each row names the suspected `emit/` site and a fix sketch — each is a future TDD fix PR,
re-graded by re-running the loop.

**These are issues the old structural metrics (word/heading/float recall, SSIM) are BLIND to** —
every one is typography/layout/leakage, not word-set content. The loop found them on the first run.

## Audit basis (2026-06-12)

8 papers, one+ per profiled class, arXiv-canonical truth: 2605.22159 (article), 2605.22507 &
2605.22765 (neurips), 2605.22820 (iclr), 2605.31244 (icml), 2605.31526 (ieeetran), 2605.31598
(lncs), 2605.22776 (article). All compile; graded with `byetex-visual-grading`.

Aggregate: **the body (math, equations, numbered bibliography, single/two-column geometry, page
density) is strong** — most `match` rows are there. Fidelity damage concentrates in the
**front-matter author block** and in **dropped vector floats**.

---

## P0 — high frequency, high severity

### 1. Author-block LaTeX leakage / mangling  — 6 of 8 papers, peak sev 5
**Symptom.** Raw LaTeX tokens leak into the rendered author block and authors/affiliations/emails
are dropped or collapsed into one run-in line. Observed: a stray leading `%`, literal `\,` `\}`
`\quad` `\hspace{..}` `&` `\textbf{...}` `\textit{...}`, a `\newcolumntype` p-column spec leaking
above Keywords (22820), only the first author surviving (22776, 31526), all affiliations dropped,
and `ß`→`Gräle` glyph loss (22159). Worst cases: 22765 renders a literal
`1 \textbf{ Umut Simsekli$^3$ \quad ...}` line; 22507 renders `% Pablo … \, … \}`.
**Why it matters.** The most reader-visible defect on page 1, and it varies per paper (22507 is far
more broken than 22765) → the handling is fragile, not uniformly wrong.
**Suspected site.** `emit/preamble.rs::materialize_authors` + `class_map.rs::parse_authors`: the
`\author{...}` raw-bytes capture (emit.rs) keeps comments/macros, and the per-class author parser
fails on multi-author / `\thanks` / `\\`-separated / `\textbf{...\quad...}` blocks, so unparsed
remnants fall through verbatim.
**Fix sketch.** Strip comments and known spacing/format macros (`\,`,`\quad`,`\hspace`,`\\`,`&`,
trailing `\}`) before/within `parse_authors`; split `\thanks{}` into a footnote (see #11); handle
the `\textbf{a \quad b}` grouped-author idiom; preserve non-ASCII (`ß`). Add per-class author
fixtures + snapshot tests. **Highest-value fix; would flip a major on ~6 papers.**

### 2. Dropped vector floats (figures & tables)  — 5 papers, peak sev 5
**Symptom.** TikZ/pgfplots VECTOR figures dropped while raster images survive: 31526 2/13 figs,
22507 4/11, 22765 6/10; 22159 0/1. Framed/tcolorbox-wrapped TABLES dropped: 22765 3/5 (+ appendix
tcolorbox sample boxes gone). Dropped floats desync pagination on later pages.
**Why it matters.** Whole figures/tables missing is a content+layout defect; `figure_ratio`/
`table_ratio` flag the count but not *which* or *why*.
**Suspected site.** TikZ/pgfplots rendering limitation (`emit/` tikz path) + framed-env unwrapping
(`emit/environments.rs`, tcolorbox/framed). Asset plan for non-image float sources.
**Fix sketch.** Out of scope for a quick win (TikZ→CeTZ is large), but: (a) unwrap
tcolorbox/framed table+figure envs so their inner float still emits; (b) emit a visible
placeholder for an un-renderable vector figure instead of dropping it silently. Track TikZ
rendering as its own epic.

---

## P1 — class-faithful typography gaps (the rubric's GAP rows, now confirmed)

### 3. Heading-size hierarchy is global-uniform, not per-class  — confirmed on icml, sev 4
**Symptom.** 31244 (ICML) section headings render ~18–24pt, dwarfing the ~10–11pt body; ICML's
headings are compact. The `#show heading.where(level…)` sizes in `build_neutral_preamble` are one
size for every class.
**Suspected site.** `emit/preamble.rs::build_neutral_preamble` (global heading show-rules) +
`style_profile.rs` (no `heading_sizes` knob).
**Fix sketch.** Add per-class heading size/spacing to `StyleProfile` (e.g. `heading_sizes: [&str;3]`,
`heading_spacing`), consume in the preamble show-rules. Snapshot per class.

### 4. ICLR small-caps title applied unconditionally  — iclr, sev 3
**Symptom.** 22820's title renders small-caps, but this paper's actual title is regular-weight
Computer Modern; the abstract heading is also wrongly small-caps.
**Suspected site.** `style_profile.rs` `Iclr` arm (`title_smallcaps: true` unconditionally).
**Fix sketch.** Confirm against the iclr_conference.sty in THIS corpus copy (older ICLR centers +
small-caps; some don't). If variable, gate on the detected sty variant or relax to non-smallcaps.
Re-verify the Unit-1 ICLR truth claim.

### 5. Figure float placement (no top/bottom floating)  — icml + general GAP, sev 2
**Symptom.** Wide figures don't span both columns / float to page top (31244); they sit inline,
shifting pagination. `#figure(...)` is emitted with no `placement:`.
**Suspected site.** `emit/figures.rs::emit_figure`.
**Fix sketch.** Map LaTeX `[t]/[b]/[p]` float hints → Typst `placement: top|bottom` (and `scope`
for full-width 2-col figures).

### 6. Hyperlink / cross-ref color not reproduced  — multiple, sev 1–2 (known GAP)
**Symptom.** Truth colors cite/ref/URL (blue/green/red hyperref boxes); typst renders them black
(22820, 22776).
**Suspected site.** `style_profile.rs` + `emit/preamble.rs` (no link show-rule).
**Fix sketch.** Detect `hyperref` `colorlinks`/`\hypersetup` colors → emit `#show link/ref/cite:
set text(...)` show-rules. Low severity; batch with other show-rules.

---

## P2 — parse/emit bugs (narrower, but real)

### 7. Inline math in section headings leaks as raw heading text  — 2 papers, peak sev 4
**Symptom.** 22159 `\section` titles with inline math render the raw math as a full-size heading
(`⟨f, gh⟩(X×B)` p.6, `⟨f, ∂x⟩(V'×V)` p.14); 22507 a `\paragraph` run-in header with inline math is
mis-segmented.
**Suspected site.** `emit/sections.rs::emit_section` (heading-title extraction when the title
contains a math zone).
**Fix sketch.** Render the heading title through the normal inline pipeline (so `$…$` becomes a
Typst math span inside the heading) instead of leaking the math token stream.

### 8. LNCS table corruption: `\multirow` + `\cmidrule`  — lncs, sev 5
**Symptom.** 31598 Table 1: every numeric data cell empty; `[]{1-5} table.cell(rowspan: 3)[…]`
leaks as raw source into the Model column; header cells render literal `*Model*` `*Method*`
(asterisks, not bold).
**Suspected site.** `emit/tables.rs` (`\multirow`/`\cmidrule{1-5}` handling; bold `**` emitted in a
non-interpreting cell context; data columns dropped).
**Fix sketch.** Reproduce with a minimal `\multirow`+`\cmidrule` fixture; fix rowspan/cmidrule
parsing so data cells aren't consumed and `\textbf` in a cell emits Typst strong, not literal `*`.

### 9. Cleveref double-prefix  — lncs, sev 4
**Symptom.** 31598: "Fig. **Figure** 3", "Section **Section** H.1" — `\cref`/`\Cref` prepends a
literal type word on top of an already-prefixed resolved reference.
**Suspected site.** the `\cref`/`\Cref` emitter (cleveref handling in `emit/bibliography.rs` or refs).
**Fix sketch.** Emit the bare `@label` (Typst supplies the "Figure"/"Section" prefix via the
reference) instead of prepending the cleveref prefix word.

### 10. Body escaping leakage  — neurips/article, sev 2
**Symptom.** 22765 `bert-base-uncased` → `bertext{-}baseext{-}uncased` (literal `{-}`/`\text`
artifacts); 22159 `ß` dropped.
**Suspected site.** inline text escaping / `{-}` brace-group handling; non-ASCII passthrough.
**Fix sketch.** Trace the `{-}` and `ß` cases to the inline emitter; add fixtures.

### 11. `\thanks` / author footnotes not split to page bottom  — multiple, coupled to #1
**Symptom.** Page-1 `\thanks` affiliation/email footnotes are dumped inline into the author block
instead of rendered at page bottom (22820, 31526, 22159).
**Suspected site.** author parsing (#1) + no `\footnote`/`\thanks` → Typst `#footnote` (rubric
footnotes GAP).
**Fix sketch.** Bundle with #1; route `\thanks` to a Typst footnote on the author.

### 12. LNCS running header/footer absent  — lncs, sev 1 (GAP)
Truth has "8  C. Eyzaguirre et al." running heads; typst has none. Low priority.

---

## Notes
- **Validation of the loop.** None of P0/#7/#8/#9/#10/#11 is detectable by the structural metrics
  (the words are all present; SSIM at 100 DPI can't see a leaked `\,` or a wrong heading size).
  The vision loop surfaced every one on the first 8-paper run. This is the answer to "the visual
  feedback loop is not strong enough."
- **Suggested fix order:** #1 (author block — 6 papers) → #3 (heading sizes) → #7 (heading math
  leak) → #8 (table multirow) → #9 (cleveref) → then the P1 typography show-rules (#4/#5/#6) → the
  larger #2 (vector floats) epic.
- Per-paper raw findings are in `tests/visual/<id>/findings.json` (gitignored; regenerate with the
  audit command in `docs/scorecard.md`).
