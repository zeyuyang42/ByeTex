---
name: bytetex-tikz-to-typst
description: Convert a TikZ/pgfplots picture from LaTeX into Typst using CeTZ or a manual rewrite. Use when a warning has `category.kind == "tikz"` or `unsupported_environment` for `tikzpicture`/`pgfplots`.
---

# Converting TikZ to Typst

Typst's analogue for TikZ is **CeTZ** (https://github.com/cetz-package/cetz),
a community Typst package for vector drawing. ByeTex does not translate TikZ
automatically because the syntaxes diverge widely; this skill walks you
through a manual rewrite.

## Inputs

- The raw `\begin{tikzpicture}...\end{tikzpicture}` source from the
  `snippet` field of the warning.
- The byte range so you can locate the placeholder in the `.typ` file.

## Procedure

1. Add the CeTZ import at the top of the `.typ`:

   ```typst
   #import "@preview/cetz:0.4.2": canvas, draw
   ```

2. Identify the TikZ primitives in the source:

   - `\draw (a) -- (b)` → `draw.line((a), (b))`
   - `\draw[->] (a) -- (b)` → `draw.line((a), (b), mark: (end: ">"))`
   - `\node at (x,y) {label}` → `draw.content((x, y), [label])`
   - `\fill[red] (0,0) circle (1)` → `draw.circle((0, 0), radius: 1, fill: red)`

3. Wrap the converted primitives in a `canvas`:

   ```typst
   #canvas({
     import draw: *
     line((0, 0), (1, 1))
     circle((0.5, 0.5), radius: 0.3)
   })
   ```

4. Replace the ByeTex placeholder in the `.typ` (the `#text(red)[...]`
   marker, or the absent region) with the rewritten `#canvas { ... }` block.

5. Re-run `typst compile <file>.typ` and visually compare against the
   original PDF (if available) for parity.

## When TikZ has too many primitives

If the TikZ picture uses pgfplots, tikz-3dplot, or other heavy libraries,
CeTZ may not cover everything. In that case:

- For plots, consider `#import "@preview/cetz-plot"` or the `plotst` package.
- For complex 3D diagrams, render the original TikZ to a standalone PDF/SVG
  using LaTeX, then `#image("diagram.svg", width: ...)` in Typst.

## Verification

After every edit, run `typst compile <file>.typ` and confirm exit 0 and no
`error:` lines in stderr.
