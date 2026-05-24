# Visual Regression Findings — 2026-05-23

**Test method:** `scripts/visual_test.py` — for each arXiv paper, run `byetex convert`
→ `typst compile` → rasterize both PDFs → **PDF source-data structural
comparison** (Round 5, see "Structural gate" below) → compare
side-by-side composite against the canonical arXiv PDF. Agent graded
composites visually.

## Structural gate (added 2026-05-23 night)

Visual review alone was too lenient — the script declared
`status: "ok"` on `2605.22315` while the user judged it "not correct
at all". A new step now runs between rasterize and composite:

1. `pdftotext -layout` extracts plain text from both `truth.pdf` and
   `typst.pdf`.
2. Letters-only tokens (length ≥ 3) form a `set`:
   - `word_jaccard = |T ∩ Y| / |T ∪ Y|`
   - `word_recall = |T ∩ Y| / |T|` (vocabulary coverage of the truth)
3. Heading-like lines (numbered or short Title-Case, with math-glyph
   and equation-residue lines filtered out) are matched substring-wise
   plus a synonym table (`references` ↔ `bibliography`,
   `acknowledgements` ↔ `acknowledgments`, etc.). Yields
   `heading_recall`.
4. `page_ratio = typst_pages / truth_pages` is bounded to [0.70, 1.30].

Each paper gets `tests/visual/<id>/structure.json` with the metrics +
`structure_ok` verdict + `fail_reasons`. `status: "ok"` now requires
both `typst_ok` **and** `structure_ok`. CLI flags
`--min-word-jaccard` / `--min-word-recall` / `--min-heading-recall` /
`--min-page-ratio` / `--max-page-ratio` override the defaults
(0.55 / 0.65 / 0.60 / 0.70 / 1.30). `--no-structure-check` bypasses.

**Round 5 baseline (with structural gate enforced):**

| stage | count |
|---|---|
| typst_ok | 1/26 |
| structure_ok | 1/26 |
| overall_ok | 1/26 (`2605.22315`: jaccard 0.85, recall 0.91, headings 0.71) |


