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

### F1. `diagnose --incremental` — re-diagnosing an edited `.typ` WIPES the edits — 3 papers, peak sev 4 — ROUTE: Loop B (CLI/diagnostic)
- **Symptom (agent's words):** "After I found fidelity issues by visual inspection,
  there was no way to get a skill-mapped diagnostic scan of the edited file. I had to
  manually scan main.typ." All 3 agents independently asked for this.
- **Evidence:** `2606.12397`, `2605.31564`, `2605.31586` (all `missing_tool_wishlist`,
  each "would_have_saved 1–2 iterations").
- **Signal:** `missing_tool_wishlist` recurring across **3/3** papers. The seam:
  `diagnose --project --out` does a clean materialize that wipes `--out` first
  (known gotcha), so an agent that edits `main.typ` can never re-scan it.
- **Next:** add a `diagnose` mode (CLI flag / MCP param) that scans an existing `.typ`
  in place (no re-materialize) and emits the same fragment→skill map.

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

### F3. `tcolorbox` has no conversion recipe — 1 paper, peak sev 3 (major skill gap) — ROUTE: Loop B (skill)
- **Symptom (agent's words):** "byetex-unsupported-environment covers theorem/lstlisting/
  beamer but NOT tcolorbox… I improvised a custom Typst block." `tcolorbox` (framed
  colored boxes, title bars) is used extensively in ML papers.
- **Evidence:** `2605.31564` (tcolorbox figure rendered as a 4em placeholder rect;
  resolution=workaround; `unclear_skill_notes` severity **major**).
- **Signal:** `needs_manual_review` float with no actionable recipe; the
  `byetex-unsupported-environment` skill gap. (Note: `lstlisting` *was* recovered fine
  via that skill — so the earlier "lstlisting dropped" guess was wrong; only the
  `needs_manual_review`→suggested_skill *routing* is off, pointing to
  `byetex-using-warnings-json` instead of the actionable skill.)
- **Next:** add a tcolorbox→Typst `#block(fill:…, stroke:…)[title + body]` recipe to
  `byetex-unsupported-environment`; fix needs_manual_review routing.

## Open — P2 (polish / low frequency)

### F4. Converter content-leak bugs surfaced by dogfood (Loop A) — 1–2 papers each
- ~~**`\footnotemark[N]` → `#footnote[]\[N\]`** (`2606.12397`)~~ — ✅ RESOLVED (PR #265):
  emitted a spurious empty footnote + leaked `[N]` as `\[N\]`. Now consumes the optional
  arg, emits `#super[N]`, no footnote (4 TDD tests; gates green).
- **Numeric assignment tail leak** (`2605.31586`): `\interfootnotelinepenalty=10000`
  dropped but `=10000` leaked as a Typst heading. A dropped `\<dimen/count>=NNNN`
  should consume its `=value` tail.
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
