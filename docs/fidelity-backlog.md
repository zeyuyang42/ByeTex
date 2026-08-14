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

## Launch audit (2026-07-26) — undisclosed limitations

Found by a full sweep of every public claim against measured ground truth, run before
posting the project publicly. These are **real gaps a new user hits early**; ranked by how
fast they bite. Two findings from the same sweep were fixed immediately (TikZ silent drop,
convert purity — v0.7.3) and are not repeated here.

### L1. `.eps` / `.ps` / `.pgf` figures become grey placeholder boxes — sev 4, wide
Typst cannot read EPS, so `emit/figures.rs` emits a labelled grey rect. On 2605.22821 that
is **32 of 34 figures**. Many older arXiv sources ship EPS exclusively. Candidate fix:
shell out to ghostscript / `epstopdf` during project materialisation when available, and
warn (rather than silently box) when not.

### L2. A tree-sitter parse failure produces NO warning — sev 4
`Category::ParseError` is never constructed anywhere, so a document whose parse collapsed
degrades silently. 64 of 495 synthetic snippets (13%) fail to parse. Worst known case
(agent-surface L1): the parse root became one ERROR node and **11 of 12 section headings
were dropped** while the output still compiled cleanly — invisible to the compile gate.
Partially recovered by #452/#453; 2605.22728 still recovers only ~8/12. Emitting the
category at all would at least make it visible.

### L3. Page density diverges from the original — sev 3, DEFERRED
`page_ratio` across the corpus runs 0.31 → 1.33. Known: NeurIPS **34pp vs 26pp** (H2),
IEEEtran **8pp vs 6pp** (M3). Both previously deferred because naive margin tightening
regressed the composite score. Needs per-class `StyleProfile` spacing, not global tweaks.

### L4. Books / theses have no style profile — sev 4
No `DocClass` variant for `book`/`report`/`memoir`/thesis classes; they fall through to
`ArxivArticle` or `Unknown` and get the neutral preamble. Measured: `gh-amberj-latex-book-template`
`page_ratio` **0.312**, `gh-dzwaneveld-tudelft-thesis` **0.50** (both `structure_failed`).
6 of 8 book/thesis corpus entries have **no truth render at all**, so they are ungated.
The README says the corpus spans books and theses — true, but they are the weakest area.

### L5. Beamer decks are always re-themed to touying/metropolis — sev 3
`emit/preamble.rs` hardcodes `themes.metropolis` regardless of the source theme.
`corpus/beamer-demo` is a *Madrid* deck scored against a metropolis render — so the beamer
fidelity number is measured against the wrong target. Beamer also requires a network fetch
of `@preview/touying`.

### L6. `Fig.~\ref{x}` renders as "Fig. Figure 3" — sev 4 — ✅ RESOLVED (PR #469, v0.7.4)
See §9 below for the root cause and the two sharp edges that reverted the first attempt.
**Resolution.** Emit the empty-supplement SHORTHAND `@key[]` for plain `\ref` and `\eqref`
(and `#ref(<key>, supplement: none)` inside math, where a bare `@key` is an identifier)
instead of the `#ref(...)` FUNCTION form the reverted attempt used. That structurally
removes both blockers: `@key[]` cannot be call-applied, so `\ref{x}(ii)` stays literal
text; and it carries no `<`/`_` for the table-cell escaper to mangle. `\eqref` was the
louder half of the same bug — `(@key)` rendered "(Equation 1)" where LaTeX prints "(1)".
`\cref`/`\Cref`/`\autoref` keep the bare `@key` (they DO print a prefix in LaTeX);
cleveref's no-prefix `\labelcref` is included in the fix. Deliberately NOT included:
`\vref` (varioref — counter + "on page N"; ByeTex does not model the page half at all,
so half-converting it would be worse), `\pageref`/`\cpageref` (page numbers) and
`\nameref` (prints a name, not a counter).
Two incidental wins: the form self-terminates, so the adjacency guard that turned
`\ref{a}--\ref{b}` into "1 –2" no longer fires, and a closing `_` can't absorb it, so
`\emph{… \ref{x}}` keeps the compact shorthand instead of falling back to `#emph[…]`.

### L7. No float placement — sev 2
`#figure(...)` is emitted with no `placement:`, so wide figures neither span columns nor
float to the page top, which shifts pagination on every two-column class. (Also §5.)

### L8. `\newcommandx` (xargs) and `\ifthenelse` are unsupported — sev 3, narrow but brutal
On one corpus paper this is **840 of 943 warnings (89%)**. Marked HARD previously.
Related: `\makeatletter` `@`-commands leak as strings (19× on one paper).