**Round 1 (Claude Opus 4.7):** 5 arXiv papers — 2/5 compiled; both show ~97% content loss; 3/5 failed typst compile.  
**Round 2 (Claude Sonnet 4.6):** 26 arXiv papers — 6/26 compiled; all 6 produce 1-page output; 20/26 failed typst compile.  
**Round 3 (after Bugs #1–#7 fixed):** 26 arXiv papers — 0/26 compiled; `\input` expansion now works so full papers are processed, exposing a new tier of errors. Bugs #10–#13 identified and fixed.  
**Round 4 (after Bugs #1–#13 fixed):** 26 arXiv papers — 0/26 compiled; seven new blocking patterns documented as Bugs #14–#20. Bugs #14 and #15 were fixed alongside Round-3 work; Bugs #16–#20 remain pending.

Bugs #1–#3 documented in Round 1. Bugs #4–#9 are new findings from Round 2. Bugs #10–#13 are Round 3 findings (all fixed). Bugs #14–#15 fixed in same patch as Round-3 work. Bugs #16–#20 are Round 4 findings (pending).

---

## Fix status (2026-05-23 evening)

Bugs #1–#7 are addressed at the converter level. Each fix has unit tests
under `crates/byetex-core/tests/`; the wider visual-compile gating moves to
the next layer of conversion gaps (matrix/array nesting, `_#` subscripts,
`\left`/`\right` delimiters, Typst's deprecated `diff` for `\partial`,
`\mathbb{R}` adjacency, `.bib` resolution) which are *not* the prescriptions
in this doc.

| Bug | Prescription summary | Status | Test |
|-----|----------------------|--------|------|
| #1 `\input` dropped | Resolve relative to base dir; expand inline; detect cycles | **Fixed**. `ConvertOptions::base_dir` now drives recursive expansion. `2605.22507`'s `.typ` grew from 60 → 1701 lines. | `tests/input_expansion.rs` (8 cases) |
| #2 `\linewidth` raw | Bare `\linewidth` / `\textwidth` / `\columnwidth` → `100%` | **Fixed**. `normalize_graphics_length` handles the no-coefficient form. | `tests/golden_m4.rs::m4_figure_bare_*` (3) |
| #3 unbalanced math brackets | Escape unmatched `[`, `]`; also escape orphaned `(`, `)` (else the partner-kind reports `unclosed`) | **Fixed**. `escape_unbalanced_math_brackets` post-processes every math body. | `tests/golden_m3.rs::m3_half_open_interval_*`, `m3_balanced_brackets_*` |
| #4 `\dagger` / `\ddagger` | Add to math symbol table | **Fixed**. Mapped to `dagger` / `dagger.double` (also `\prime`). | `tests/golden_m3.rs::m3_dagger_ddagger_in_math_table` |
| #5 letter+command fusion (`t\in` → `tin`) | Separator before alphabetic math symbol when previous is a letter | **Fixed**. `push_math_symbol` inserts a space at the boundary. | `tests/golden_m3.rs::m3_letter_then_math_command_keeps_separator` |
| #6 `\(` / `\)` verbatim | Treat as inline-math delimiters | **Fixed**. Math child filter drops `\(` and `\)` so the wrapped body emits as `$...$`. | `tests/golden_m3.rs::m3_paren_math_delimiters_treated_as_inline_math` |
| #7 `\newtheorem` verbatim | Drop the definition silently | **Fixed**. `theorem_definition` joins `new_command_definition` / `counter_declaration` in the silent-drop branch. | `tests/golden_m4.rs::m4_newtheorem_dropped_silently` |
| #8 BibLaTeX `@string` macros | Preprocess `.bib` to substitute string macros before handing to Typst | **Deferred**. Touches the CLI's bibliography handoff, not the converter. Tracked for the next iteration. | — |
| #9 missing `.bib` path | Probe alternative `.bib` locations | **Deferred**. Same as #8 — moved to the bibliography track. | — |
| #10 math `\#` triggers code mode | Keep the backslash in `\#` / `\$` / `\&` / `\_` / `\{` / `\}` math mappings | **Fixed**. `f_\#` now emits `f_(\#)`, accepted by Typst as the math escape (was emitting `f_(#)` → "unexpected closing paren"). | `tests/golden_m3.rs::m3_math_escape_for_hash_dollar_etc` |
| #11 `\mathbb`-style wrap fuses with prior letter | Generalize the letter-boundary check from `push_math_symbol` to every letter-starting wrapper (`bb(`, `sqrt(`, `binom(`, `op(`) | **Fixed**. `ensure_math_letter_boundary` is now called from `emit_math_wrap` / `_sqrt` / `_binom` / `_operatorname`. `\in\mathbb{R}` emits `in bb(R)` instead of the fused `inbb(R)`. | `tests/golden_m3.rs::m3_mathbb_does_not_fuse_with_preceding_letter` |
| #12 backticks in body trigger Typst raw | Switch `\texttt{X}` from `` `X` `` to `#raw("X")`; escape stray source backticks in the typography pass | **Fixed**. `` `partial' `` from LaTeX left-single-quote escapes to `` \`partial' ``; `\texttt{X}` emits the function form (no backticks). | `tests/golden_m2.rs::m2_lone_backtick_in_body_gets_escaped`, `m2_texttt_uses_raw_function_form` |
| #13 `\partial` emits deprecated `diff` | Map `\partial` to `partial`; `\langle`/`\rangle` to `chevron.l`/`chevron.r` | **Fixed**. Removes the deprecation warning cascades on math-heavy papers. | `tests/golden_m3.rs::m3_partial_uses_modern_typst_name` |
| #14 `\left`/`\right` emitted verbatim (22/26 papers) | Remove sizing delimiters; drop `\left`/`\right` prefixes so Typst auto-pairs the delimiter | **Fixed** (in same patch as Round-3 work). `emit.rs` line 3214 maps `\left`/`\right` → `""`. | `tests/golden_m3.rs::m3_left_right_strip_for_balanced_parens` |
| #15 Math spacing macros fuse with adjacent letter (25/26) | Map `\thinspace` → `thin` and apply letter-boundary guard before the token | **Fixed** (in same patch as Round-3 work). `emit.rs` line 3197 maps `\thinspace` → `thin`; `push_math_symbol` inserts separator. | `tests/golden_m3.rs::m3_thin_space_doesnt_fuse` |
| #16 `\label{key_with_underscores}` splits in math (11/26) | Extract full label text past `_` or pre-escape `_` inside label braces | **New — pending fix**. `\label{eq:edl_objective}` → `<eq:edl>` + `""_ob j e c t i v e}` artifact. | — |
| #17 Unescaped `_`/`*`/`#` in array cell content (15/26) | Escape `_`, `*`, `#` in `emit_tabular` cell content | **New — pending fix**. `[_sla]` opens unclosed italic markup. | — |
| #18 `\def` primitives emitted verbatim (6/26) | Add `\def`, `\edef`, `\gdef` to the silent-drop list alongside `\newcommand` | **New — pending fix**. `\def\vocab{K}` passes through as backslash syntax error. | — |
| #19 `image("???")` placeholder fails compile (12/26) | Emit a Typst `rect()` placeholder or comment block instead of `image("???")` | **New — pending fix**. Typst aborts when the file `???` is not found. | — |
| #20 `\\[length]` linebreak+spacing in math (≥3/26) | Strip `\\[Nmm]` to `\` in align/gather bodies; Typst does not support inter-row vertical space | **New — pending fix**. `\[1mm\]` produces unclosed delimiter in alignment blocks. | — |

---

## Bug #1 — `\input` directives are dropped, not expanded (P0 — BLOCKER)

### What happens

Every `\input{file.tex}` and `\include{file.tex}` directive is categorised as
`needs_manual_review` and silently dropped from the output. byetex only converts the
direct content of the top-level `.tex` file.

Since virtually all real-world arXiv papers split content across multiple files, the
output `.typ` contains only what is literally in the root file — usually a title block
and abstract — and discards all body sections.

### Evidence

Paper `arxiv:2605.22507` ("Generative Modeling by Value-Driven Transport", stat.ML):
- `0-main.tex`: 109 lines; uses `\input{1-intro.tex}` … `\input{5-conclusion.tex}`
- `0-main.typ` output: 60 lines — title block + abstract only
- Truth PDF: 30 pages. Typst PDF: **1 page** (Δ = −29 pages, 97% content loss)

Warning sidecar confirms all `\input` calls flagged:
```
needs_manual_review | \input{style/header.tex}
needs_manual_review | \input{1-intro.tex}
needs_manual_review | \input{2-prelim.tex}
needs_manual_review | \input{3-algorithm.tex}
needs_manual_review | \input{4-experiments.tex}
needs_manual_review | \input{5-conclusion.tex}
needs_manual_review | \input{999-app_1_literature.tex}
...
```

Paper `arxiv:2605.22557` (math.NA): 28-page truth → 1-page typst (Δ = −27), same cause.

### Fix

**Implement `\input` / `\include` expansion in the converter.** When byetex encounters
an `\input{path}` or `\include{path}` node:

1. Resolve `path` relative to the directory of the currently-processed `.tex` file.
2. Read the included file and parse it through the same tree-sitter LaTeX grammar.
3. Emit the included file's content inline at the point of the `\input` directive.
4. Track included paths to detect circular includes.
5. Propagate warnings from included files, annotated with the source file path.

**Relevant code to change:**
- `crates/byetex-core/src/emit.rs` — the main node-dispatch function where `\input`
  is currently producing a `needs_manual_review` warning. Change the handler to recurse.
- `crates/byetex-core/src/lib.rs` — the top-level `convert(source: &str)` signature
  currently has no knowledge of the filesystem. To support `\input`, a new entry point
  (or an additional parameter) must supply the base directory path so included files can
  be resolved. Example signature: `convert_file(path: &Path) -> ConversionResult`.
- `crates/byetex-cli/src/main.rs` — the `convert` subcommand already receives the
  file path; pass it down to the new `convert_file` entry point.

**Verification:** After the fix, re-run:
```bash
python scripts/visual_test.py --papers 2605.22507 2605.22557
```
Expected: `typst_pages` climbs from 1 toward the `truth_pages` count (30 and 28
respectively). Residual page-count delta will reflect the next tier of conversion gaps.

---

## Bug #2 — LaTeX dimensions emitted raw into Typst `image()` width (P1)

### What happens

When converting `\includegraphics[width=\linewidth]{fig.png}` (or `\textwidth`),
byetex emits the Typst `image()` call with the LaTeX dimension token verbatim:

```typst
image("fig.png", width: \linewidth)   // INVALID Typst
```

Typst rejects `\` in code context, so `typst compile` fails immediately.

### Evidence

Paper `arxiv:2605.22776`:
```
error: expected expression
  ┌─ main_en.typ:157:34
157 │   image("SDPM_reverse.png", width: \linewidth),
```

Paper `arxiv:2605.22820`:
```
error: expected expression
  ┌─ main.typ:492:34
492 │   image("Figs/splines.png", width: \textwidth),
```

Both papers blocked from compiling; composites could not be produced.

### Fix

In the `\includegraphics` emitter, translate the `width=` option from LaTeX dimensions
to Typst percentages before emitting:

| LaTeX `width=` value | Typst `width:` value |
|---|---|
| `\linewidth`, `\textwidth`, `\columnwidth` | `100%` |
| `0.9\linewidth`, `0.9\textwidth` | `90%` |
| `0.5\linewidth` | `50%` |
| `N\linewidth` (general) | `{N*100}%` |
| `3cm`, `72pt`, absolute units | keep as-is: `3cm`, `72pt` |

**Relevant code:** `crates/byetex-core/src/emit.rs` — locate the `\includegraphics`
handler; add a dimension-translation step before writing the `width:` argument.

**Verification:** After the fix:
```bash
python scripts/visual_test.py --papers 2605.22776 2605.22820
```
Expected: both now reach `typst_ok: true` and produce composite PNGs for grading.

---

## Bug #3 — Unclosed math delimiters in Typst output (P1)

### What happens

Some LaTeX math constructs are converted to Typst math that has mismatched delimiters,
causing `typst compile` to fail with "unclosed delimiter" errors.

### Evidence

Paper `arxiv:2605.22159`:
```
error: unclosed delimiter
  ┌─ GS4AGBEM.typ:1575:14
1575 │   for some $sin(0,s_*]$ and $ "Const" (s)$
```

The LaTeX source likely has `$\sin(0, s_*]$` — an interval with `(` open and `]` close
(mixed delimiters, valid in math notation). byetex emits `sin` as a bare identifier
inside a Typst math expression, and the `]` closes the outer Typst delimiters prematurely.

Second error in the same file:
```
error: unclosed label
  ┌─ GS4AGBEM.typ:1597:21
1597 │          "diamT" ^s&"if"s<1/2,\
```

### Fix

**Two sub-issues:**

1. **`\sin` (and other standard math operators) emitted as bare `sin`:** In Typst math,
   standard operators are written unescaped (`sin`, `cos`, `max`) or with `op("sin")`.
   byetex should map `\sin`, `\cos`, `\max`, etc. to their bare Typst equivalents. Check
   the existing math symbol table in `crates/byetex-core/src/math.rs` (or equivalent)
   for any gaps in standard operator coverage.

2. **Mixed math delimiters `(a, b]` / `[a, b)`:** These are valid in math notation but
   Typst's math parser treats `[` and `]` as square brackets that must balance. The fix is
   to emit mixed-delimiter intervals as: `lr("(" + content + "]")` using Typst's `lr()`
   function which accepts unbalanced delimiter strings. Alternatively, escape as
   `\[` / `\]` in a raw string context.

**Relevant code:** `crates/byetex-core/src/emit.rs` — math expression emitter;
the math symbol table (likely in a separate `math_symbols.rs` or similar).

**Verification:**
```bash
python scripts/visual_test.py --papers 2605.22159
```
Expected: `typst_ok: true`; a composite PNG is produced. The paper has 1752 warnings
(unusually high — also worth checking for duplicates or O(n²) warning emission).

---

## Supplementary: 1752 warnings on a 10-page paper

Paper `arxiv:2605.22159` emits **1752 warnings for a 10-page source** (~175/page).
The typical arXiv paper in the corpus has 9–160 warnings. This outlier count may indicate:

- A warning emitted inside a loop or repeated expansion that runs O(n) per macro use
- A `\newcommand` with many call sites each emitting its own warning

Worth checking whether the warning count is a deliberate product of the document's
content or a performance regression in the emitter.

---

---

## Bug #4 — `\dagger` / `\ddagger` not in math symbol table (P1 — NEW)

### What happens

`${}^\dagger$` is emitted with `agger` as an unknown identifier. The `\d` prefix appears to be consumed silently, leaving `agger` as a bare Typst identifier.

### Evidence

Papers `arxiv:2605.22485` and `arxiv:2605.22584`:
```
error: unknown variable: agger
  ┌─ main.typ:7:22
7 │   Robert Altmann${}^\dagger$ \and ...
```

### Fix

Add to the math symbol table in `crates/byetex-core/src/emit.rs` (or `math_symbols.rs`):
- `\dagger` → `dagger.op`
- `\ddagger` → `dagger.double.op`
- `\star` → `star.op` (if not already present)

---

## Bug #5 — `\in` merged with preceding letter in math (P1 — NEW)

### What happens

`$t\in[0,T]$` is emitted as `$tin[0,T]$` — the letter `t` and control sequence `\in` are merged into the single token `tin`.

### Evidence

Paper `arxiv:2605.22315`:
```
error: unknown variable: tin
  ┌─ main4-final.typ:85:12
85 │   value and $tin[0,T]$ is the time variable
```

### Fix

In the math tokenizer/emitter, ensure a separator is emitted between a bare letter and a following control sequence. The letter `t` must be emitted before `\in` is processed.

**Relevant code:** `crates/byetex-core/src/emit.rs` — math expression walker.

---

## Bug #6 — `\(` / `\)` math delimiters emitted verbatim (P1 — NEW)

### What happens

`\(` and `\)` (LaTeX inline math delimiters, equivalent to `$...$`) are passed through to Typst output verbatim, producing invalid `\` in code context.

### Evidence

Paper `arxiv:2605.22724`:
```
error: the character `\` is not valid in code
  ┌─ Near-optimal_rates_arxiv.typ:152:12
152 │ where $\(N_#\)$ denotes ...
```

### Fix

In the command handler, treat `\(` as opening inline math and `\)` as closing it. If nested inside `$...$`, strip them. Relevant code: `crates/byetex-core/src/emit.rs`.

---

## Bug #7 — `\newtheorem` / `\newtheorem*` emitted verbatim (P1 — NEW)

### What happens

`\newtheorem*{remark}{Remark}` is passed through to Typst output verbatim, causing a backslash-in-code error.

### Evidence

Paper `arxiv:2605.22821`:
```
error: unclosed delimiter
  ┌─ main.typ:82:11
82 │ \newtheorem*{remark}{Remark}
  │            ^
```

### Fix

`\newtheorem` should be classified as `needs_manual_review` and **dropped** (with a warning), not emitted. Relevant code: `crates/byetex-core/src/emit.rs` — preamble command handler.

---

## Bug #8 — BibLaTeX `@string` macros not resolved (P2 — NEW)

### What happens

Some `.bib` files use `@string{key = "value"}` abbreviations. Typst's BibLaTeX parser does not support `@string` macros, causing compile failure.

### Evidence

Paper `arxiv:2605.22765`:
```
error: failed to parse BibLaTeX (unknown abbreviation "icassp")
  ┌─ bibliography.bib:1950:14
1950 │   booktitle = icassp,
```

### Fix

Post-process `.bib` files before handing to Typst: parse all `@string{key = "value"}` definitions, then replace bare `key` references in field values with their quoted strings. This can be done as a preprocessing step in the CLI when it detects a `#bibliography(...)` call in the emitted `.typ`.

---

## Bug #9 — Missing bibliography file not resolved (P2 — NEW)

### What happens

The bibliography file expected by the emitted Typst (`#bibliography("main.bib")`) is not found at the expected path.

### Evidence

Paper `arxiv:2605.22814`:
```
error: file not found (searched at .../source/main.bib)
  ┌─ main.typ:67:14
67 │ #bibliography("main.bib", style: "apa")
```

### Fix

The CLI should probe for the correct `.bib` file path the same way it probes for `.tex` files. If `\bibliography{refs}` is the LaTeX source, check `refs.bib`, `refs/refs.bib`, etc.

---

## Re-run checklist for the fixing agent

After each fix, run the visual test to confirm regressions don't appear:

```bash
# After Bug #1 fix (\input expansion) — full 26-paper corpus:
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22159 2605.22281 2605.22312 2605.22315 2605.22485 2605.22507 \
  2605.22549 2605.22557 2605.22579 2605.22584 2605.22724 2605.22728 \
  2605.22736 2605.22738 2605.22746 2605.22765 2605.22776 2605.22779 \
  2605.22786 2605.22794 2605.22795 2605.22800 2605.22814 2605.22817 \
  2605.22820 2605.22821

# After Bug #2 fix (\linewidth/\textwidth/\columnwidth in images):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22549 2605.22579 2605.22738 2605.22776 2605.22817 2605.22820

# After Bug #3 fix (math delimiters):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22159 2605.22281 2605.22312 2605.22728 2605.22736 2605.22786 2605.22821

# After Bug #4 fix (\dagger in math table):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22485 2605.22584

# After Bug #5 fix (\in merged with letter):
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22315

# After Bug #6 fix (\( \) verbatim):
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22724

# After Bug #7 fix (\newtheorem verbatim):
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22821
```

Full composite grading (once all 26 compile) should be re-run by an agent with vision.

---

## Context: existing findings this relates to

See `docs/test-results-2026-05-23.md` for the prior test run:

- **Finding #1** (macro density) — Bug #1 here (`\input` not expanded) is a concrete
  instance. The broader macro-expansion problem covers `\newcommand` too.
- **Finding #2** (ambiguous_math dominance) — Bug #3 here is one manifestation.
- Bug #2 (`\linewidth` in images) is **new** — not present in the prior findings.

---

## Round 3 findings (post-Bug-#1–#9 fixes, 2026-05-23 evening)

After Bugs #1–#7 landed, four new failure modes dominated the surviving
compile errors. Each is documented below and addressed in the same patch.

## Bug #10 — `\#` in math triggers Typst's code context (P0 — NEW)

### What happens

`f_\#` in LaTeX (literal `#` as subscript, often used for the
pushforward operator) was mapped in the math symbol table as `\#` → `#`.
Inside math, `#` is how Typst leaves math and enters code context, so
`f_(#)` was parsed as `f_(<code expression>)` and Typst reported
"unexpected closing paren" at the `)`.

### Evidence

Paper `arxiv:2605.22507`:
```
error: unexpected closing paren
447 │ ... function $f: "real" ^(d times d) "ra"   "real" ^d$, we use $f_(#)p$ ...
```

Paper `arxiv:2605.22724` shows the same pattern with `$N_#$`.

### Fix

Keep the leading backslash in the math symbol table for every char that
has special meaning in Typst code/math:

```
\\# → \#      \\$ → \$      \\% → \%      \\& → \&
\\_ → \_      \\{ → \{      \\} → \}
```

Typst then takes them as math escapes for the literal characters.

---

## Bug #11 — letter-starting math wrappers fuse with preceding letter (P0 — NEW)

### What happens

`\in\mathbb{R}` emitted as `inbb(R)`. The fix for Bug #5 — separator
between a letter and a math-symbol replacement (`\in` → `in`) — only
covered `push_math_symbol` (the lookup-table path). Function-call
wrappers (`bb(`, `sqrt(`, `binom(`, `op(`) wrote their prefix straight
through `push_str`, so the boundary check never fired.

### Evidence

Paper `arxiv:2605.22820`:
```
error: unknown variable: inbb
561 │ ... Let $q(p,x)inbb(R)^n_(+)$ denote the demand vector at prices $p inbb(R)^n_(+)$ ...
```

### Fix

Extract the separator logic out of `push_math_symbol` into
`ensure_math_letter_boundary(next: &str)` and call it from
`emit_math_wrap`, `emit_math_sqrt`, `emit_math_binom`, and
`emit_math_operatorname` before the letter-starting prefix.

---

## Bug #12 — stray backticks in body open Typst raw blocks (P1 — NEW)

### What happens

A lone `` ` `` in the source (LaTeX uses it as a left single quote, e.g.
`` `partial' ``) opens a Typst raw block. Without a matching closing
`` ` ``, Typst fails with "unclosed raw text". Earlier the
`\texttt{X}` → `` `X` `` emission would conflict with any escape we tried
in the typography pass, because both source and texttt-emitted backticks
look identical by the time the pass runs.

### Evidence

Paper `arxiv:2605.22821`:
```
error: unclosed raw text
514 │ grapheme or morpheme boundaries (analogously to `Boundless`; @schmidt2025boundlessbpe), ...
```

(The actual source of the unclosed raw is line 335's `` `partial' ``
single-quote pattern; the `Boundless` form is from `\texttt{Boundless}`
and is now emitted as `#raw("Boundless")` instead.)

