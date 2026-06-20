---
name: byetex-beamer
description: How ByeTex converts a LaTeX beamer PRESENTATION (slides) to Typst — what it handles natively (so you don't re-implement it) and the few constructs you must fix by hand. Read this FIRST when the source is `\documentclass{beamer}`.
---

# Converting beamer presentations

When the source is `\documentclass{beamer}`, ByeTex already converts most of the deck
**natively** — do NOT rebuild slide helpers, blocks, columns, or overlay handling from
scratch. Run `byetex convert deck.tex`, look at the `.typ`, and only fix the small set of
constructs listed under *Not handled* below.

## What ByeTex does for you (don't re-implement)

| Beamer source | ByeTex output |
|---|---|
| `\documentclass{beamer}` | a slide page — **4:3 by default**, `[aspectratio=169]` → 16:9 |
| `\begin{frame}{Title}…\end{frame}` | one page per slide (`#pagebreak(weak: true)`), title in the theme color |
| `\frametitle{Title}` | the slide title (bold, theme color) |
| `\frame{X}` (command form) | a slide rendering `X`; `\frame{\titlepage}` → the title slide |
| `\begin{columns}` + `\begin{column}{0.5\textwidth}` | `#grid(columns: (0.5fr, …), …)` |
| `\begin{block}{T}` / `alertblock` / `exampleblock` | a titled `#block` (blue / red / green accent) |
| `\setbeamercolor` / `\usecolortheme` / `\definecolor` | frame-title color **detected** from the theme |
| `\pause`, `\only<>`, `\uncover<>`, `\onslide<>`, `\visible<>`, `\alt<>{a}{b}`, `\item<1->` | **overlays collapsed** — content shown once, spec stripped |
| `\title`/`\author`/`\institute`/`\date` + title slide | author + institute as plain centered lines |

**Overlays collapse — this is correct.** A static PDF can't animate, so each overlay step
is shown once in its final state. The output therefore has FEWER pages than the beamer
truth PDF (beamer emits one page per `\pause`/`<n->` step). **Do not "fix" the page count
by adding slides** — fewer pages is the intended behavior.

## Not handled — fix these by hand in the `.typ`

| Construct | Status | Manual fix |
|---|---|---|
| `\tableofcontents` | dropped (warns) | Rebuild the section list by hand: `#outline()` won't list beamer `\section`s (they're emitted as headings); write a small `#list[Section 1][Section 2]…` or a manual `#enum`. |
| `\subtitle{…}` | dropped | Add it under the title in the title block: `#text(size: 1.1em)[Subtitle]`. |
| `\alert{X}` color | text kept, red lost | If the red matters, wrap: `#text(fill: red)[X]`. |
| Footer / headline / navigation chrome (page N/M, nav bars, section dots) | not rendered | Cosmetic; add a `#set page(footer: …)` only if asked. |
| `\AtBeginSection` section-transition slides | not auto-generated | Add a manual slide per section if the deck relies on them. |

## Procedure

1. `byetex convert deck.tex` → `deck.typ` (+ `deck.warnings.json`).
2. Read `deck.warnings.json` — `\tableofcontents` and other drops appear there with
   `category.kind`. (If you're in a sandbox with only `diagnostics.json`, run
   `byetex convert` yourself to get the warnings.)
3. `typst compile deck.typ` to confirm it builds (it usually already does).
4. Open `deck.typ` and apply ONLY the *Not handled* fixes above — everything else is
   already converted. Verify against the source slide-by-slide.

## Rules

- NEVER migrate the whole deck to Touying/polylux — ByeTex's page-per-frame output is the
  supported path and needs no package import.
- NEVER re-implement blocks, columns, the title slide, or overlay handling — they're done.
- Page count < beamer truth is expected (overlay collapse), not a bug to repair.