### L9. Silent, unwarned formatting drops at corpus scale — sev 2, very wide
From `docs/fidelity-nonvisual-audit.md`, "fidelity lost with NO warning": `\vspace`/`\hspace`
(39 papers, 1205×), `\definecolor` (28), `\textcolor` (23 papers, 310×), `\resizebox` (21),
`>{}/<{}/@{}` column decorators (18), `\item[custom-label]` (16 papers, 268×), `enumitem`
list styles (15), `\colorbox` (3). Several are already handled but still counted; the audit
over-reports. **Worth re-verifying which are genuinely unhandled before acting** — a probe
during this sweep found `\textcolor`, `\definecolor`, `\cmidrule` and `\item[label]` all
working, so this list is stale as a whole.

### L10. `acmart` and `elsarticle` are advertised but never validated — sev 3
The corpus contains **no** paper in either class (census: 40 `article`, 4 `amsart`,
3 `IEEEtran`, 2 `svmult`, 2 `llncs`, 1 `revtex4-1`, plus one-offs). Both have `StyleProfile`
unit tests but no real-paper evidence. Either add one of each to the corpus or soften the
claim.

### L11. Three corpus papers are ungated — sev 2
`beamer-demo`, `ctan-memoir` and `gh-maurovm-thesis-template` are in neither `known_pass`
nor `known_fail` in `scripts/acceptance_baseline.json`, so a regression in them fails
nothing. `gh-maurovm-thesis-template` additionally hits the ambiguous-entry error (two
same-depth `\documentclass` files, neither named `main`).

### L12. `#cite(form:)` is gated on a real `.bib` — sev 2
Papers that ship only a `.bbl` / inline `thebibliography` fall back to bare `@key`, losing
`\citet`-style prose citations. Deliberate (it prevents an abort), but undisclosed.

### L13. A `\ref`/`\cite` inside a TABLE CELL renders as literal text — sev 3
Found while fixing L6, **pre-existing on main** and independent of it. `escape_text_cell`
escapes the `@` sigil (`[see Fig.~\@fig:a]`), so every reference and citation inside a
table cell is emitted as a dead literal instead of a link — the number never resolves.
The escaper is right to escape a *source* `@`, but wrong to escape one ByeTex itself
emitted. Candidate fix: make the cell escaper ref/cite-aware (copy an emitted `@key`,
`@key[]` or `#ref(...)` token through verbatim, the way it already copies `#raw(...)`).

---

## P0 — high frequency, high severity

### 1. Author-block LaTeX leakage / mangling  — 6 of 8 papers, peak sev 5  — ✅ RESOLVED (PR #219)

**Resolution (2026-06-12).** Two-stage "sanitize → parse" (`class_map.rs::sanitize_author_block`):
a denylist tokenizer strips comments + non-displaying spacing macros (`\,`/`\;`/`\hspace{}`/`~`/`&`/
`\|`) + unknown braced commands, unwraps font-style wrappers (`\textbf`/`\small`/…) keeping inner
text, and preserves accents (`\"u`→ü) + `\\`/`\quad` separators. `parse_generic_block` now splits
`\and` / comma-names+shared-`\\`-lines / `\textbf{a \quad b}` groups; substantive `\thanks`→
affiliation/email. Two load-bearing fixes found while re-grading: an `emit.rs` `\author` capture
that brace-matched its real extent (tree-sitter mis-bounds bare comma lists), and
`refine_from_package` now matching path-prefixed conference packages (`style/neurips_2026`) — which
ALSO restored NeurIPS/ICML/ICLR title+abstract styling on those papers. Re-graded 2605.22507 (now
3 clean authors + affiliations + rules), 22765 (`\quad`-row split), 22159 (un-glued). **Residuals
logged below as 1a/1b (out of the stop-the-leakage scope).**

- **1a (sev 2):** `\newcolumntype{C}[1]{>{}p{#1}}` p-column spec still leaks above Keywords on 22820 —
  a *preamble capture-boundary* issue (the spec leaks as body text, not via the author block).
  Investigate `\newcolumntype` handling in emit (it should be consumed like a definition, not emitted).
- **1b (sev 1):** `&`-separated authors (`Carlos Heredia & Daniel Roncel`, 22820) render the `&` as a
  literal ampersand without splitting — can't blindly split on `&` (legitimate in "ICREA & Univ").
  Low value; revisit only if a real template needs it. `ß`→glyph loss (22159) is a Typst font issue.

---

**Original report.**
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

### 3. Heading-size hierarchy is global-uniform, not per-class  — confirmed on icml, sev 4  — ✅ RESOLVED (PR #220)
**Resolution.** Added `StyleProfile.heading_sizes: [&str;3]`, consumed in `build_neutral_preamble`.
ICML/NeurIPS/ICLR/LNCS/SvMult → `[1.2em,1.0em,1.0em]` (their `\large\bf`/`\normalsize` sectioning at
a 10pt body, verified against the class `.sty`/`.cls` `\@startsection` fonts); article + every
unprofiled class keeps the historical 1.44/1.2/1em (byte-identical). Re-graded 2605.31244 — section
headings now proportionate.

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

