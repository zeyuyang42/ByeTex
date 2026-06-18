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
- **Next (biggest first):** skip `\ExplSyntaxOn … \ExplSyntaxOff` regions like the
  existing `\makeatletter` region-skip ([[project-preamble-leakage]] #131); then the
  `\setminted`/`\refstepcounter`/`\begin{document}` leak sources. Pairs with F12
  (a `leaked_to_body` vs `dropped_silently` warning category).

### F6. `byetex diagnose <main.typ>` (PR #278) is shipped but not DISCOVERABLE — 3 papers, peak sev 4 — ROUTE: Loop B (surface)
- **Symptom:** all 3 agents *still* wished for "diagnose --incremental on the edited
  .typ" — even though #278 added exactly that and updated `byetex-repair-loop`. During
  **fidelity** work the seed already compiles, so agents never enter the repair loop /
  read that skill, and never discover the capability.
- **Next:** surface "re-scan an edited `.typ` with `byetex diagnose <main.typ>`" where
  fidelity agents actually look — the agent_brief's fidelity guidance (not just the
  "How to repair" section) and `byetex-getting-started`. Cheap, high-leverage (the
  feature already exists; it just isn't found).


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

### F7. Algorithm/pseudocode environments dropped entirely — 2 papers, peak sev 4 — ROUTE: Loop A (+ skill)
- **Symptom:** `\begin{algorithm}` bodies are **completely absent** from the `.typ`
  (empty `needs_manual_review` placeholder), not even raw text — so the agent has
  nothing to translate (2605.31510: 3 algos; 2605.22728: 5 algos; resolution=gave_up).
- **Next:** preserve the algorithm body (raw/structured) so it's recoverable, AND add
  an algorithm→Typst recipe to `byetex-unsupported-environment` (it covers tcolorbox/
  lstlisting/beamer but not algorithm/algorithmic pseudocode).

### F8. `\accentset{accent}{base}` drops its argument → `"accentset"` string — 1 paper (37×), peak sev 4 — ROUTE: Loop A (math)
- **Symptom:** `\accentset{\circ}{\bm h}` emits the bare string `"accentset"` in math
  with the base/accent lost (37 sites, different targets) — not recoverable by the
  agent (2605.31510, resolution=gave_up). byetex-math documents `attach`, but only
  works when the argument survived.
- **Next:** handle `\accentset{a}{b}` in math → `accent(b, a)` / `attach(b, t: a)`.

### F9. `byetex-using-warnings-json`: ranges are LaTeX lines, not `.typ` lines — 2 papers, peak sev 4 (major) — ROUTE: Loop B (skill + tool)
- **Symptom:** the skill says "fix the `.typ` at the given line/column range", but the
  ranges are in the **LaTeX source**; after conversion (and edits) they don't map to
  `.typ` lines, so agents grep for rendered strings by hand (2605.31510, 2605.22728).
- **Next:** correct the skill to say the ranges are source-side + route to
  `byetex diagnose <main.typ>` (F6) for `.typ`-line-anchored errors; consider adding
  `.typ` line numbers to `warnings.json` (overlaps F13).

## Open — P2 (polish / low frequency)

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
