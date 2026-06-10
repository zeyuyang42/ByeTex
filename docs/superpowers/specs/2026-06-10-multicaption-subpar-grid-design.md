# Multi-caption float splitting via `subpar.grid` — Design

**Status:** approved 2026-06-10
**Area:** `crates/byetex-core/src/emit/figures.rs` (+ `preamble.rs` / `emit.rs` for the conditional import)
**Corpus drivers:** 2605.22507, 2605.31063, 2605.31604

## Problem

`emit_figure` does a single tree-walk and captures **at most one** `\caption` and
**one** `\captionof`, one `\includegraphics`, and one nested tabular, then emits a
single `#figure(...)` with a single `caption:`. When a float holds multiple
captioned sub-blocks, every caption after the first is dropped — and often the
later body content too (only the first image/table is kept).

All affected corpus papers currently *compile* (the corpus is at 100%
ByeTeX-attributable compile-rate); this is a **fidelity / content-loss** gap, not
a compile blocker.

Two real structures appear in the corpus:

- **Pattern A — explicit sub-environments.** `\begin{subfigure}` / `\begin{subtable}`
  / `\begin{subcaptionblock}` panels, each with its own `\caption`+`\label`, plus
  an optional float-level main `\caption`+`\label`. (e.g. 2605.31604: one main
  caption + five `subtable`s.) Today these get partial handling: each panel renders
  as `figure(image(...), caption: [..])` inside a plain `grid(columns: 2, ...)` that
  becomes the body of the outer `#figure`. But the panel **labels are not attached
  to the panels** (they are pushed to the outer label set and only get hidden
  anchors), and there is **no sub-numbering** (1a/1b) — each inner `figure` carries
  its own independent counter.

- **Pattern B — top-level multi-caption.** Two or more `\captionof` / `\caption`
  sit directly in the float (each usually inside a `\begin{minipage}`), with no
  sub-environment wrapper. (e.g. 2605.22507: stacked tabulars + `\captionof{table}`
  then an image + `\captionof{figure}`; 2605.31063: side-by-side minipages, a table
  with `\captionof{table}` and an image with `\captionof{figure}`.) Today only the
  first caption survives and trailing content is lost.

## Goal

Emit a [`@preview/subpar`](https://typst.app/universe/package/subpar) grid — one
inner `figure(...)` per captioned sub-block, each with its own caption and its own
referenceable label and proper sub-numbering — with the float's main caption/label
on the grid. Single-caption floats remain **byte-identical** to today.

```typst
#subpar.grid(
  figure(image("a.png"), caption: [Left panel]), <fig:a>,
  figure(table(...), kind: table, caption: [Right panel]), <tab:b>,
  columns: (1fr, 1fr),
  caption: [Main caption],   // omitted when the float has no main caption
  label: <fig:main>,         // omitted when the float has no main label
)
```

## Design

### Detection

Inside `emit_figure`, after the existing discovery walk, classify the float:

1. **Pattern A** if `subfigures` (the already-collected `subfigure`/`subtable`/
   `subcaptionblock`/`subfloat` env list) has length ≥ 2, **or** length ≥ 1 with a
   float-level main caption. Each sub-env is one sub-block.
2. **Pattern B** if there is no sub-env but ≥ 2 caption sources (`\caption` /
   `\captionof`) reachable at the float level. Segment the float's direct children
   into sub-blocks: walking in source order, each caption command *closes* the run
   of content nodes seen since the previous caption (or the float start). The
   content run + that caption = one sub-block. When the content units are
   `\begin{minipage}` blocks, the minipage is the sub-block boundary (the caption
   inside it belongs to that minipage). A final caption/label that follows the last
   sub-block with no further content is the **parent** caption/label.
3. **Otherwise** (≤ 1 caption, no sub-envs): the existing single-figure path,
   unchanged.

### Sub-block → inner `figure(...)`

Refactor the existing body+caption+kind+label assembly (figures.rs ~386–439) into a
reusable helper:

```rust
/// Render one captioned block as a Typst `figure(...)` string (no leading `#`,
/// no trailing label). `kind` is the parent float's kind; `caption` / `label`
/// are this block's own.
fn emit_figure_inner(
    &mut self,
    body_str: &str,
    kind: Option<&str>,
    caption_text: Option<&str>,
) -> String
```

- The **single-caption** path becomes `self.out.push('#'); self.out.push_str(&inner); <label attach>` — must produce byte-identical output to today (verified by the existing figure snapshots).
- Each sub-block renders its content run through the normal body chain (image →
  `image("…")`, tabular → bare `table(...)`, subfigure panel → `render_subfigure_panel`)
  and its own caption via `emit_figure_inner`.

### Grid assembly (multi-caption path)

```
#subpar.grid(
  <inner_figure_1>, <label_1>,
  <inner_figure_2>, <label_2>,
  ...
  columns: <cols>,
  caption: [<main caption>],   // only if present
  label: <main label>,         // only if present
)
```

- `<label_i>`: the sub-block's picked referenced label (via `pick_label_to_attach`
  over that block's labels). A sub-block with no referenced label emits its inner
  figure with no trailing `<…>`. Extra referenced labels in a block reuse the
  existing `#hide[#figure([]) <key>]` anchor pattern (emitted after the grid).
