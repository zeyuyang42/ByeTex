# Beamer → touying: verified design (Phase 3)

The plan adopts the Typst **touying** slide framework for beamer decks (user-chosen), to get
real overlays, section slides, and header/footer **chrome** — none of which the current
plain-Typst beamer path produces. This note records the *verified* design so the emitter
implementation works against a proven target. The hand-authored target deck is
`docs/beamer-touying-target.typ` (compiles + renders faithfully).

## Verified environment (probed 2026-06-23, typst 0.14.2)
- **Pin `@preview/touying:0.7.3`** — compiles under typst 0.14.2; downloads + caches on first
  `typst compile` (the acceptance gate resolves it from the package cache after a one-time fetch).
  0.5.3/0.6.1 also download but the simple-theme `title-slide` API differs; 0.7.3 is the target.
- **Theme: `metropolis`** (`#import themes.metropolis: *`). Gives a dark header bar with the
  frame title, a footer with the slide number, and an accent progress line — the chrome the
  truth beamer has. (university/dewdrop also compile; metropolis is the closest beamer feel.)

## Mapping (beamer → touying)
| beamer | touying |
|---|---|
| `\documentclass[aspectratio=169]{beamer}` + `\title/\subtitle/\author/\institute/\date` | `#show: metropolis-theme.with(aspect-ratio: "16-9", config-info(title:, subtitle:, author:, institution:, date:))` |
| `\frame{\titlepage}` / `\maketitle` | `#title-slide()` |
| `\begin{frame}{T} … \end{frame}` | `== T` + content (touying makes each `==` a slide with the header bar) |
| `\section{X}` | `= X` (touying renders a section divider slide) |
| `\tableofcontents` | `== Outline` + `#outline(title: none, indent: 1em)` |
| `\begin{columns}{\begin{column}…}` | `#cols[ … ][ … ]` |
| `\begin{block}{T}…` / `exampleblock` / `alertblock` | a titled `#block(...)` (stroke/fill per kind) |
| `\alert{x}` | `#text(fill: red)[x]` |
| theme colors (`\usecolortheme`, `\setbeamercolor`) | map onto the touying theme's `config-colors`/`primary` (refinement) |

## Phasing
- **3a (scaffolding, NEXT):** emit the import + theme + `config-info`, `\titlepage`→`#title-slide()`,
  frames→`==` slides, `\section`→`=`, `\tableofcontents`→outline, columns/blocks/alert. Overlays
  **collapsed to final state** (current behavior) for now. Re-bless all beamer snapshot tests
  (they all churn — expected). Verify vs the beamer-demo truth.
- **3b:** section-divider slides gated on `\AtBeginSection` (touying section dividers are
  automatic for `=`; suppress if the deck has no `\AtBeginSection`), header/footer chrome tuning,
  theme-color mapping.
- **3c (overlays):** `\pause`→`#pause`; `\item<n->`→incremental reveal. **GOTCHA (verified):**
  `#only("1-")[…]` *inside a list item / context* panics ("Unsupported mark touying-fn-wrapper");
  use `#pause` between items or the callback-style `#slide(repeat: n, self => …)` with
  `utils.methods(self)`. So overlays need the callback form, not inline `#only`, inside lists.

## Acceptance-gate note
The gate runs `typst compile --no-pdf-tags main.typ`. touying multi-column/PDF-tags already
needed `--no-pdf-tags` (the harness sets it). Confirm the package cache is warm in CI or the
first compile fetches it.
