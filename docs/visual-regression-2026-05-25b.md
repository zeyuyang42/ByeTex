# Visual Regression — 2026-05-25 (second pass)

## Context

After PRs #38 (Bug #29 primes), #39 (Bug #26 matrix fusion), #40 (Bug #25 macro
expansion fusion), #41 (Bug #28 notempty), and #42 (Bug #27 bibliography
missing), the canonical 5-paper visual_test run shows:

| Stage | Count |
|---|---|
| convert_ok | 5/5 |
| typst_ok | **1/5** |
| structure_ok | 0/5 |

Corpus-wide, **7/26 papers compile cleanly** (up from 1/26 at session start).

This document catalogues the new tier of errors that remain (Bugs #30–#34).

## Per-paper status

### 2605.22507

**Original first error (FIXED):** `\multicolumn` body / chained-macro `dnu`
(Bugs #22, #25).

**New first error:** `unknown variable: primal` at line 567:5:

```typst
 quad quad thin cal(F)_(\#) mu_H &= nu_("tgt")
 <eq:primal-lp> $
```

The `\label{eq:primal-lp}` was attached to an `align` row but emitted *inside*
the math span (before the closing `$`). Typst math reads `<eq:primal-lp>` as
`<eq` `:` `primal` `-lp>` — `primal` is an unknown identifier.

**Bug #30:** math-environment labels emitted inside the closing `$` instead
of after it. The math-env emitter's `pending_math_label` flush attaches the
label correctly when the `\label{...}` is at the end of the env body, but
mid-body labels (e.g. attached to a single `\\`-separated row) leak inline.

---

### 2605.22557

**Original first error (FIXED):** `\\[2ex]` row-break (Bug #21), `Qmat`
fusion (Bug #26).

**New first error:** `unknown variable: lpha` at line 792:11:

```typst
[mat(0, 0\alpha^i_t, -alpha^i_t)], quad D^i_t=-1,])
```

The `0\alpha` has a stray backslash before `alpha` — comes from the matrix
cell containing a `\\` row-break that Bug #20's fix appended a `\n` after,
but the `\\` is INSIDE a nested matrix cell where its bytes shouldn't have
been intended as a row break of the outer matrix.

**Bug #31:** `\\` row-break inside a nested matrix cell leaks as a literal
`\` byte into Typst output. Probably needs the row-break handler to be
aware of nesting depth or to not append `\n` when inside an inner cell.

---

### 2605.22776

**Original first error (FIXED):** alphanumeric subscript (Bug #23),
missing-bib file (Bug #27).

**New first error:** `label <Wang-Li-Reddy-2019> does not exist`.

The paper bundles only `Stas.bib` but the body cites entries from the other
3 `.bib` files (which Bug #27 silently dropped). Without those entries
defined, every `\cite{key}` produces a "label does not exist" error.

**Bug #32:** post-Bug-#27 follow-up — when we drop a missing `.bib`, the
`\cite{key}` calls that referenced it should also drop (or warn-and-emit
plain text) rather than producing broken `@key` references. Requires
either BibTeX simulation (parse the surviving `.bib` to know which keys
are defined, drop refs to undefined keys) or accepting the limitation.
**Likely defer** — non-trivial.

---

### 2605.22159

**Original first error (FIXED):** `\ref` in math (Bug #24), `[^(]` leak
from `\notempty` (Bug #28).

**New first error (before this round):** `unknown variable: hj`.

The source `\{\genvarBdh[j]\}_{j=1}^{n_h}` expanded to `g_hj` because the
bare-letter subscript `_h` had no separator before the following `j`,
fusing into the unknown identifier `hj`.

**Bug #33 (FIXED in this report's PR):** Bare-letter subscript followed by
another letter token. Fix: the subscript emitter now drops a
`MATH_WORD_BOUNDARY` sentinel after a letter-ending bare subscript so
`collapse_math_spaces` inserts a separator when the next token is letter/
digit.

Post-fix the paper fails on the same Bug #32 class (citation labels not in
the bundled `.bib`).

---

### 2605.22820

**typst_ok: TRUE** (passes typst compile)

**structure_ok: FALSE**

- `word_jaccard: 0.71` ✓ (> 0.55 threshold)
- `word_recall: 0.79` ✓ (> 0.65 threshold)
- `heading_recall: 0.09` ✗ (< 0.60 threshold)
- `page_ratio: 0.64` ✗ (< 0.70 threshold) — typst output is 28 pages vs truth's 44

**Bug #34:** Even when papers compile, structural fidelity is poor. The
content (words) survives at 71% Jaccard / 79% recall — readable. But
headings barely match (9%) and the output is 35% shorter than the truth
PDF. This is a *fidelity* bug class, not a compile bug. Likely causes:
section heuristics mismatched, page-break behaviour different, or some
content (tables, figures) being silently dropped.

**Likely defer** — fidelity work is a separate track from compile-blockers.

## Open bugs table

| # | Description | Affected papers | Priority |
|---|---|---|---|
| 30 | math-env label emitted inside `$...$` instead of after | 2605.22507 | high (compile-blocker) |
| 31 | `\\` row-break inside nested matrix cell leaks `\` | 2605.22557 | high (compile-blocker) |
| 32 | post-#27: `\cite{key}` to undefined bib entries breaks compile | 2605.22776 + others | low — needs BibTeX simulation |
| 33 | bare-letter subscript fuses with next letter (`g_h` + `j` → `g_hj`) | 2605.22159 + likely others | **FIXED in this report's PR** |
| 34 | typst_ok papers have low heading_recall / shrunken page count | 2605.22820 | defer — fidelity, not compile |

## Recommended next steps

1. **Bug #30** — labels inside math spans. Likely a small fix to the math-env
   emitter; the existing `pending_math_label` flush just needs to also fire
   at math-row positions.
2. **Bug #31** — nested-matrix row-break leak. Needs investigation of the
   nesting condition; may revisit the Bug #20 `\n` append decision.
3. **Bug #34** structural fidelity — Once #30, #31 are out of the way and
   the typst_ok count is higher, fidelity becomes the main signal.
4. **Bug #32** — defer until BibTeX simulation is on the roadmap.
