# ByeTex Agent-Surface Backlog

Ranked friction the **fresh dogfood agent** (`byetex-dogfood-tester`, Sonnet, byetex
surface only) hit while repairing seeded conversions in a sandbox. Each item names
whether the fix is **Loop A** (deterministic converter) or **Loop B** (the agent
surface: skill / MCP tool / CLI flag / diagnostic), with paper evidence. Ranked by
frequency × peak severity.

- **Machine source of truth:** `docs/agent-surface-backlog.jsonl` (one record per
  dogfood run, appended by `scripts/dogfood.py score`). This `.md` is curated from it.
- **How items arrive:** `score` prints `NEEDS_FIX` for a paper whose report contains a
  stuck point (`workaround`/`gave_up`), a `blocker`/`major` unclear-skill note, a
  recurring `missing_tool_wishlist`, a `self_report_mismatch`, or a silent fidelity loss.
- **Routing & verdict rules:** see `docs/autonomous-dev.md`.
- **Resolution discipline:** a fix cites the item id (Fn) it closes and re-dogfoods the
  evidence papers **twice** before the item moves to Resolved.

Item id scheme: `F<n>`. Severity peaks 1–5 (reader/agent impact).

---

All 3 tick-1 (2026-06-17) reports are **complete real reports** (each agent ran
~40–66 min and emitted a final report; seeds already COMPILED, so all work was
fidelity polish). Verdicts: all `NEEDS_FIX` (clean compile reached only via
workaround/gave-up). Re-dogfood any item's evidence papers twice before marking it
Resolved.

## Open — P0 (frequent × blocking)

> **Round 3 (2026-06-19)** — re-dogfood of the lowest-recall arxiv papers
> (`2606.12397`, `2605.22765`, `2605.22786`) after round-2 cleared. **F6 VERIFIED
> LANDED** (all 3 agents now use `byetex diagnose paper.typ`). New theme below.

### G1. Author-block parsing — 3 papers, peak sev 4 (major) — ✅ MOSTLY RESOLVED (#299 + #301)
- **Symptom:** author blocks mis-parse across all 3 papers. (a) marker leak
  `\footnotemark[1]`→`\[1\]` (2606.12397) — **✅ #299**; (b) **5 authors CONCATENATED
  into one name** (2605.22786, NeurIPS `\textbf`+`\quad` pattern) — **✅ #301**
  (`parse_neurips_textbf_authors`, splits + attaches `$^{n}$` legend affiliations).
- **Residual (P2):** `\blfootnote` / `\addtocounter{c}{-1}` (negative-value counter
  that doesn't node-parse) still leak (2605.22765); per-author affiliation-superscript
  display is approximate. Low value — revisit if a dogfood re-flags it.

### G2. `unsupported_command` → `byetex-using-warnings-json` circular routing — 2 papers, sev 4 — ✅ RESOLVED (PR #303)
- **Symptom:** 96 `unsupported_command` warnings all `suggested_skill =
  byetex-using-warnings-json`, which only explains the schema — "lands on the same page
  they started from" (2605.22765, 2605.22786). Same class as the `needs_manual_review`
  routing fixed in #274.
- **Next:** route common `unsupported_command`s to an actionable skill (math/custom-
  macros/unsupported-environment by name), or make `byetex-using-warnings-json` a real
  dispatch table. Pairs with adding an `algorithm` recipe (G4).

> **Round 2 (2026-06-18)** — fresh dogfood of the new hardest-3 (`2605.22821`,
> `2605.31510`, `2605.22728`) after the tick-1 backlog cleared. All 3 seeds compiled;
> all work was fidelity; all `NEEDS_FIX`.

### F5. Preamble / non-body content leaks verbatim into the body — 3 papers, peak sev 5 (blocker) — ROUTE: Loop A (region-skip)
- **Symptom (agent's words):** content that should be dropped is rendered as garbage
  text. `\ExplSyntaxOn … \ExplSyntaxOff` (expl3) leaked **~294 lines** + `\setminted{}`
  options (2605.22821); `\begin{document}` + affiliation block (2605.22728);
  `\refstepcounter{ALC@line}`, `12pt`, `url@samestyle` (2605.31510). Flagged
  `unsupported_command` "raw source dropped" but **not** dropped — leaked.
- **Signal:** stuck_point(workaround) on 3/3 + `unclear_skill_notes` **blocker**.
- **Progress:** `\ExplSyntaxOn … \ExplSyntaxOff` region-skip ✅ (PR #282, ~294 lines
  → 0); `\setminted[..]{..}` options + counter commands (`\setcounter`/`\stepcounter`/
  `\refstepcounter`) ✅ (PR #289 — node-kind drop + minted arg consumption; code-review
  caught & fixed an over-consumption regression). **Still open:** `\begin{document}`+
  affiliation leak (2605.22728), and the pre-existing tree-sitter over-attachment where
  a `{...}`-led paragraph after a no-output command is swallowed. Pairs with F12
  (`leaked_to_body` vs `dropped_silently`).

### F6. `byetex diagnose <main.typ>` (PR #278) is shipped but not DISCOVERABLE — 3 papers, peak sev 4 — ✅ ADDRESSED (PR #284), verify next round
- **Symptom:** all 3 agents *still* wished for "diagnose --incremental on the edited
  .typ" — even though #278 added exactly that. Root cause: `byetex-getting-started` (the
  FIRST skill read) still carried the stale "Critical rule: do NOT re-run byetex
  diagnose" and had **no fidelity-phase guidance**, so during fidelity work (seed already
  compiles) agents never reached `byetex-repair-loop` where #278 was documented.
