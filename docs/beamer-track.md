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
  `\alt<spec>{a}{b}` → shows default, drops spec + alt (PR #325, v0.5.4). Overlays COMPLETE.
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

## Round-4 dogfood (2026-06-20, v0.5.4) — agent-surface verification

Fresh Sonnet agent repaired a realistic Madrid 16:9 deck (`corpus/beamer-demo`, seed
compiled, fidelity 0.867). **Verdict: the CONVERTER handles beamer well, but the AGENT
SURFACE is silent about it** — the agent reinvented helpers ByeTex already provides
(blocks, columns, `\alert`) because no skill documents ByeTex's native beamer support,
and couldn't see what was dropped (no warnings.json in the sandbox).

### Top findings (round-4 backlog)
- **R1 (P0, BLOCKER) — no beamer skill.** `byetex-unsupported-environment` says only
  "beamer frame → Touying or polylux" with no recipe. Need a `byetex-beamer` skill
  documenting what ByeTex DOES natively (frames→pages, `columns`→grid, blocks→titled
  `#block`, overlays→collapsed, theme colors→detected, title slide auto) + the known gaps,
  so agents stop reinventing and target the real gaps.
- **R2 ✅ FIXED (PR #339, v0.5.12) — dogfood/diagnose sandbox lacks `warnings.json`.** `diagnose --project`
  (used by dogfood prepare) writes `diagnostics.json` (compile errors) but not
  `warnings.json` (dropped-construct signals), so agents can't see silent drops.
- **B-toc (P1, converter) — `\tableofcontents` dropped** (warns): the Outline frame shows
  only its title, no section list. Emit a section list / `#outline()`.
- **B-subtitle (P1, converter) — `\subtitle{…}` dropped** from the title slide. Render it.
- **B-alert-color (P2) — `\alert` content renders but loses the red color** (agent
  misreported as "dropped"; content IS present).
- Page count (5 vs 8 truth) is the intended OVERLAY COLLAPSE (each `\item<n->`/`\pause`
  step is a separate page in beamer; ByeTex shows the final state) — NOT a bug; the agent
  over-corrected by adding pages. Document this so agents/graders don't treat it as a gap.
