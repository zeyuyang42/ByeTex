# Visual Regression Findings — 2026-05-23

**Test method:** `scripts/visual_test.py` — for each arXiv paper, run `bytetex convert`
→ `typst compile` → rasterize both PDFs → compare side-by-side composite against the
canonical arXiv PDF. Agent graded composites visually.

**Round 1 (Claude Opus 4.7):** 5 arXiv papers — 2/5 compiled; both show ~97% content loss; 3/5 failed typst compile.  
**Round 2 (Claude Sonnet 4.6):** 26 arXiv papers — 6/26 compiled; all 6 produce 1-page output; 20/26 failed typst compile.

Bugs #1–#3 documented in Round 1. Bugs #4–#9 are new findings from Round 2.

---

## Fix status (2026-05-23 evening)

Bugs #1–#7 are addressed at the converter level. Each fix has unit tests
under `crates/bytetex-core/tests/`; the wider visual-compile gating moves to
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

---

## Bug #1 — `\input` directives are dropped, not expanded (P0 — BLOCKER)

### What happens

Every `\input{file.tex}` and `\include{file.tex}` directive is categorised as
`needs_manual_review` and silently dropped from the output. bytetex only converts the
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

**Implement `\input` / `\include` expansion in the converter.** When bytetex encounters
an `\input{path}` or `\include{path}` node:

1. Resolve `path` relative to the directory of the currently-processed `.tex` file.
2. Read the included file and parse it through the same tree-sitter LaTeX grammar.
3. Emit the included file's content inline at the point of the `\input` directive.
4. Track included paths to detect circular includes.
5. Propagate warnings from included files, annotated with the source file path.

**Relevant code to change:**
- `crates/bytetex-core/src/emit.rs` — the main node-dispatch function where `\input`
  is currently producing a `needs_manual_review` warning. Change the handler to recurse.
- `crates/bytetex-core/src/lib.rs` — the top-level `convert(source: &str)` signature
  currently has no knowledge of the filesystem. To support `\input`, a new entry point
  (or an additional parameter) must supply the base directory path so included files can
  be resolved. Example signature: `convert_file(path: &Path) -> ConversionResult`.
- `crates/bytetex-cli/src/main.rs` — the `convert` subcommand already receives the
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
bytetex emits the Typst `image()` call with the LaTeX dimension token verbatim:

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

**Relevant code:** `crates/bytetex-core/src/emit.rs` — locate the `\includegraphics`
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
(mixed delimiters, valid in math notation). bytetex emits `sin` as a bare identifier
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
   bytetex should map `\sin`, `\cos`, `\max`, etc. to their bare Typst equivalents. Check
   the existing math symbol table in `crates/bytetex-core/src/math.rs` (or equivalent)
   for any gaps in standard operator coverage.

2. **Mixed math delimiters `(a, b]` / `[a, b)`:** These are valid in math notation but
   Typst's math parser treats `[` and `]` as square brackets that must balance. The fix is
   to emit mixed-delimiter intervals as: `lr("(" + content + "]")` using Typst's `lr()`
   function which accepts unbalanced delimiter strings. Alternatively, escape as
   `\[` / `\]` in a raw string context.

**Relevant code:** `crates/bytetex-core/src/emit.rs` — math expression emitter;
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

Add to the math symbol table in `crates/bytetex-core/src/emit.rs` (or `math_symbols.rs`):
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

**Relevant code:** `crates/bytetex-core/src/emit.rs` — math expression walker.

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

In the command handler, treat `\(` as opening inline math and `\)` as closing it. If nested inside `$...$`, strip them. Relevant code: `crates/bytetex-core/src/emit.rs`.

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

`\newtheorem` should be classified as `needs_manual_review` and **dropped** (with a warning), not emitted. Relevant code: `crates/bytetex-core/src/emit.rs` — preamble command handler.

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
