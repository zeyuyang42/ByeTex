---
name: byetex-tables-layout
description: Table fidelity (booktabs rules, colspan/rowspan, over-/under-declared columns, brace-wrapped cells) and page-layout notes (two-column, density). Use when a `#table(...)` won't compile or renders wrong, or the overall page layout differs from the LaTeX.
---

# byetex: tables & layout

## Tables

A LaTeX `tabular`/`array`/`tabularx`/`tblr` becomes a Typst `#table(...)`:

```typ
#table(
  columns: 3,
  align: (left, center, right),
  stroke: none,
  table.hline(stroke: 0.08em),
  [Method], [Acc], [Time],
  table.hline(stroke: 0.05em),
  [Ours], [0.91], [12s],
  table.hline(stroke: 0.08em),
)
```

Booktabs rules (`\toprule`/`\midrule`/`\bottomrule`) map to `table.hline` with
`stroke: none` on the table (no vertical lines), matching LaTeX's look.

### Common table errors

- **`cell's colspan would cause it to exceed the available column(s)`** — a
  `\multicolumn{N}` spans more columns than remain on its row, usually because a
  short row (fewer cells than `columns`) shifted placement. ByeTex pads short rows,
  but if you've hand-edited, ensure every logical row fills `columns` slots; insert
  empty `[]` cells, or fix the `colspan:`/`rowspan:` on `table.cell(...)`.
- **`unexpected argument` after a cell** — a `#hide[...]`/`#box[...]` chained onto a
  following `[...]`; wrap the construct in parens so it's self-delimiting.
- **Raw LaTeX leaking in a cell** (`\textbf`, `\small`, `±`) — a brace-wrapped cell
  `{...}` wasn't converted; replace its content with the Typst equivalent
  (`*bold*`, drop font-size switches, `plus.minus`).

## Page layout

- **Two-column** classes render the body wrapped in `#columns(2)[...]`; the title
  block stays full-width. If a wide figure/table overflows a column, wrap it in
  `#place(...)` or move it out of the columns.
- **Density** — ByeTex emits a 10pt default with indent-only paragraph spacing to
  match LaTeX's compactness. If your page count diverges a lot, check `#set par(...)`
  / `#set text(size: ...)` near the top of the `.typ`.

These are fidelity tweaks — they don't usually block compilation. Prioritise a
clean `typst compile` first, then refine layout.
