# Visual Regression — 2026-05-25

## Context

After PRs #35 and #36 (which closed Bugs #16–#24 — the entire previous tier of
documented compile-blockers), all five canonical test papers still fail
`typst compile`, but on **different** errors than the 2026-05-24 report. This
document catalogues the new tier (Bugs #25–#29).

Run command: `uv run --with requests --with Pillow python scripts/visual_test.py`

| Stage | Count |
|---|---|
| convert_ok | 5/5 |
| typst_ok | **0/5** |
| structure_ok | 0/5 |

## Papers tested

### 2605.22507 — Generative Modeling by Value-Driven Transport

**Original first error (FIXED):** `\multicolumn` body now lands inside the
`table.cell(colspan: N)[...]` call (Bug #22).

**New first error:** `unknown variable: dnu` at line 483:

```typst
x||^2 dnu_("src")(x). $ <eq:monge>
```

The LaTeX source is `\dd\src` where the paper defines
`\newcommand{\dd}{\mathrm{d}}` and `\newcommand{\src}{\nu_{\text{src}}}`.
Direct conversion of just `$\dd\src$` produces the correct `"d"nu_("src")` —
so something about the *paper-scale* convergence (perhaps a different
prepass-time expansion or a sub-emitter losing the `\mathrm{d}` → `"d"`
mapping under chained `\newcommand` expansion) is producing bare `d` then
bare `nu` with no separator. Result: `dnu` reads as one identifier.

**Bug #25:** Chained user-macro expansion drops the upright-text wrapper on
single-letter `\mathrm{d}` somewhere downstream of `expand_user_macro`.

---

### 2605.22557 — SIAM Numerical Analysis paper

**Original first error (FIXED):** `\\[2ex]` row-break broken bracket (Bug #21).

**New first error:** `unknown variable: Qmat` at line 548:

```typst
= Qmat(tilde(F)_1(L(bold(v)))\
```

The output `Qmat(` is being parsed as a function call to an undefined
identifier `Qmat`. The source likely has `Q^{\mathrm{mat}}(...)` or a custom
`\Qmat` macro emitting fused letters.

**Bug #26:** Multi-letter math identifier emitted as a single fused token
without splitting / quote-wrapping. Same shape as #25 in spirit but for an
operator-call construction.

---

### 2605.22776 — SDPM survival analysis paper

**Original first error (FIXED):** alphanumeric subscript `_{i1}` (Bug #23).

**New first error:** `file not found: TabTrans.bib` at line 431:

```typst
#bibliography(("TabTrans.bib", "Survival_analysis.bib", "Surv_Attent.bib", "Stas.bib",), style: "ieee")
```

The paper bundles only `Stas.bib`; the other three `.bib` files referenced by
`\bibliography{...}` are not in the arXiv source. Typst aborts because at
least one path doesn't resolve.

**Bug #27:** `\bibliography{a,b,c,d}` should fall through to whatever subset
of files exist on disk. Either probe each path before emitting and silently
drop missing ones (with a warning), or wrap the whole call in a try/catch
equivalent. This is the bibliography track (deferred Bug #9 family) coming
back to bite.

---

### 2605.22159 — Generalized Spectral paper

**Original first error (FIXED):** `\ref` in math → `@eqn` ambiguity (Bug #24).

**New first error:** `unexpected hat` at line 352:

```typst
<= min_(g_h[^(]){}in B_h)||b-g_h[^(]){}||_(H^(-1/2)(Sigma))
```

The label key `g_h[^(]` is a fragment leaked into math output. Walking
backward: the `[^(]` shape is the byproduct of escape-passing on a label key
that contained `^` (caret). Our label / math-bracket escaping currently
handles `_` (Bug #16's fix) but not `^`. When the LaTeX source has a label
like `\label{g^h}` (or anything math-meta in the key), the `^` falls through
unescaped and Typst tries to parse it as a superscript operator.

Note: looking at the `g_h[^(]){}` pattern, this isn't a label key —
it's the *body* of a `\min_{...}` subscript that contains text with `^`
and `(` characters from a user macro expansion. The escape-pass is missing
for math subscript bodies, not labels.

**Bug #28:** Math-mode subscript bodies don't escape `^` / `(` / `)` when
they were intended as text content (i.e. came from expanding a macro
whose body had bracketed plain text). Probably a missed
`escape_unbalanced_math_brackets` call site OR the upstream macro emits
the wrong shape entirely.

---

### 2605.22820 — Log-Linear Rao-Blackwellization paper

**Original first error (FIXED):** subscript `_{i0}` (Bug #23).

**New first error:** `unknown variable: pair` at line 1280:

```typst
a_(i j)(h_("pair"))
B_i"(u_i)^(top) U^((i j))(h_("pair"))B_j(u_j). $
```

Visually the `"pair"` is in quotes so it should be a Typst string literal.
But Typst reports `pair` as unknown variable — which means in the column
where Typst points, the quotes aren't actually there (or the parser is
mid-string for some reason). Likely interaction between the `_(` subscript
and the bare `"` that opens — Typst may be reading the inside of `_("...")`
as math context.

Probably actually fine but needs investigation of column 12.

**Bug #29:** `_("...")` math-text-in-subscript loses string-literal status
in some contexts. Need to verify by reading the bytes around column 1280:12
of `main.typ` and comparing to LaTeX source.

---

## Open bugs table

| # | Description | Affected papers | Track |
|---|---|---|---|
| 25 | Chained `\newcommand` expansion drops `\mathrm{d}` → `"d"` wrapper | 2605.22507 | macro expansion |
| 26 | Multi-letter math identifier fused at operator boundary (`Qmat(`) | 2605.22557 | math word splitter |
| 27 | `\bibliography{a,b}` aborts when any one `.bib` is missing | 2605.22776 | bibliography (Bug #9 family) |
| 28 | `^` and parens leak into math subscript body (label-escape gap or macro body) | 2605.22159 | math escape |
| 29 | `_("pair")` math-text-in-subscript loses string-literal status | 2605.22820 | math subscript |

All five bugs are distinct from anything in Bugs #1–#24. Bugs #25, #26, #29
are math-emit edge cases; Bug #27 is the long-deferred bibliography track;
Bug #28 is a math escape gap.

## Recommended sequencing

1. **Bug #29** first — likely the smallest scope (just check whether the
   string literal survives, may already be working in many cases).
2. **Bug #26** — multi-letter identifier splitting at operator boundaries.
   Diagnostic first; the cause may overlap with Bug #25.
3. **Bug #25** — chained macro expansion drops wrappers. Trickier; needs
   tracing through `expand_user_macro` and `render_in_sub_emitter`.
4. **Bug #27** — bibliography fallback. Touches CLI/asset-resolution paths.
5. **Bug #28** — math escape gap. Need to reproduce minimally first.

Each bug should land with a red test in `crates/byetex-core/tests/golden_m*.rs`.
