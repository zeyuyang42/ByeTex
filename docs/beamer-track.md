# Beamer (slides) support — track

User-chosen expansion direction (2026-06-19): support LaTeX `beamer` presentations.
Baseline before work: the `beamer` class was undetected and every `frame` dropped —
a deck rendered as only its title (all slides lost).

## Done

- **B1 — `columns` / `column`. ✅ DONE (PR #311, v0.4.9).** → `#grid(columns: (Nfr,…),
  gutter: 1em, [cell],…)`; `{width}` → `fr` ratio (leading-dot `.45`→`0.45` normalized).
- **B2 — `block`/`alertblock`/`exampleblock`. ✅ DONE (PR #313, v0.4.10).** → titled
  `#block` (accent-colored header + left rule; blue/red/green accents).

- **B0 — frame foundation (PR #309, v0.4.8).** `DocClass::Beamer` detection; each
  `frame` → one page per slide (weak `#pagebreak()`); `\begin{frame}{Title}{Subtitle}`
  + `\frametitle{…}` → bold title / regular subtitle. Probe deck 1 page → 4.

## Open (ranked)

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