- `<main caption>` / `<main label>`: the float-level caption/label not consumed by
  any sub-block.

### Column heuristic

Derive each sub-block's width fraction from its `minipage`/`subfigure`/`subtable`
optional `{0.41\textwidth}` (or `\linewidth` / `\columnwidth`) argument. Greedily
pack sub-blocks into rows whose cumulative width ≤ ~1.05; `columns` = the maximum
number of blocks in any row, emitted as `(1fr,) * cols` → `columns: (1fr, 1fr)`.
When no width fractions are present (pure stacked content, e.g. 2605.22507) →
`columns: 1`.

Worked examples:
- 0.41 + 0.58 minipages → one row of 2 → `columns: (1fr, 1fr)`.
- 0.32 × 3 subtables → one row of 3 → `columns: (1fr, 1fr, 1fr)`.
- 0.32 + 0.65 subtables → cumulative 0.97 ≤ 1.05, one row of 2 → 2 columns.
- stacked tabular + image, no widths → `columns: 1`.

### Kind

All panels in one grid inherit the **parent float's kind**: `table`/`table*` →
`kind: table`; `figure`/`figure*` → image default (no `kind:`). A `\captionof{type}`
on a sub-block still sets that block's kind when it differs (best-effort). The rare
mixed float (2605.22507: a table sub-block and a figure sub-block in one `figure`
env) renders both under the figure default — it compiles and shows both captions;
exact per-block kind fidelity there is out of scope.

### Conditional package import

- Add `used_subpar: bool` to `Emitter` (default false).
- Set it true whenever a `#subpar.grid(` is emitted.
- In `finish()` (document assembly), when `used_subpar` is set, prepend
  `#import "@preview/subpar:0.2.2"\n` **above** the neutral preamble. Single-caption
  documents keep the no-import, fully self-contained preamble (preamble.rs invariant
  preserved for everything that doesn't use the grid).
- Pin version `0.2.2` (validated against typst 0.14.2; downloads + caches; offline
  after first fetch).

## Testing (TDD)

**Regression guards (must stay green, no snapshot churn except intended):**
- `crates/byetex-core/tests/captionof.rs` — single `\captionof` → one `#figure`,
  `caption:`, `kind:`, label.
- `crates/byetex-core/tests/multi_label_figure.rs` — multi-label hidden anchors.
- All figure insta snapshots — single-caption output byte-identical.

**New tests (`crates/byetex-core/tests/multicaption_grid.rs`):**
- Pattern B, side-by-side: two `\captionof` minipages (0.41 / 0.58) → `#subpar.grid`
  with two `figure(...)`, both `<label>`s, `columns: (1fr, 1fr)`, no `caption:`.
- Pattern B, stacked: tabular + `\captionof{table}` then image + `\captionof{figure}`
  → `columns: 1`, both captions present, `kind: table` on the table block.
- Pattern A: main `\caption`+`\label` + three `subtable`s (0.32 each) → grid with
  `caption:` + `label:` on the grid, three inner figures with their own labels,
  `columns: (1fr, 1fr, 1fr)`.
- Import emitted exactly once when a grid is present; absent otherwise.
- Cross-reference: `\ref{sub}` in body → `@sub` (resolves to the sub-numbered panel).

**Corpus / gate:**
- `cargo test --workspace` green; `cargo clippy -p byetex-core --lib` clean.
- Acceptance gate stays **PASS 45 / BYETEX_FAIL 0** (2605.22507, 2605.31063,
  2605.31604 must still compile — now via subpar).
- `scripts/visual_test.py` from the worktree on 2605.22507 / 2605.31063: every
  caption now visible.

## Scope cuts (YAGNI)

Deferred to follow-ups: precise per-row width spans / unequal column tracks,
`\ContinuedFloat`, captions containing footnotes, and `subfig`-package legacy
`\subfloat` numbering quirks. First PR covers detection + grid emission + width-based
column packing + per-block labels + conditional import.