- **Fix (PR #284):** rewrote `byetex-getting-started` — replaced the stale rule with the
  in-place `byetex diagnose paper.typ` guidance, added a "fidelity phase" section, framed
  the task as compile→fidelity. **Verify on the next dogfood round** (do the agents stop
  asking for it / start using `diagnose <main.typ>`).


### F1. `diagnose --incremental` — re-diagnosing an edited `.typ` WIPES the edits — 3 papers, peak sev 4 — ✅ RESOLVED (PR #278)
- **Symptom (agent's words):** "After I found fidelity issues by visual inspection,
  there was no way to get a skill-mapped diagnostic scan of the edited file. I had to
  manually scan main.typ." All 3 agents independently asked for this.
- **Evidence:** `2606.12397`, `2605.31564`, `2605.31586` (all `missing_tool_wishlist`).
- **Fix:** `byetex diagnose <file.typ>` (and the MCP `diagnose` tool with a `.typ`
  path) now compiles an existing `.typ` IN PLACE and maps its typst errors without
  re-converting, so edits survive (`src_fragment`/`skill_name` null — no source map).
  The agent_brief + `byetex-repair-loop` skill now tell agents to re-scan via
  `byetex diagnose <main.typ>` instead of the old "never re-run diagnose" rule.
  New `diagnose_typ.rs` integration test; verified end-to-end (edited `.typ` →
  error mapped at the right line, edit preserved).

## Open — P1 (class / recipe gaps)

### F2. ACL / venue style overrides class defaults (a4 + 10pt + 2.5cm) — 3 papers, peak sev 4 — ROUTE: Loop A (class fidelity) — ✅ RESOLVED (PR #267)
- **Fix:** `Layout::apply_venue_style(class)` forces a4 + 10pt for `DocClass::Acl`
  + 2.5cm margin (unless explicit user geometry), at begin-document. Corpus fidelity
  **0.821→0.826**; 5 ACL papers' page_ratio → ~1.0 (2606.12397 1.643→0.929) and
  word_recall up (0.646→0.717); +4 structure_ok; baseline promoted. 5 TDD tests.
- **Symptom (agent's words):** "ACL style overrides documentclass font size (11pt→10pt),
  letter→a4, 1in→2.5cm margins. byetex did not pick this up, leading to ~50% page-count
  inflation that I had to fix manually by reading acl.sty." This is the dominant
  `page_ratio` driver across all 3 hardest papers.
- **Evidence:** `2605.31586` (page 27→21 vs 18 truth after a4/10pt by hand; +0.043
  fidelity), `2605.31564` (page_ratio 1.32), `2606.12397` (1.14).
- **Signal:** deterministic `page_ratio` overshoot on 3/3 + explicit ACL trace. ACL is
  already detected for two-column ([[project-two-column-layout]] #247) and there's a
  per-DocClass `StyleProfile` ([[project-class-fidelity]] #210–214) — extend the ACL
  hook to set a4 paper + 2.5cm margins + 10pt body when `acl.sty`/`\usepackage{acl}`
  is present (PACKAGE-keyed, not DocClass — `\documentclass{article}`+`\usepackage{acl}`).
- **Note:** render-affecting → run the fidelity gate; expect page_ratio to *improve*
  (legit baseline bump), guard non-ACL papers with precise detection.

### F3. `tcolorbox` has no conversion recipe — 1 paper, peak sev 3 — ✅ RESOLVED (PR #273 + #274)
- **Symptom (agent's words):** "byetex-unsupported-environment covers theorem/lstlisting/
  beamer but NOT tcolorbox… I improvised a custom Typst block." `tcolorbox` (framed
  colored boxes, title bars) is used extensively in ML papers.
- **Fix:** (1) PR #273 added a reusable `#let tcolorbox(...)` recipe + option-mapping
  table to `byetex-unsupported-environment` (and broadened its description to cover
  `needs_manual_review` boxes). (2) PR #274 routed the `needs_manual_review` default
  `suggested_skill` from `byetex-using-warnings-json` → `byetex-unsupported-environment`
  so agents are auto-routed to the recipe.
- **Verified:** re-dogfood of `2605.31564` (2026-06-18) — **stuck_points: []**, agent
  used the recipe successfully ("provided the exact recipe to rebuild tcolorboxes");
  grey placeholder → 3 styled framed boxes matching truth. The major `unclear_skill_note`
  that drove that run's NEEDS_FIX was the routing gap, now closed by #274.
- ~~**Residual: `figure*` two-column spanning**~~ — ✅ RESOLVED (PR #276): `emit_figure`
  now wraps a starred float (`figure*`/`table*`) in `#place(top, scope: "parent",
  float: true)[…]` under two-column, so wide floats (and rebuilt `needs_manual_review`
  boxes) span both columns automatically. 5 TDD tests.

### F7. Algorithm/pseudocode environments dropped entirely — 2 papers, peak sev 4 — ✅ RESOLVED (converter; PR #294)
- **Symptom:** `\begin{algorithm}` bodies were **completely absent** from the `.typ`
  (empty `needs_manual_review` placeholder) — agent had nothing to translate.
- **Fix (PR #294):** `emit_figure` now captures the nested `algorithmic` block(s) and
  renders their steps (left-aligned; `\State`/`\For`/… degrade to text) as the figure
  body. 2605.31510 word_recall 0.823→0.846, structure_ok False→True. 4 TDD tests.
- **Residual (Loop B, lower value now):** a dedicated algorithm→Typst recipe in
  `byetex-unsupported-environment` would let an agent restore the pseudocode STRUCTURE
  (keywords/indent), not just the content. Defer until a dogfood shows it still hurts.

### F8. overset family drops args → `"accentset"`/`"overset"` strings — 1 paper (37×), peak sev 4 — ✅ RESOLVED (PR #286)
- **Symptom:** `\accentset{\circ}{\bm h}` (and `\overset`/`\underset`/`\stackrel`)
  emitted the bare command name as a string in math with both args lost (2605.31510:
  37 `\accentset` sites). byetex-math documented `attach` but the converter never did it.
- **Fix:** `emit_math_attach` maps the whole family to `attach(base, t|b: script)`
  (top-set overset/stackrel/accentset, bottom-set underset/underaccent). 2605.31510:
  `"accentset"` 37→0, replaced by 37 `attach(...)`. 5 TDD tests.

### F9. `byetex-using-warnings-json`: ranges are LaTeX lines, not `.typ` lines — 2 papers, peak sev 4 (major) — ROUTE: Loop B (skill + tool)
- **Symptom:** the skill says "fix the `.typ` at the given line/column range", but the
  ranges are in the **LaTeX source**; after conversion (and edits) they don't map to
  `.typ` lines, so agents grep for rendered strings by hand (2605.31510, 2605.22728).
- **Next:** correct the skill to say the ranges are source-side + route to
  `byetex diagnose <main.typ>` (F6) for `.typ`-line-anchored errors; consider adding
  `.typ` line numbers to `warnings.json` (overlaps F13).

## Open — P2 (polish / low frequency)

### G3. `byetex diagnose <.typ>` now surfaces FIDELITY warnings — 3 papers — ✅ RESOLVED (PR #307)
- **Symptom:** all 3 round-3 agents now USE `byetex diagnose paper.typ` (F6 landed) but
  note it only maps COMPILE errors, not the fidelity `warnings.json` against the edited
  `.typ`. They want a re-scan that flags leaked-LaTeX / fidelity issues post-edit.
- **Next:** extend the in-place `diagnose <.typ>` to also run a leaked-fragment scan
  (overlaps the old F12/F13 `warnings --fidelity` wish).

### G4. `algorithm` box framing (skill recipe) — 2 papers — ✅ RESOLVED (PR #305)
- **Symptom:** #294 preserves the algorithm pseudocode as prose, but agents want the
  numbered-box framing (`\STATE`/`\FOR`/`\ENDFOR` → numbered indented steps). No
  `algorithm`/`algorithmic` entry in `byetex-unsupported-environment`.
- **Next:** add an algorithm→Typst recipe (numbered block / `#enum` with indent) to the
  skill; route `\STATE`/`\FOR` unsupported_command warnings there (pairs with G2).

### F10. `@`-command (`\makeatletter`) macros leak as strings — 1 paper (19×) — Loop A
- `\E` (defined via `\@ifstar`) renders as `"@ifstar" "@@E" "@E"` strings in math
  (2605.31510). `@`-named macro call sites lose their structure.

### F11. More deprecated Typst symbols in math — minor
- `times.circle` → `times.o` (2605.22728); `angle.l/.r` → `chevron.l/.r` already added
  to byetex-math (#280). Consider a deprecated-symbol cheatsheet in the skill.

### F12. `leaked_to_body` vs `dropped_silently` warning category — Loop B (taxonomy)
- Agents can't tell from `warnings.json` whether an `unsupported_command` was dropped
  or leaked into the body (it claims "dropped" even when it leaked — see F5). A distinct
  category would tell them to go delete the garbage. (Best paired with fixing F5.)

### F13. `warnings.json` → `.typ` line numbers — Loop B
- Several agents wanted each warning to carry the `.typ` line it maps to, not just the
  LaTeX source range (overlaps F9). Largely subsumed by F6's `diagnose <main.typ>`.

### F4. Converter content-leak bugs surfaced by dogfood (Loop A) — 1–2 papers each
- ~~**`\footnotemark[N]` → `#footnote[]\[N\]`** (`2606.12397`)~~ — ✅ RESOLVED (PR #265):
  emitted a spurious empty footnote + leaked `[N]` as `\[N\]`. Now consumes the optional
  arg, emits `#super[N]`, no footnote (4 TDD tests; gates green).
- ~~**Numeric assignment tail leak** (`2605.31586`)~~ — ✅ RESOLVED (PR #271):
  `\interfootnotelinepenalty=10000` dropped but `=10000` leaked as a heading.
  `emit_generic_command` now consumes a `=<number>[unit][ plus/minus <d>]` tail after
  an unhandled control word (`peek_tex_assignment_end`). 5 TDD tests.
- ~~**Leaked `\label` fragments as body text** (`2605.31586`)~~ — ✅ RESOLVED (PR #269):
  underscore labels on a heading (`\label{sec:exp1_main}`) leaked the `_main` tail as
  body text. `emit_section` now consumes the full brace span via
  `extract_label_name_and_end` + `skip_until`. 4 TDD tests.
- **`warnings --fidelity` leak scanner** (Loop B wish, `2605.31586`): a post-convert
  scan that flags leaked label/numeric-tail/custom-comment-macro fragments in the
  `.typ` body (all invisible to `warnings.json`, which only logs the original command).

## Resolved

_None yet. Format:_

> ### F0. <one-line symptom> — N papers, peak sev X — ROUTE: Loop B (skill) — ✅ RESOLVED (PR #NNN)
> - **Symptom (agent's words):** "<why_insufficient / wishlist text>"
> - **Evidence:** `<id>` (resolution=gave_up, after=0.71 vs before=0.69), `<id>`, …
> - **Signal:** unclear_skill_notes(blocker) + stuck_point(gave_up)
> - **Fix:** <what changed> — re-dogfooded `<id>`,`<id>` twice → GOOD_ENOUGH.

## Round-4 arxiv re-dogfood (2026-06-20, 2605.22765, v0.5.6)

Math-heavy diffusion paper; 113-min agent run. Compiled from the start; all fidelity work.
Findings (general, non-beamer):
- **A1 ✅ FIXED (PR #331, v0.5.7) — `\addtocounter{c}{n}` leaks as body text** (verified on main:
  `\addtocounter{footnote}{-1}` renders literally). Recurring across multiple dogfoods.
  Negative-value counters don't node-parse → fall to generic → args leak. Fix: drop the
  whole `\addtocounter{}{}` (incl. both arg groups) in any class.
- **A2 ✅ FIXED (PR #335, v0.5.10) — `\label` leaks as text inside a `proposition`** (`\_to\_denoiser`
  shown as body). A `\label` in a theorem-like env emitted as a text fragment.
- **A3 (P2, skill) — `\newcommandx` (xargs) + `\ifthenelse` macros** → 838 ambiguous_math
  upright-text literals. `byetex-custom-macros` only covers plain `\newcommand`. Hard
  (conditional, arg-count-dependent macros); document the limitation + a manual recipe.
- **A4 ✅ MOOT (resolved by A1 #331; all counter cmds now drop cleanly, no leak to triage) — extend `byetex-using-warnings-json` triage** to list
  `\addtocounter`/`\setcounterref`/`\crefalias` as "benign if dropped; check body for leaked
  text" so agents find the A1 leaks fast.
- **A5 ✅ FIXED (PR #337, v0.5.11) — `\text{…}` containing unconverted inner math/macros** (cases() conditions like
  `\text{if $\mask$}`) — the outer `\text` converts but inner `$…$`/macros don't.

## Round-5 dogfood (2026-06-21, v0.5.12→13) — R2 verified helpful

Two fresh agents WITH the new `warnings.json` sidecar (R2 #339). Both confirmed it HELPED:
the math agent "warnings.json was very helpful for prioritizing… 840 ambiguous_math grouped
by macro name with occurrence counts"; the thesis agent used it to find `\tableofcontents`/
`\frontmatter` drops. R2 measurably improved agent effectiveness vs round-4.

### Done this round
- **longtable** dropped → `#table` (PR #341, v0.5.13). VERIFIED bug.

### Thesis (book-class) findings — NEW track
- **T1 ✅ FIXED (PR #343, v0.5.14) — `\subtitle` dropped in non-beamer** (report/book). VERIFIED. The
  subtitle machinery exists (beamer #329); extend capture to report/book + render under
  `\maketitle` title. Quick.
- **T2 ✅ FIXED (PR #345, v0.5.15) — `\section*` inside `\chapter` is level-1, not level-2** (book/report
  heading hierarchy: chapter=1, section=2). VERIFIED. Headings flattened, hierarchy lost.
- **T3 ✅ DONE (PR #349, v0.5.18) — no `byetex-book` skill** (like byetex-beamer R1): `\frontmatter`/
  `\mainmatter` page-numbering, `\tableofcontents`→#outline, chapter-vs-section depth,
  thesis title page. All had `suggested_skill: null`.
- **T4 (P2) — book/thesis author block is article-style** (superscript affiliation) — wrong
  for a thesis title page (title+subtitle only).
- **T5 (P2, warnings) — `byetex-using-warnings-json` triage** conflates benign drops
  (`\newpage`) with HIGH-IMPACT structural drops (ToC, frontmatter); should distinguish.

### Math-paper findings (recurring, = round-4 A3)
- **A3 (P2, HARD) — `\newcommandx`+`\ifthenelse` macros** = 840/943 warnings (89%); the #1
  math-paper fidelity gap. `byetex-custom-macros` only covers plain `\newcommand`.
- **M1 (P2, warnings) — `ambiguous_math` warnings have EMPTY src_fragment/typ_region** → agents
  can't locate them in the .typ programmatically; had to grep. Fixable warning-quality bug.

## Round-6 dogfood (2026-06-21, v0.5.18→19) — book-class work VALIDATED

Thesis RE-TEST (same doc as round-5) + a hard paper, both with warnings.json + the new
byetex-book skill. **Result: the book-class track measurably paid off.** Round-5 needed
6 manual workarounds (ToC/page-num/subtitle/heading-levels/longtable/author-block all
improvised); round-6 the agent confirmed those 5 are now auto-handled and "the byetex-book
skill saved significant exploration time" — it did NOT reinvent them. Paper agent: "surface
worked well", warnings.json prioritized correctly, one skill read sufficed.

### Done this round
- **A7 appendix counter** (PR #351, v0.5.19): `\appendix` now resets the heading counter
  (D/E → A/B). VERIFIED.

### New findings (round-6)
- **A6 ✅ FIXED (PR #353, v0.5.20) — `\begin{titlepage}` emits as LOOSE body content** (not isolated):
  in a thesis the inner titlepage tables flow into the frontmatter. VERIFIED. Fix: map
  `titlepage` env to a `#page[...]`/pagebreak-isolated scope.
- **T4 (still open) — thesis author block article-style** (superscript affiliation on a
  thesis title page). The byetex-book skill flags it but no converter fix yet.
- **M2 (P3, paper) — lstlisting per-line highlights** (`\bluebg`/`\pinkbg` via `(*..*)`):
  #raw has no per-line bg API; document limitation (or `#show raw.line`). Niche.
- **M3 (P3) — `dot.circle`/`bracket.double` Typst DEPRECATIONS** emitted by the converter;
  could emit `dot.o`/`bracket.stroked` directly (Typst version drift). Low.
