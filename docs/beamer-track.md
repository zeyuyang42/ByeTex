# Beamer (slides) support — track

User-chosen expansion direction (2026-06-19): support LaTeX `beamer` presentations.
Baseline before work: the `beamer` class was undetected and every `frame` dropped —
a deck rendered as only its title (all slides lost).

## Done

- **B0 — frame foundation (PR #309, v0.4.8).** `DocClass::Beamer` detection; each
  `frame` → one page per slide (weak `#pagebreak()`); `\begin{frame}{Title}{Subtitle}`
  + `\frametitle{…}` → bold title / regular subtitle. Probe deck 1 page → 4.

## Open (ranked)

- **B1 — `columns` / `column`.** Beamer's two-column slide layout is dropped
  (`unsupported_environment`) → column CONTENT is lost. Map to a Typst `grid`/`#columns`.
  High value (very common). The `{width}` arg of `\begin{column}{0.5\textwidth}` →
  column ratio.
- **B2 — `block` / `alertblock` / `exampleblock`.** Titled callout boxes, dropped →
  content lost. Map to a titled `#block(...)` (reuse the tcolorbox recipe shape).
- **B3 — title slide.** `\frame{\titlepage}` (a `\frame` COMMAND with `\titlepage`
  arg) and bare `\titlepage` → render the title block as the first slide. Currently
  `\frame` is `unsupported_command`; `\titlepage` does nothing.
- **B4 — presentation page geometry.** Beamer slides are 4:3/16:9 landscape, larger
  base font, no justification. Give `DocClass::Beamer` a presentation `StyleProfile` +
  `#set page(paper: "presentation-16-9")`.
- **B5 — overlays.** `\item<1->`, `\onslide`, `\pause`, `\only<>`/`\uncover<>` —
  overlay specs. MVP: drop the overlay spec, show all content (no animation in a PDF).
  Currently `<1->` may leak. Verify.
- **B6 — `\section`/`\subsection` + `\tableofcontents`.** Section frames / TOC nav.
- **B7 — corpus + fidelity.** Add real beamer decks to the corpus with slide-aware
  visual fidelity testing (page count, per-slide word recall).

## Notes / gotchas

- `frame` env + `\frametitle` are GATED on `DocClass::Beamer` (non-beamer `frame`
  untouched). A frame expanded inside a macro runs in a sub-emitter whose
  `detected_class` is `Unknown`, so it would NOT be slide-styled — thread the class in
  if macro-wrapped frames matter (B-follow-up).
- Title detection: only leading curly groups on the SAME LINE as `\begin{frame}`.
