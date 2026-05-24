# Visual Regression — 2026-05-24

## Context

Five compile-blocker bugs were fixed in `crates/bytetex-core/src/emit.rs`
(see commit message). After the fixes, the five target arXiv papers were re-run
via `bytetex convert --project` + `typst compile`. The original first errors
are gone; the papers now fail on newly-revealed second-order issues. Per the
policy in the fix plan, new issues are catalogued here rather than fixed in the
same pass.

---

## Papers tested

### 2605.22507 — Generative Modeling by Value-Driven Transport

**Original blocker (FIXED):** `\tag{Dual LP}` emitted `$ "tag" …

` inside equation.

**New first error:** `expected comma` at line 1565:

```typst
table.cell(colspan: 4)[_Path energy_]
```

`table.cell(colspan: N)` is not valid Typst syntax without a trailing comma or
closing paren. The converter is emitting cell attributes without proper Typst
table-cell notation. Root cause: the `\multicolumn{4}{...}` handler produces
`table.cell(colspan: 4)` without a body argument — the body `[...]` immediately
after is not attached. Fix: the multicolumn emitter must produce
`table.cell(colspan: 4)[body]` in one expression.

---

### 2605.22557 — (SINUM paper)

**Original blocker (FIXED):** unclosed delimiter from nested math environment.

**Remaining error (pre-existing Bug #20):** `unknown variable: ex` from the
`\\[2ex]` row-break-with-length command inside a math align environment:

```typst
[[2ex]
```

The `\\[2ex]` optional-length argument is emitted verbatim as `[[2ex]` which
Typst parses as two nested arrays and an unknown variable `ex`. This is tracked
as Bug #20 with an existing ignored red test
(`m3_align_row_break_strips_optional_length`). The fix must strip the length
argument from `\\[length]` inside math.

---

### 2605.22776 — SDPM survival analysis paper

**Original blocker (FIXED):** `unknown variable: dt` from letter fusion.

**New first error:** `unknown variable: i1` at line 145:

```typst
w_(i1)x_i
```

The LaTeX source has `w_{i1}` (subscript `i1`) — two math atoms `i` and `1`
in LaTeX. The converter produces `w_(i1)` and Typst reads `i1` as a single
alphanumeric identifier, which is not defined. Fix: inside subscript/superscript
groups, alphanumeric sequences containing digits (like `i1`) should be broken
into separate atoms: `w_(i 1)`.

---

### 2605.22159 — Generalized Spectral paper

**Original blocker (FIXED):** `\diamT` custom macro emitted as raw `\diamT`
backslash text.

**New first error:** `unknown variable: eqn` at line 741:

```typst
(@eqn:AMPa)
```

A `\ref{eqn:AMPa}` (or `\eqref`) inside a displayed math formula is producing
`(@eqn:AMPa)`. In Typst math mode, `@label` is a cross-reference. Typst
apparently sees `eqn` as an identifier here rather than parsing `@eqn:AMPa` as
a reference tag. Root cause: the reference emitter may be producing the `@ref`
syntax inside math where Typst's parser interprets it differently than in text
mode.

---

### 2605.22820 — Log-Linear Rao-Blackwellization paper

**Original blocker (FIXED):** `arrow.r(0,infinity)` parsed as a function call.

**New first error:** `unknown variable: i0` at line 774:

```typst
epsilon.alt_i(log p_i-log p_(i0))
```

Same root cause as 2605.22776: `_{i0}` in LaTeX produces `_(i0)` in Typst.
Typst evaluates `i0` as an identifier — unknown. Fix is the same: break
digit-adjacent alphanumeric subscript content into separate atoms.

---

## Summary of open bugs

| # | Description | Affected papers | Fix target |
|---|---|---|---|
| 21 | `\\[length]` in math emits `[[len]` | 2605.22557 | extend Bug #20 fix |
| 22 | `\multicolumn{N}` → `table.cell(colspan: N)` missing body | 2605.22507 | table emitter |
| 23 | Alphanumeric subscripts (`i0`, `i1`) → Typst identifier lookup fails | 2605.22776, 2605.22820 | subscript group emitter |
| 24 | `\ref{label}` / `\eqref{label}` inside math → `@label` parse ambiguity | 2605.22159 | ref emitter in math mode |

Bug #21 is a variant of existing Bug #20. Bugs #22-#24 are newly surfaced.

All bugs listed in `visual-regression-2026-05-23.md` (Bugs #1-#20) are
unchanged: #1-#15 were fixed in a prior pass, #16-#20 have red tests and remain
pending.
