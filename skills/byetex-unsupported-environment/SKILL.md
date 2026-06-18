---
name: byetex-unsupported-environment
description: Translate a LaTeX environment ByeTex doesn't recognise (beamer frame, theorem, lstlisting, minted, tcolorbox/colored boxes, etc.) into Typst. Use when a warning has `category.kind == "unsupported_environment"`, OR a `needs_manual_review` float whose body is a custom environment (e.g. a `tcolorbox`) you must rebuild by hand.
---

# Translating an unsupported LaTeX environment

ByeTex supports a fixed set of environments (article body, itemize/enumerate/
description, math envs, tabular, figure). Anything outside this set emits an
`unsupported_environment` warning with the env name in `category.name`.

## Common translations

| LaTeX env                    | Typst equivalent                                              |
|------------------------------|---------------------------------------------------------------|
| `theorem` / `lemma` / `proof`| `#theorem(...)` from `@preview/ctheorems` package             |
| `lstlisting` / `minted`      | `#raw(lang: "rust", "code")` or fenced ``` ```lang ``` block  |
| `verbatim`                   | `#raw("text")` or a backtick raw block                        |
| `quote` / `quotation`        | `#quote[text]`                                                |
| `center`                     | `#align(center)[text]`                                        |
| `flushleft` / `flushright`   | `#align(left)[text]` / `#align(right)[text]`                  |
| `tabbing`                    | A custom table or `#stack(...)` layout                        |
| `tcolorbox` / `\tcbox`       | `#block(fill: …, stroke: …, …)[…]` — see the recipe below     |
| `mdframed` / `framed`        | `#block(stroke: …, inset: …, …)[…]` (same shape, no fill)     |
| `beamer frame`               | Migrate to Touying (Typst slides) or polylux                  |
| `appendix`                   | `#set heading(numbering: "A.1")` for the appendix region      |

## Recipe: `tcolorbox` (and other colored/framed boxes)

`tcolorbox` is a *very common* ML-paper package for framed, colored callout boxes
(often inside a `figure*` that ByeTex flags `needs_manual_review`). There is no
Typst package to import — rebuild it with a `#block`. Drop this helper near the top
of `main.typ` once, then call it per box:

```typst
// One reusable helper — paste once near the top of main.typ.
#let tcolorbox(title: none, fill: rgb("#eef3ff"), frame: rgb("#3366cc"), body) = block(
  fill: fill,
  stroke: 0.6pt + frame,
  radius: 2pt,
  width: 100%,
  inset: 0pt,
  breakable: true,
)[
  #if title != none {
    block(fill: frame, inset: (x: 8pt, y: 4pt), width: 100%, below: 0pt)[
      #text(fill: white, weight: "bold")[#title]
    ]
  }
  #block(inset: 8pt, width: 100%)[#body]
]
```

Then translate each `\begin{tcolorbox}[opts] … \end{tcolorbox}`:

```typst
// \begin{tcolorbox}[colback=blue!5, colframe=blue!50!black, title=Example]
#tcolorbox(title: [Example], fill: rgb("#eef3ff"), frame: rgb("#27408b"))[
  … the box body, itself converted to Typst …
]
```

Map the options:

| tcolorbox option        | Typst                                                       |
|-------------------------|-------------------------------------------------------------|
| `colback=<color>`       | `fill:` — a light tint (xcolor `blue!5` ≈ `rgb("#eef3ff")`) |
| `colframe=<color>`      | `frame:` — the border + title-bar colour                    |
| `title=<text>`          | `title: [<text>]`                                           |
| no `title`              | omit `title:` (the helper then renders no title bar)        |
| `sharp corners`         | `radius: 0pt`                                               |
| `boxrule=<len>`         | the `stroke` thickness (`0.6pt` default above)              |

xcolor mixes like `blue!5` (5% blue on white) have no exact Typst form — eyeball a
light `rgb("#…")` tint; the goal is a faithful *look*, not a pixel match. For the
inline `\tcbox{…}` form use `#box(fill: …, stroke: …, inset: 3pt)[…]`.

## Procedure

1. Read the warning's `snippet` to see the exact LaTeX source.
2. Decide whether the environment has a direct Typst analogue (above) or
   requires a package import (e.g. `ctheorems`, `Touying`).
3. Locate the placeholder in the `.typ` (use the warning's byte range).
4. Replace the placeholder with the translated Typst.
5. If a Typst package is needed, add `#import "@preview/<pkg>:<ver>"` near the
   top of the file.

## When no direct mapping exists

For exotic envs (e.g. chemfig, feynman, music notation), no Typst package
may exist. Options:

- Render the original LaTeX fragment to SVG/PDF and embed via `#image`.
- Drop the environment and add a `// TODO: re-implement <env>` comment.
- Ask the user — some envs are project-specific and need their input.

## Verification

`typst compile <file>.typ` must exit 0. Visually compare the rendered output
against the original LaTeX PDF where possible.
