---
name: byetex-figures-subpar
description: Handle figures and multi-caption floats emitted as `#subpar.grid(...)` (needs `@preview/subpar`), plus the `#figure`/`#image` path and image resolution. Use when the `.typ` contains `subpar.grid` or a figure/image fails to compile.
---

# byetex: figures & subpar grids

## Single figures

A LaTeX `figure`/`table` becomes a `#figure(...)`:

```typ
#figure(
  image("plot.pdf", width: 80%),
  caption: [A plot.],
) <fig:plot>
```

`kind: table` is set when the body is a tabular (so refs read "Table N"). A
missing image file → byetex emits a grey placeholder rect; swap in the real path
relative to the `.typ` (in `--project` mode assets are copied alongside `main.typ`,
so a bare filename resolves).

## Multi-caption floats → `#subpar.grid`

A float with **multiple captioned sub-blocks** (side-by-side `minipage`s, or
`\begin{subfigure}`/`\begin{subtable}`, or stacked content with two `\captionof`)
is emitted as a [`@preview/subpar`](https://typst.app/universe/package/subpar)
grid — one inner `figure(...)` per sub-block, each with its own caption + label and
real sub-numbering (1a, 1b):

```typ
#import "@preview/subpar:0.2.2"

#subpar.grid(
  figure(image("a.png"), caption: [Left]), <fig:a>,
  figure(table(...), kind: table, caption: [Right]), <tab:b>,
  columns: (1fr, 1fr),
  caption: [Overall caption.],
  label: <fig:main>,
)
```

## Common compile errors

- **`unknown variable: subpar`** — the `#import "@preview/subpar:0.2.2"` line is
  missing from the top of the file. Add it. (ByeTex adds it automatically when it
  emits a grid; only restore it if you deleted it.)
- **`cell's colspan would cause it to exceed the available column(s)`** — a table
  inside a panel has a `\multicolumn`/short-row issue; see `byetex-tables-layout`.
- **Package fetch fails offline** — `@preview/subpar` downloads on first use, then
  caches. If the environment is fully offline and uncached, replace the
  `#subpar.grid(...)` with a plain `#grid(columns: ..., figure(...), figure(...))`
  (you lose sub-numbering but it compiles with no package).

## Caption/label fidelity

Each sub-block keeps its own `<label>`, so `\ref`/`\cref` to a sub-figure resolves
to "Figure 1a". The grid's `columns:` is derived from the LaTeX minipage/subfigure
width fractions; adjust the tuple if the layout looks wrong.