### Fix

Two-part:

1. Change `\texttt{X}` to emit `#raw("X")` (Typst's function form of
   inline raw text), so we no longer emit any backticks ourselves.
2. Add a lone-backtick branch to `post_process_typography`: any `` ` ``
   left after the `` `` `` → `"` pair conversion is now a source-only
   single backtick, so escape it as `` \` ``.

---

## Bug #13 — `\partial` emits deprecated Typst symbol `diff` (P2 — NEW)

### What happens

Typst 0.13+ deprecated `diff` in favor of `partial` and `angle.l` /
`angle.r` in favor of `chevron.l` / `chevron.r`. Math-heavy papers (e.g.
`2605.22315`) emit dozens of deprecation warnings per compile.

### Evidence

```
warning: `diff` is deprecated, use `partial` instead
87 │ $cal(L) := (diff) / (diff t) + (1) / (2) sigma^2 S^2 (diff^2) / (diff S^2) + (r-delta)S (diff) / (diff S) - r$
```

### Fix

Update the math symbol table:

```
\\partial → partial   (was: diff)
\\langle  → chevron.l (was: angle.l)
\\rangle  → chevron.r (was: angle.r)
```

---

## Round 4 findings (post-Bug-#1–#13 fixes, 2026-05-23 late evening)

After Bugs #1–#13 are applied, 26/26 papers still fail to compile. The
dominant patterns are documented below as Bugs #14–#20.

---

## Bug #14 — `\left`/`\right` sizing delimiters emitted verbatim (P0 — **FIXED**)

### What happens

`\left(`, `\left[`, `\left\{`, `\right)`, `\right]`, `\right\}` pass through
to the Typst output verbatim. Typst's math mode does not know the LaTeX `\left`
macro: the `\` starts a backslash escape and the subsequent letters `eft` or
`ight` are parsed as unknown identifiers.

### Evidence

Paper `arxiv:2605.22315` (20 warnings):
```
error: unknown variable: eft
  ┌─ main4-final.typ:99:17
