# Changelog

Notable changes to ByeTex. Format loosely follows
[Keep a Changelog](https://keepachangelog.com); versions follow semver.

## [0.6.4] — unreleased

### Added
- **Chapter-per-page density for book/report/thesis classes.** In a chapter-bearing class
  (`book`/`report`/thesis) every `\chapter` (and `\chapter*`) issues a `\clearpage` in
  LaTeX, so each chapter starts on a fresh page. ByeTex previously emitted chapter headings
  with no page break, so chapters packed together and converted theses ran roughly half the
  page count of the truth (the tudelft thesis was ~6 pages vs the truth's 12). ByeTex now
  emits a `#pagebreak(weak: true)` before each top-level (level-1 = `\part`/`\chapter`)
  heading. Applies to both numbered `\chapter{…}` and starred `\chapter*{…}` (frontmatter
  Preface / Summary / Nomenclature). `weak: true` collapses the break against an existing
  one (the cover page, the titlepage isolation, a `\frontmatter`/`\mainmatter` numbering
  switch), so the first chapter never leaves a blank page. Gated on chapter-bearing classes
  only — the article family keeps `\section`s inline (`\section`/`\subsection`, level ≥ 2,
  never break). The tudelft thesis now renders 10 pages (was 6).

- **Generic thesis/report cover page for `\coverimage` + `\makecover`** (Phase 4). Thesis
  and report classes (e.g. `tudelft-report`) define a designed cover page — a
  near-full-bleed cover image plus a banner carrying the title / subtitle / subject /
  author — that ByeTex previously dropped entirely (the directives `\coverimage`,
  `\makecover`, `\subject` were unhandled). ByeTex now detects `\coverimage{path}` (the
  asset is resolved and copied into the project output via the existing AssetRef
  plumbing) and, on `\makecover`, emits a generic cover page as the document's first
  page: a full-page `#image(..., fit: "cover")` with an overlaid dark title banner. Gated
  on chapter-bearing classes so articles/papers are unaffected; degrades gracefully to a
  banner-only page when the cover image is missing. The bespoke per-class art (logo, exact
  banner colours/fonts) is approximated, not replicated. The tudelft thesis now renders a
  cover page that was previously dropped.
- **Beamer overlays become touying incremental builds** (Phase 3c). Previously every
  overlay collapsed to its final state (everything shown at once); now beamer build
  specs drive sub-slide reveals: `\pause` → `#pause`; a sequential
  `\item<1->`/`<2->`/`<3->` list reveals one item per sub-slide (a `#pause` is injected
  between the spec-bearing items); a slide-top-level `\only<n>{X}` → `#only("n")[X]` and
  `\uncover`/`\onslide`/`\visible<n->{X}` → `#uncover("n-")[X]`. Reveals work inside the
  native `#grid`/`#block` that columns/blocks emit; only a reveal nested inside another
  reveal (which touying panics on, "Unsupported mark `touying-fn-wrapper`") is rendered
  collapsed. Incremental (`<+->`) and multi-segment (`<1,3>`) specs are also rendered
  collapsed for now. All gated on the beamer document class; non-beamer overlay handling
  is unchanged. The beamer-demo deck grows from 5 to 9 pages as the three-item
  "Why Scaling Laws?" frame and the `\only<2>` of "Loss Model" expand into build steps.

### Changed
- **Beamer theme colors map onto touying, and section-divider slides are gated**
  (Phase 3b refinement of the touying conversion). A detected beamer "structure" color
  — from `\setbeamercolor{frametitle|structure}{fg=…}` or `\usecolortheme{name}`
  (`beaver`/`crane`/`default`/…) — is now mapped to the metropolis accent via
  `config-colors(primary: …)`, so the header progress line / accent matches the deck's
  theme instead of metropolis's default orange. Detect-don't-hardcode: no override is
  emitted when the source defines no theme color. Section-divider slides are now gated on
  the deck actually installing one: a `\section` produces a standalone divider slide only
  when the preamble has `\AtBeginSection` (or `\setbeamertemplate{section page}`), matching
  real beamer; otherwise the section becomes a navigation-only heading
  (`= X <touying:hidden>`) that no longer renders a spurious extra page. The beamer-demo
  deck drops from 8 pages (title + outline + 3 dividers + 3 content) to 5 (title + outline
  + 3 content), matching the LaTeX truth.

### Added
- **Beamer decks now emit native Typst `touying` slides** (Phase 3a). A `beamer`
  document is converted to a `touying` presentation with the `metropolis` theme — a real
  slide framework with a dark header bar carrying the frame title, a footer with the slide
  number, and the accent progress line — instead of the old hand-rolled
  `#set page(paper: "presentation-…")` plain-Typst slides. Mapping:
  `\documentclass[aspectratio=…]{beamer}` + `\title`/`\subtitle`/`\author`/`\institute`/`\date`
  → `#show: metropolis-theme.with(aspect-ratio:, config-info(…))`; `\frame{\titlepage}` →
  `#title-slide()`; `\begin{frame}{T}` / `\frametitle{T}` → a `== T` slide; `\section{X}` →
  a `= X` section-divider slide; `\subsection` → a `=== ` in-slide heading;
  `\tableofcontents` → `#outline(title: none, indent: 1em)`; a bare `\frame{…}` →
  `#slide[…]`. Columns/blocks/`\alert` are unchanged (they already produce valid Typst that
  works inside touying slides). Overlays remain collapsed to their final state (no
  `#pause`/`#only` is emitted); real overlays and theme-color mapping are deferred to later
  phases. The compile gate resolves `@preview/touying:0.7.3` from the package cache.

## [0.5.23] — unreleased

### Fixed
- ACL author blocks: route through the NeurIPS-style author parser (same `\textbf{Name
  \textsuperscript{n}}` + `\textsuperscript{n} Institution` legend). Real institutions are now
  kept and keyed per author, a `\thanks{Correspondence…}` note is no longer mis-used as the
  affiliation, and `\thanks`/`\footnotemark` no longer leak into names (Phase 2).

## [0.5.22] — unreleased

### Fixed
- ACL papers: the title now renders at `\Large` (1.44em) per `acl.sty`, matching the truth,
  instead of the oversized neutral 1.5em it inherited (the title was visibly too large).

## [0.5.21] — unreleased

### Fixed
- Chapter-bearing layout (`\section` level under `\chapter`, `\tableofcontents`→`#outline`,
  `\frontmatter`/`\mainmatter` page numbering) is now decided by whether the document
  actually uses `\chapter` — detected in the entry-file prepass and via a project-wide scan
  of `\input`'d files — instead of a brittle class-NAME substring heuristic. Fixes false
  positives (`booklet`/`workbook` were treated as chapter-based) and false negatives (a
  custom chapter class whose chapters live in `\input`'d files; health-check P1).

## [0.5.20] — 2026-06-22

First release since v0.3.0 — it bundles all the 0.4.x/0.5.x work below. Highlights:

- **Beamer presentation support** — a LaTeX beamer deck converts to Typst slides: frames
  (one page each), `columns`→grid, blocks→titled `#block`, the title slide, overlays
  collapsed, per-deck theme colors (detected, not hard-coded), and 4:3 / 16:9 geometry.
- **Book/report/thesis support** — chapter/section heading hierarchy, `\tableofcontents`→
  `#outline`, `\frontmatter`/`\mainmatter` page numbering, `longtable`→`#table`, `\subtitle`,
  `\appendix` lettering, and isolated `\begin{titlepage}`.
- **Agent surface** — `diagnose <.typ>` leaked-LaTeX scan, a `warnings.json` sidecar from
  `diagnose --project`, and new `byetex-beamer` / `byetex-book` skills (15 skills total).
- **Converter fixes** — author-block marker leaks, `\addtocounter`/theorem-`\label`
  underscore leaks, `\text{$…$}` inner-math, the `\abstract`/algorithm-block recipes, and
  many more (see the per-version sections below).

### Fixed
- `\begin{titlepage}` is now isolated on its own page (pagebreak before + after) instead
  of its content flowing into the following frontmatter/chapter (round-6 dogfood A6).

## [0.5.19] — 2026-06-22

### Fixed
- `\appendix` now resets the heading counter (`#counter(heading).update(0)`), so the
  first appendix is A — previously appendices continued the body count (e.g. D/E after
  three chapters; round-6 dogfood).

## [0.5.18] — 2026-06-22

### Added
- New `byetex-book` skill documenting how ByeTex converts book/report/thesis classes
  natively (ToC, page numbering, chapter/section hierarchy, long tables) and the few
  constructs to fix by hand — so agents stop re-implementing what works (round-5 T3).
  Linked from `byetex-getting-started` (doc-type routing) and the skills INDEX.

## [0.5.17] — 2026-06-22

### Fixed
- Book/report `\frontmatter`/`\mainmatter` now switch page numbering (roman → arabic
  reset to 1) via Typst `#set page(numbering:)` + a page-counter reset, instead of being
  dropped (round-5 dogfood T-frontmatter).

## [0.5.16] — 2026-06-22

### Fixed
- Book/report/thesis `\tableofcontents` now renders a `#outline` of the chapters/sections
  instead of being dropped (extends the beamer ToC to chapter-bearing classes; round-5 T-toc).

## [0.5.15] — 2026-06-22

### Fixed
- Book/report/thesis heading hierarchy: in a chapter-bearing class (`book`/`report`/
  `memoir`/KOMA/thesis), `\section` now renders at heading level 2 under `\chapter`
  (subsection at 3, …) instead of being flattened to level 1 (round-5 dogfood T2).

## [0.5.14] — 2026-06-22

### Fixed
- `\subtitle{…}` is now rendered under the title for ALL document classes (report, book,
  thesis, article-with-subtitle-package), not just beamer — it was dropped elsewhere,
  losing the subtitle on title pages (round-5 dogfood T1).

## [0.5.13] — 2026-06-22

### Fixed
- `longtable`/`longtable*`/`xltabular` (multi-page tables, common in theses and papers)
  now render as a Typst `#table` instead of being dropped wholesale; the page-break
  markers (`\endhead`/`\endfoot`/…) are dropped no-ops (round-5 dogfood). The table
  `\caption` is not yet carried over.

## [0.5.12] — 2026-06-22

### Fixed
- `byetex diagnose --project`/`--flat` now writes a `<stem>.warnings.json` sidecar next to
  the `.typ`, so an agent repairing a diagnosed project (e.g. the dogfood harness) can see
  silently-dropped constructs instead of only compile errors (round-4 dogfood R2).

## [0.5.11] — 2026-06-22

### Fixed
- `\text{…}` inside math now re-converts an embedded `$…$` to Typst math (e.g.
  `\text{if $x_t = y$}` in a `cases()` condition) instead of leaving the dollar signs
  literal. Handles escaped `\$`, unbalanced `$`, and quote/backslash escaping safely
  (round-4 dogfood A5).

## [0.5.10] — 2026-06-22

### Fixed
- `\label{key_with_underscore}` inside a theorem-like environment no longer leaks its
  tail (`_to_denoiser`) as body text — tree-sitter truncates the key at the first `_`, so
  the whole `\label{…}` is now consumed (round-4 dogfood A2).

## [0.5.9] — 2026-06-22

### Fixed
- Beamer `\section`/`\subsection` between frames now starts its own section slide
  instead of the heading bleeding onto the previous slide (round-4 B6).

## [0.5.8] — 2026-06-22

### Fixed
- Beamer `\tableofcontents` now renders a section outline (`#outline`) on the slide
  instead of being dropped — the Outline slide lists the deck's sections (round-4 B-toc).

## [0.5.7] — 2026-06-22

### Fixed
- `\addtocounter{c}{-1}` (and the counter-setter family) no longer leak as literal body
  text when a value breaks the parse — a negative step parses as a greedy ERROR node; the
  command + its args are now dropped while following content is preserved (round-4 A1).

## [0.5.6] — 2026-06-22

### Fixed
- Beamer `\subtitle{…}` is rendered under the title on the title slide instead of being
  dropped (round-4 dogfood B-subtitle).

## [0.5.5] — 2026-06-22

### Added
- New `byetex-beamer` skill documenting how ByeTex converts beamer presentations
  natively (frames, columns, blocks, overlays, theme colors) and the few constructs to
  fix by hand — so agents stop re-implementing what the converter already does. Linked
  from `byetex-getting-started` and `byetex-unsupported-environment` (round-4 dogfood R1).

## [0.5.4] — 2026-06-22

### Fixed
- Beamer `\alt<spec>{default}{alternative}` now shows the default arg and drops the
  spec + the alternative (was leaking the `<spec>` and rendering both args).

## [0.5.3] — 2026-06-22

### Fixed
- Beamer title slide shows the author and `\institute` as plain centered lines instead
  of the academic-paper superscript-numbered affiliation footnoting.

## [0.5.2] — 2026-06-22

### Added
- Beamer frame titles now render in the deck's theme color, DETECTED per deck:
  `\setbeamercolor{frametitle|structure}{fg=…}` + `\definecolor` are honored exactly,
  `\usecolortheme{name}` maps to the theme's structure color, and a stock deck falls
  back to beamer's default structure blue (instead of a hard-coded blue for all decks).

## [0.5.1] — 2026-06-22

### Fixed
- Beamer aspect ratio: decks now default to the beamer-standard **4:3** slide page and
  honor `\documentclass[aspectratio=169]{beamer}` (and 16:10/14:9) for widescreen,
  instead of always forcing 16:9. (Class-option parsing now keeps `key=value` values.)

## [0.5.0] — 2026-06-22

Autonomous-dev cycle: a self-improving loop that raises converter fidelity and
hardens the agent surface, dogfooded by a fresh model each tick. Highlights below.

### Added
- **`byetex diagnose <file.typ>`** — diagnose an already-edited Typst file IN PLACE
  (compile + map errors, no re-convert), so an agent's fixes survive; wired into the
  CLI, the MCP `diagnose` tool, and the agent_brief / repair-loop guidance.
- **ACL venue page geometry** — auto-detect `acl.sty` and apply a4 paper, 2.5cm
  margins and a 10pt body (the values the venue forces over the class options).
- **`tcolorbox` recipe** — a reusable `#block`-based translation in the
  `byetex-unsupported-environment` skill, plus `needs_manual_review` skill routing.
- **overset family** — `\overset`/`\underset`/`\stackrel`/`\accentset` → Typst
  `attach(base, t|b: script)` (previously dropped both args).

### Fixed
- array `>{…}` column-decorator alignment now propagates to the Typst column.
- Numerous body-leak bugs: `\footnotemark[N]`, heading underscore `\label`, TeX
  register/penalty `=NNNN` tails, `\ExplSyntaxOn…\ExplSyntaxOff` expl3 regions,
  `\setminted`/counter-command arguments.
- `figure*` / `table*` now span both columns in two-column layouts.
- `byetex-math` skill: `op(..., limits: #true)` (bare `true` doesn't compile) +
  `chevron.l/.r` over deprecated `angle.l/.r`; `byetex-getting-started` gained a
  fidelity-phase section.
- `algorithm` floats now render their `algorithmic` pseudocode instead of an empty
  placeholder.
- `\abstract{…}` **command** form (class-redefined, e.g. bytedance) is now captured
  like the `abstract` environment instead of being dropped.
- Author-block marker leak: `\footnotemark[N]` (and other `\cmd[opt]` markers) no
  longer leak their `[N]` as literal text next to author names.
- NeurIPS multi-author blocks (`\textbf{Name}$^{n}$ \quad …` with a `$^{n}$`
  affiliation legend) now split into separate authors instead of one concatenated name.
- `byetex-using-warnings-json` skill: actionable triage table for `unsupported_command`
  (most are benign drops) instead of a circular self-pointer; range note clarified.
- `byetex-unsupported-environment` skill: `algorithm`/`algorithmic` pseudocode recipe
  (captioned `#figure` + numbered `#enum`, with a `\STATE`/`\FOR`/`\IF` line mapping).
- `byetex diagnose <file.typ>` (in-place) now also scans for **leaked LaTeX**
  (un-converted `\command`s and `\[..\]` markers that compile but render literally),
  surfacing fidelity issues that `typst compile` reports as clean.
- `byetex-getting-started` skill documents the new in-place leaked-LaTeX scan so agents
  discover it.

### Added
- Foundational **beamer** (LaTeX presentations) support: the `beamer` class is detected
  and each `frame` renders as its own page with a bold slide title (`\begin{frame}{T}`
  or `\frametitle{T}`) instead of being dropped. (Columns, blocks, and the title slide
  are follow-ups.)
- Beamer `columns`/`column` → a Typst `#grid` (column widths mapped to `fr` ratios),
  so two-column slide layouts keep their content instead of being dropped.
- Beamer `block`/`alertblock`/`exampleblock` → a titled `#block` (accent-colored header
  + left rule) instead of being dropped.
- Beamer `\frame{…}` command form renders as a slide (short-form slides no longer
  dropped); `\frame{\titlepage}`/`\titlepage` resolve to the auto-emitted title slide.
- Beamer decks now render on a landscape 16:9 slide page (presentation geometry: larger
  base font, tight margins, ragged-right) instead of the us-letter article layout.
- Beamer overlay specs (`\item<1->`, `\pause`, `\only`/`\uncover`/`\onslide`/`\visible`/
  `\alert`) are handled: content is shown unconditionally and the `<…>` spec no longer leaks.

## [0.3.0] — 2026-06-15

Agent-in-the-loop hardening + distribution. The MCP server grows from 7 to
**11 tools**, the visual-fidelity loop becomes one command, and ByeTex ships as a
Claude Code plugin.

### Added
- **Agent tools (7 → 11), MCP + CLI parity:** `validate` (Stage-0 input oracle),
  `compile` (typst → PDF with structured errors), `render` (→ per-page PNGs),
  `explain` (per-node LaTeX → Typst map).
- `byetex convert -c <code>` / `-` (stdin) — convert a snippet to stdout, no files.
- `convert_fragment` now honours its `context_hint` (math hints wrap the fragment
  so bare math converts as math, not an unknown text command).
- `byetex review <paper>` — one-command visual grading packet (truth↔typst page
  images) for the `byetex-visual-grading` skill.
- Two-layer fidelity regression gate: `scripts/fidelity_gate.sh` (deterministic
  structural metrics) + `scripts/findings_diff.py` (vision-graded findings).
- **Claude Code plugin:** `.claude-plugin/plugin.json` + `marketplace.json`,
  `.mcp.json`, and a SessionStart hook; skills restructured to
  `skills/<name>/SKILL.md`.
- Distribution: `install.sh`, a Homebrew formula (`packaging/byetex.rb`), and
  crates.io metadata.

### Changed
- CLI package renamed `byetex-cli` → `byetex` so `cargo install byetex` works
  (binary name and the `crates/byetex-cli` directory are unchanged).
- `byetex_core` gains shared `validate`, `compile`, and `snippet` modules,
  mirroring `diagnose` (pure logic + an orchestrator both surfaces share).

## [0.2.0] · [0.1.0]

See the git history (`git log v0.1.0..v0.2.0`). Highlights: the hand-rolled
LaTeX → Typst core, project mode, the `diagnose` repair loop + skills, per-class
fidelity, and the corpus reaching 100 % compile.
