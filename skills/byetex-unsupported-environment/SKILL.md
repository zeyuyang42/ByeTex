---
name: byetex-unsupported-environment
description: Translate a LaTeX environment ByeTex doesn't recognise (beamer frame, theorem, lstlisting, minted, etc.) into Typst. Use when a warning has `category.kind == "unsupported_environment"`.
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
| `beamer frame`               | Migrate to Touying (Typst slides) or polylux                  |
| `appendix`                   | `#set heading(numbering: "A.1")` for the appendix region      |

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
