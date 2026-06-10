---
name: byetex-math
description: Resolve math conversion gaps — missing symbols/operators, `#text(red)[\foo]` placeholders, custom operators via `op(...)`, matrices via `mat(...)`. Use when a warning has `category.kind == "ambiguous_math"` or an unknown `\foo` appears in a math zone.
---

# byetex: math gaps

ByeTex converts ~450 math symbols/operators deterministically. A math command it
doesn't recognise becomes an **`ambiguous_math`** warning and is emitted as a red
placeholder in the `.typ`:

```typ
$ ... #text(red)[\foo] ... $
```

Typst then fails (e.g. `unknown variable: foo`). Replace each placeholder with the
Typst equivalent.

## Find the Typst symbol

Most LaTeX names map to a Typst symbol of the same or similar name (no backslash):
`\alpha`→`alpha`, `\Rightarrow`→`arrow.r.double`, `\leq`→`<=`, `\partial`→`diff`,
`\nabla`→`nabla`, `\infty`→`infinity` (or `oo`). Search the Typst symbol list
(`https://typst.app/docs/reference/symbols/`) for the concept. Multi-letter
identifiers are variables in Typst, so wrap upright text with `upright("...")` or
`"..."`.

## Custom operators

`\operatorname{Foo}` / `\DeclareMathOperator{\Foo}{Foo}` / `\mathrm{Foo}` used as
an operator → Typst `op("Foo")` (add `limits: true` for sum-like
under/over-script placement):

```typ
op("argmax", limits: true)_x f(x)
```

## Matrices, cases, stacks

- `\begin{pmatrix}…\end{pmatrix}` → `mat(delim: "(", a, b; c, d)` (delims:
  `(`,`[`,`|`,`||`,`{`,`none`).
- `\begin{cases}…\end{cases}` → `cases(a & "if" x, b & "otherwise")`.
- `\substack{a\\b}` / stacked limits → `stack(a, b)` or `attach`.
- `\overset{a}{b}` / `\stackrel{a}{b}` → `attach(b, t: a)`.

## Unknown user macro in math

If the red token is a user `\newcommand` (not a standard symbol), translate the
macro instead — see `byetex-custom-macros`. After every replacement, re-run
`typst compile` to confirm the math zone parses.