99 │   cal(L)V dot.c\left(V-G\right)=0, $ <BS-model>
```

Paper `arxiv:2605.22584` (8 `\left`/`\right` occurrences):
```
error: unclosed delimiter
  ┌─ main.typ:483:9
483 │ C = \left(
```

Paper `arxiv:2605.22795` (80 occurrences):
```
error: unknown variable: eft
  ┌─ Consevative_drifting_rates.typ:84:30
84 │ $ K_h(u)=(2pi h^2)^(-d/2)exp\left(-("norm"^2)/(2h^2)\right), $
```

**Scope:** 22/26 papers contain `\left` or `\right` in their `.typ` output.
This is the most widespread single compile blocker.

### Fix

In `crates/byetex-core/src/emit.rs`, handle `\left` and `\right` as sizing
wrappers rather than opaque commands:

- `\left(` + content + `\right)` → `lr("(" + content + ")")` using Typst's
  auto-sizing `lr()` function.
- `\left.` (empty/null delimiter, used for one-sided pairs) → drop the `\left.`
  entirely.
- Unmatched `\left` or `\right` (unclosed pairs due to multi-line alignment) →
  drop the sizing macro and emit the bare delimiter.
- `\left\{` / `\right\}` → `\{` / `\}` (already handled as math escapes by
  Bug #10 fix, but `\left` prefix needs removing).

The simplest safe default: **drop `\left` and `\right` verbatim**, keeping only
the delimiter character. Typst auto-sizes delimiters in math without hints.

**Relevant code:** `crates/byetex-core/src/emit.rs` — in the `emit_math_command`
dispatch (or `emit_generic_command` when inside math context). The node kind for
`\left(` in tree-sitter-latex is `left_right` or parsed as a sequence of
`generic_command` + delimiter.

---

## Bug #15 — Math spacing macros fuse with adjacent letters (P1 — **FIXED**)

### What happens

`\thinspace`, `\medspace`, `\thickspace`, `\negmedspace`, `\negthickspace` are
mapped to their Typst symbol names (`thin`, `med`, `thick`, etc.) via the math
symbol table. When the preceding token ends with a letter, the separator check
from Bug #11 does not cover these names, so the math symbol name fuses with the
adjacent letter.

### Evidence

Paper `arxiv:2605.22820`:
```
error: unknown variable: thind
  ┌─ main.typ:592:6
592 │ E(p,x)thind(log p), $
```
Source: `E(p,x)\thinspace d(\log p)` — the letter `d` and `thin` fuse to `thind`.

Paper `arxiv:2605.22485`:
```
error: unknown variable: dthin
  ┌─ AltMU26.typ:389:134
389 │ ... C_dthin || u ||_("cV") ...
```
Source: `C_d\thinspace\|u\|` — subscript letter `d` fuses with `thin`.

Paper `arxiv:2605.22549`:
```
error: unknown variable: nthin
  ┌─ main_mhsic.typ:93:49
93 │ $nthin "HSIC"_n$
```

**Scope:** 25/26 papers have `thin` in their `.typ` math output. In most cases
the spacing is at a word boundary and compiles fine, but any case where the
preceding token ends with a letter (including subscript letters) triggers the
fusion.

### Fix

The Bug #11 fix added `ensure_math_letter_boundary` to `push_math_symbol`.
Extend the same guard to spacing macros: when emitting `thin`, `med`, `thick`,
etc. in math mode, call `ensure_math_letter_boundary("thin")` before the emit.
Alternatively, map these to Typst's `#h()` horizontal-space function which does
not trigger identifier fusion:

```
\thinspace   →  #h(0.167em)
\medspace    →  #h(0.222em)
\thickspace  →  #h(0.278em)
\negthinspace → #h(-0.167em)
\negmedspace →  #h(-0.222em)
\negthickspace → #h(-0.278em)
```

**Relevant code:** `crates/byetex-core/src/emit.rs` — the math symbol table
entries for spacing macros (search for `"thin"` or `"thinspace"` in the symbol
map initializer).

---

## Bug #16 — `\label{key_with_underscores}` splits at `_` in math environments (P1 — NEW)

### What happens

Inside a math environment (equation, gather, align), tree-sitter-latex parses
`_` as the subscript operator even when it appears inside a `\label{...}` group.
So `\label{eq:edl_objective}` is parsed as:

- `\label{eq:edl}` — the label definition, with key `eq:edl`
- `_objective}` — a subscript on the implicit empty atom, with content `objective`

byetex lifts the label correctly (`<eq:edl>`), but then encounters the orphaned
subscript node and emits it as `""_ob j e c t i v e}` — each letter of `objective`
becomes a separate Typst math identifier, and the closing `}` is emitted verbatim.

### Evidence

Paper `arxiv:2605.22746` (4 occurrences):
```
error: unknown variable: ob
  ┌─ main.typ:196:5
196 │ $ ""_ob j e c t i v e}
```
Source LaTeX: `\label{eq:edl_objective}` inside `\begin{gather}`.

Paper `arxiv:2605.22776` (3 occurrences):
```
error: unknown variable: ex
  ┌─ main_en.typ:77:5
77 │ $ ""_ex p e c t a t i o n}
```
Source LaTeX: `\label{fig:expectation}` — the `_` in `expectation` triggers the split.

**Scope:** 11/26 papers contain the `""_[letter] [letter]` pattern in their
`.typ` output, indicating a label with underscores that was split.

### Fix

Two possible approaches:

1. **Post-process in `extract_label_name`**: After extracting the label key from
   the `curly_group_label` node, also scan the subsequent sibling nodes for a
   `subscript` node that immediately follows, and append its text content to
   reconstruct the full key (e.g., `eq:edl` + `_` + `objective` → `eq:edl_objective`).

2. **Pre-process the source**: Before handing the `.tex` to tree-sitter, escape
   `_` inside all `\label{...}`, `\ref{...}`, and `\eqref{...}` argument groups
   (e.g., replace `_` with `-` or a hex escape), then undo the escape when
   emitting `<...>` and `@...` labels. Fragile in general but scoped to a
   well-defined syntactic context.

**Relevant code:** `crates/byetex-core/src/emit.rs`:
- `extract_label_name` (~line 3240) — reads the `label` grandchild; extend to
  also consume a following subscript sibling.
- The `pending_math_label` logic (~line 257) — where the lifted label is stored.

---

## Bug #17 — Unescaped `_`, `*`, `#` in Typst array cell content (P1 — NEW)

### What happens

Typst's array/table syntax uses `[content]` for cell content, where content is
parsed as Typst markup. In that context, `_..._` is italic, `*...*` is bold,
and `#` enters code mode. LaTeX table cells often contain underscores (as
identifiers or subscripts), asterisks (as footnote marks), and `#` (as
column separators in raw data). These are emitted verbatim into `[...]` cells,
breaking the Typst markup parse.

### Evidence

Paper `arxiv:2605.22794`:
```
error: unclosed delimiter
  ┌─ main.typ:216:13
216 │   [T141zh], [_sla], [_compliance], [_audit],
```
The LaTeX cell `_sla` (an identifier starting with underscore) opens italic
markup in Typst but never closes it.

Paper `arxiv:2605.22786`:
```
error: unclosed delimiter
  ┌─ neurips_2026.typ:314:32
314 │   [*Topology*], [*Benchmark*], [*], [#Agents*], [*Method*], ...
```
The cell `[*]` opens bold markup with `*` but finds only `]` before the closing
`*` — unclosed bold. The cell `[#Agents*]` switches to code mode at `#`.

**Scope:** 15/26 papers contain `[_` or `[*` in their tabular output.

### Fix

In `emit_tabular`, when rendering cell content into `[...]`, apply an escape
pass over the plain-text portions:

- `_` → `\_` (escaped underscore, which Typst renders as a literal `_`)
- `*` → `\*` (escaped asterisk)
- `#` → `\#` (escaped hash, outside of intentional `#function()` calls)

For cells that byetex already knows contain math (wrapped in `$...$`), no
escaping is needed for those regions since `$...$` uses math mode parsing.
Only the non-math literal text portions need escaping.

**Relevant code:** `crates/byetex-core/src/emit.rs` — `emit_tabular` and the
cell-content rendering path it delegates to.

---

## Bug #18 — `\def` and TeX primitive definitions emitted verbatim (P1 — NEW)

### What happens

`\def\foo{...}`, `\edef\foo{...}`, `\gdef\foo{...}` are TeX primitives for
macro definition. byetex already silently drops `\newcommand`, `\renewcommand`,
and `\newtheorem` (Bug #7 fix), but the lower-level TeX primitives are not in
that drop list. They pass through to the Typst output verbatim, producing a
backslash character that Typst rejects in markup context.

### Evidence

Paper `arxiv:2605.22765` (146 `\def` occurrences):
```
error: unclosed delimiter
  ┌─ main.typ:118:12
118 │ \def\nsets{^*}
```
The `\d` is a backslash escape, `{^*}` is an unclosed brace group.

Paper `arxiv:2605.22820` (384 occurrences — the highest `\def` density in the corpus):
The paper uses `\def` extensively for shorthand math macros. All expand to
LaTeX notation that byetex cannot evaluate, so dropping them is the correct
fallback behavior (same as `\newcommand`).

**Scope:** 6/26 papers contain verbatim `\def` in their `.typ` output.

### Fix

In `crates/byetex-core/src/emit.rs`, add `\def`, `\edef`, `\gdef`, `\xdef`,
`\let`, and `\futurelet` to the silent-drop branch alongside `\newcommand` and
`\newtheorem`. Each should emit a `custom_macro` warning so the user knows the
definition was dropped and may need to manually substitute call sites.

---

## Bug #19 — `image("???")` placeholder causes compile failure (P1 — NEW)

### What happens

When `\begin{figure}` contains no `\includegraphics` (e.g., the graphics line
is commented out, or the figure contains only text), the emitter produces the
sentinel string `image("???")` as a placeholder. Typst attempts to open a file
literally named `???`, fails to find it, and aborts compilation.

### Evidence

Paper `arxiv:2605.22281` (9 occurrences):
```
#figure(
  image("???"),
  caption: [Deblurring and inpainting...],
) <fig:example-intro-plot>
```
The corresponding LaTeX figure had a commented-out `\includegraphics` line:
```latex
% \includegraphics[width=0.95\linewidth]{plot_00_flsqr.png}
```

**Scope:** 12/26 papers contain `image("???")` in their `.typ` output. In most
cases the primary compile error is a different bug, but once other bugs are fixed,
these placeholders become the next blocker.

### Fix

Change the no-graphics-found code path in `emit_figure` (~line 2118 of
`crates/byetex-core/src/emit.rs`) to emit a non-blocking placeholder:

```typst
rect(width: 100%, height: 4cm, fill: luma(230), stroke: 1pt)
```

This produces a grey box the size of a typical figure — visually obvious that
something is missing, but does not cause a compile abort. Alternatively, wrap
the `image("???")` in a `/* ... */` Typst comment so the output compiles cleanly
and the human reviewer can see exactly what needs to be replaced.

---

## Bug #20 — `\\[length]` linebreak+spacing in math environments (P1 — NEW)

### What happens

In LaTeX `align` and `gather` environments, `\\[1mm]` means "new row, add
1 mm of extra vertical space above the next row". byetex converts the `\\` row
separator to `\` (Typst's align row separator), but the optional length argument
`[1mm]` is not consumed. The `\[1mm\]` fragment then appears inline in the Typst
math expression, where `\[` is a character escape that Typst parses as an
unclosed square bracket.

### Evidence

Paper `arxiv:2605.22736`:
```
error: unclosed delimiter
  ┌─ GOTD_arxiv.typ:367:23
367 │   "displaystyle" min_(X in "eanifold") & f(X) \[1mm\]
368 │   "s.\,t." & h(X)=0, \[1mm\]
1080 │   \[-1mm\] $ $ <eq:CMP>
```
Source LaTeX: `\min_{X\in\mathcal{M}} & f(X) \\[1mm] \text{s.t.} & h(X) = 0`.

**Scope:** Confirmed in `arxiv:2605.22736`; the pattern `\[` appears in several
other `.typ` files with math alignment.

### Fix

In the `\\` (line-break) handler inside `emit_tabular` or the align-row emitter
in `crates/byetex-core/src/emit.rs`, after emitting `\` for the row break,
consume and discard the optional `[length]` argument if present. The length is a
formatting hint that Typst's align does not support; dropping it is the correct
behavior.

Pattern to detect and drop: after a `\\` row-break token, if the next
non-whitespace characters match `[N<unit>]` (where unit is `mm`, `cm`, `pt`,
`em`, `ex`, `in`), consume through the closing `]` without emitting anything.

---

## Re-run checklist for the next fixing agent

After each fix, run the visual test to confirm forward progress:

```bash
# Verify all 4 inhouse templates still compile (regression guard):
cargo test -p byetex-core --test template_budgets

# After Bug #14 fix (\left/\right):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22315 2605.22584 2605.22724 2605.22795 2605.22814

# After Bug #15 fix (spacing macros):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22485 2605.22549 2605.22820

# After Bug #16 fix (label underscore split):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22746 2605.22776

# After Bug #17 fix (table cell escaping):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22738 2605.22779 2605.22786 2605.22794 2605.22800

# After Bug #18 fix (\def verbatim):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22765 2605.22820

# After Bug #19 fix (image(???) placeholder):
uv run --with requests --with Pillow python scripts/visual_test.py --papers \
  2605.22159 2605.22281

# After Bug #20 fix (\\[length] in math):
uv run --with requests --with Pillow python scripts/visual_test.py --papers 2605.22736

# Full corpus run (expect >0/26 compile after all fixes land):
uv run --with requests --with Pillow python scripts/visual_test.py
```
