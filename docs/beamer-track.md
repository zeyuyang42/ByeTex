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

- **B3 — title slide / `\frame{…}` cmd. ✅ DONE (PR #315, v0.4.11).** `\frame{X}`
  → slide; `\frame{\titlepage}`/`\titlepage` → auto-emitted title (no blank slide).
- **B4 — presentation page geometry. ✅ DONE (PR #317, v0.4.12).** Beamer →
  `presentation-16-9` landscape page, 22pt font, tight margins, ragged-right.
- **B5 — overlays. ✅ DONE (PR #319, v0.5.0).** `\pause`/`\only`/`\uncover`/
  `\onslide`/`\visible`/`\action`/`\alert` + `\item<spec>`: strip `<…>`, show content.
  KNOWN GAP: `\alt<spec>{a}{b}` (two-arg) still leaks spec + duplicates — follow-up.
- **B6 — `\section`/`\subsection` + `\tableofcontents`.** Section frames / TOC nav.
- **B7 — corpus + fidelity.** Add real beamer decks to the corpus with slide-aware
  visual fidelity testing (page count, per-slide word recall).

## Notes / gotchas

- `frame` env + `\frametitle` are GATED on `DocClass::Beamer` (non-beamer `frame`
  untouched). A frame expanded inside a macro runs in a sub-emitter whose
  `detected_class` is `Unknown`, so it would NOT be slide-styled — thread the class in
  if macro-wrapped frames matter (B-follow-up).
- Title detection: only leading curly groups on the SAME LINE as `\begin{frame}`.

## B7 measurement (2026-06-19, v0.5.1)

Rendered a realistic deck (title + outline + columns/block/math + overlays) via tectonic
(truth) vs byetex. **Result: strong content fidelity** — 4/4 pages, 0 warnings, columns +
block + math + bullets + title all faithful; overlay specs stripped cleanly. Fixed the top
gap: aspect ratio (default 4:3 + honor `aspectratio=169`, PR #321).

Remaining visual gaps (ranked, low): frame-title COLOR (byetex black-bold vs beamer theme
blue); title-slide affiliation rendered as superscript-footnote (vs plain centered line).
Both aesthetic, not content. A full corpus-ingest + automated slide fidelity harness is
deferred (manual render-compare suffices for now).
