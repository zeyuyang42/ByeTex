---
name: byetex-book
description: How ByeTex converts a LaTeX BOOK/REPORT/THESIS (chapter-bearing classes) to Typst — what it handles natively (so you don't re-implement it) and the few constructs to fix by hand. Read this FIRST when the source is `\documentclass{book}`/`report`/`memoir` or a thesis/dissertation class.
---

# Converting books, reports, and theses

When the source is a chapter-bearing class — `\documentclass{book}`/`report`/`memoir`,
the KOMA `scrbook`/`scrreprt`, or a custom thesis/dissertation class (e.g.
`tudelft-report`) — ByeTex already converts most of the structure **natively**. Do NOT
rebuild the table of contents, page numbering, heading hierarchy, or long tables by hand.
Run `byetex convert thesis.tex`, read the `.typ`, and only fix the small set below.

## What ByeTex does for you (don't re-implement)

| Book/report source | ByeTex output |
|---|---|
| `\documentclass{book}`/`report`/thesis | chapter-bearing layout — `\section` nests UNDER `\chapter` |
| `\chapter{T}` / `\section{T}` / `\subsection{T}` | headings at level 1 / **2** / 3 (not flattened) |
| `\section*{T}` (starred) | unnumbered heading at the correct level |
| `\tableofcontents` | `#outline(depth: 3)` — lists chapters/sections/subsections |
| `\frontmatter` | `#set page(numbering: "i")` — roman page numbers |
| `\mainmatter` | `#set page(numbering: "1")` + page-counter reset to 1 |
| `\title`/`\subtitle`/`\author` + `\maketitle` | a centered title block (title, subtitle, author) |
| `\coverimage{fig}` + `\makecover` (thesis/report) | a generic cover page — full-page `#image(fit: "cover")` + an overlaid title banner (title / subtitle / `\subject` / author); the cover image is copied into the output |
| `\begin{longtable}{…}` / `xltabular` | a Typst `#table` (page-break markers dropped) |
| `\begin{tabular}` / `\multicolumn` / `\multirow` | Typst tables (same as papers) |

## Not handled — fix these by hand in the `.typ`

| Construct | Status | Manual fix |
|---|---|---|
| `\backmatter` | dropped (warns) | Only un-numbers chapters; if it matters, set `#set heading(numbering: none)` from that point. Page numbering is unaffected. |
| Author block on a thesis title page | rendered article-style (superscript affiliation refs) | A thesis title page usually wants title + subtitle + a plain centered author/institute. Rewrite the title block without the `#super[1]` affiliation markers if it looks wrong. |
| Cover-page exact design (`\coverimage`/`\makecover`) | a GENERIC cover (image + dark banner) is emitted, but the class's bespoke art is NOT replicated | ByeTex approximates: it does not reproduce the exact banner colours/fonts, the institution logo, or the rotated affiliation. If you need the class-faithful cover, tweak the emitted `#page(margin: 0pt)[…]` block — adjust the banner `fill`/`#text` colours, add a `#place(bottom + left)[#image("logo.…")]` for the logo, etc. |
| `\listoffigures` / `\listoftables` | dropped (warns) | Add `#outline(target: figure.where(kind: image))` / `…kind: table)` if you need them. |
| `\printnomenclature` / `\printglossary` | dropped (warns) | Rebuild the list by hand (usually a `#table` of symbols). |
| `\chapter*{T}` frontmatter chapters (Preface, etc.) | numbered like a normal heading | Add `numbering: none` if it should be unnumbered (front-matter chapters usually are). |

## Procedure

1. `byetex convert thesis.tex` → `thesis.typ` (+ `thesis.warnings.json`).
2. Read `thesis.warnings.json` — `\backmatter`, lists, glossaries appear there. The
   structural ones above are ALREADY converted; only the *Not handled* rows need work.
3. `typst compile thesis.typ` to confirm it builds.
4. Apply ONLY the *Not handled* fixes; verify the title page and chapter/section nesting
   against the source.

## Rules

- NEVER re-implement the ToC, page numbering, heading levels, or long tables — they're done.
- A multi-file thesis (`\input{chapters/…}`) is flattened automatically; content is preserved.
- If the source is a `beamer` PRESENTATION, not a book, use `byetex-beamer` instead.