### 7. Inline math in section headings leaks as raw heading text  — ✅ NOT A CONVERTER BUG; metric artifact fixed (PR #221)
**Diagnosis.** Investigated 22159: byetex's `\section` titles with inline math convert CORRECTLY
(`\section{… $\Omega$ …}` → `== … $Omega$ …`). The `⟨f, gh⟩(X×B)` "heading" the grader saw was the
second line of a multi-line `$ … $` **display equation** (`<eqn:DSP>`) whose `=` is the equation's
equals sign — `scripts/visual_test.py::typ_headings` regex-matched the `=`-leading line as a heading
with no math-block awareness. The ICML `heading_recall 0.45` was the same class of artifact (`\paragraph`-
level `#heading(level: 4,…)` run-ins over-counted vs `source_headings`' level-1-3 scope).
**Fix.** `typ_headings` now tracks `$…$` parity (skips `=`-lines inside an open math block) and caps
at heading levels 1-3 (markers `={1,3}`; `#heading(level: N>3)` excluded) to match `source_headings`.
Re-measured: 22159 heading_recall → 1.00; **2605.31244 (ICML) → 1.00 and flipped structure_failed → ok**.
The residual real defect nearby is a broken custom-operator macro (`\opV` → `op("\opV_{\mathgroup=-1…}")`)
— a separate math/macro item, not a heading bug. Strengthens the loop's heading metrics.

### 8. LNCS table corruption: `\multirow` + `\cmidrule`  — lncs, sev 5
**Symptom.** 31598 Table 1: every numeric data cell empty; `[]{1-5} table.cell(rowspan: 3)[…]`
leaks as raw source into the Model column; header cells render literal `*Model*` `*Method*`
(asterisks, not bold).
**Suspected site.** `emit/tables.rs` (`\multirow`/`\cmidrule{1-5}` handling; bold `**` emitted in a
non-interpreting cell context; data columns dropped).
**Fix sketch.** Reproduce with a minimal `\multirow`+`\cmidrule` fixture; fix rowspan/cmidrule
parsing so data cells aren't consumed and `\textbf` in a cell emits Typst strong, not literal `*`.

### 9. Reference double-prefix  — lncs+others, sev 4  — ✅ RESOLVED (PR #469, v0.7.4; see L6)
**Symptom.** "Fig. **Figure** 3", "Section **Section** H.1".
**Root cause (2026-06-12, corrected).** NOT `\cref`. byetex converts `\cref`/`\ref` correctly. The
double-prefix is from plain **`\ref`**: authors very commonly write `Fig.~\ref{x}` / `Section~\ref{x}`
(manual prefix). LaTeX `\ref` renders only the counter ("3"), but byetex maps `\ref` → `@key`, and
Typst's `@key`/`#ref` AUTO-prepends "Figure"/"Section" → "Fig. Figure 3". `\cref`/`\autoref` (which
SHOULD prefix) keep `@key` and are correct.
**Attempted fix (REVERTED — too many sharp edges to land cleanly this session).** Map plain `\ref` →
`#ref(<k>, supplement: none)` (counter only, faithful). This is correct in principle and passed unit
tests, but the `#ref(...)` FUNCTION form is fragile where the `@key` shorthand was robust, causing
**compile regressions** the acceptance gate caught: (a) `\ref{x}(ii)` → `#ref(…)(ii)` parses `(ii)`
as a CALL (`unknown variable: ii`, 2605.22800) — fixable with a trailing-space guard before `(`/`[`/`.`;
(b) `\ref` inside a **table cell** gets its `<…>`/`_` escaped by the cell-content escaper →
`#ref(\<sec\_x>…)` → "character `\` is not valid in code" (2605.31072). (b) is the blocker: the cell
escaper mangles the fn-form's label. Churns ~6 ref test files too.
**How it was finally fixed (PR #469).** Neither of the two options below — a THIRD one avoids
the problem entirely: Typst's `@key[…]` shorthand takes a supplement as a trailing content
block, so the EMPTY block `@key[]` gives the counter-only render without ever leaving the
robust shorthand shape. Nothing to re-escape, nothing callable. Options (1) and (2) are kept
below for the record.

**Better approach for a clean future fix (superseded):** either (1) make the cell/escaping path ref-aware so an
emitted `#ref(...)` is never re-escaped, THEN re-apply the `\ref`→`supplement: none` + the `(`/`[`/`.`
adjacency guard (both are written-and-tested in git history of the reverted `fidelity-cleveref`
branch); or (2) a global preamble show-rule that strips the supplement for plain refs without the
fn-form (investigate whether `#show ref:` can distinguish `\ref` from `\cref` call sites — likely
needs a per-call marker, so (1) is more tractable). Keep `@key` for `\cref`/`\autoref`/`\eqref`.

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
