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

## Open — P0 (frequent × blocking)

_None yet — populated by the first dogfood cycles._

## Open — P1 (class / recipe gaps)

> ⚠️ The 3 items below are **provisional** — the tick-1 (2026-06-17) dogfood agents
> were suspended mid-run and never emitted a final report (see
> `project-autonomous-dev-loop` memory). Signals are **observed from the agent
> transcripts + the deterministic `score` fidelity deltas**, not self-reported. Each
> MUST be re-dogfooded (un-truncated) twice before it's acted on or Resolved.

### F1. Two-column `figure*` / large floats don't span columns — 2 papers, peak sev 4 — ROUTE: Loop A (likely)
- **Symptom (observed):** in a two-column layout, a wide float (`figure*`, a big
  `tcolorbox`) is emitted with `placement: none` (inline) instead of a parent-scope
  spanning float, so it overflows a column → blank page before it + page-count
  overshoot. The agent manually reached for `#place(scope: "parent", float: true)`
  (the [[project-two-column-layout]] recipe) but burned many turns and on 2605.31564
  made `page_ratio` *worse* (1.32→1.37).
- **Evidence:** `2605.31564` (after=0.77 vs before=0.768, page_ratio 1.32→1.37),
  `2606.12397` (after≈0.74 vs before=0.74 flat, page_ratio 1.14).
- **Signal:** deterministic page_ratio overshoot + observed stuck point (manual
  `scope:"parent"` fight).
- **Next:** check whether the converter emits `figure*` / wide floats with
  `#place(scope:"parent", float:true)` under `#set page(columns: 2)`; if not, that's
  the deterministic fix.

### F2. `lstlisting` code listing inside a figure float is dropped — 1 paper, peak sev 3 — ROUTE: Loop A/B (TBD)
- **Symptom (observed):** a `figure` whose body is a `\begin{lstlisting}` Python
  pseudocode block landed in `warnings.json` as `needs_manual_review` and its content
  was omitted from `main.typ`; the agent identified it but could not recover the
  listing from the surface alone → fidelity stayed ~flat at 0.74.
- **Evidence:** `2606.12397` (after≈0.74 vs before=0.74, word_recall 0.707).
- **Signal:** silent content loss flagged only as `needs_manual_review`; agent had no
  recipe to translate `lstlisting` → Typst `raw` block.
- **Next:** decide route — deterministic `lstlisting`→`#raw(block:true, lang:…)` emit
  (Loop A) vs a skill recipe for code listings (Loop B). Confirm on re-dogfood.

### F3. page_ratio overshoot dominates the hardest-3 fidelity gap — 3 papers, peak sev 3 — ROUTE: Loop A (density)
- **Symptom (observed):** across all 3 hardest papers the largest single fidelity
  drag is `page_ratio` (Typst renders more pages than the LaTeX truth). 2605.31586's
  agent recovered +0.044 fidelity almost entirely by cutting page_ratio 1.5→1.17 by
  hand — a lever the converter should pull deterministically. Known long-tail per
  [[project-layout-density-fidelity]]; re-confirm whether a generic density knob moves
  the corpus before opening a PR.
- **Evidence:** `2605.31586` (1.5→1.167 by agent), `2605.31564` (1.32), `2606.12397`
  (1.14).
- **Signal:** deterministic page_ratio is the top metric gap on every hardest-3 paper.

## Open — P2 (polish / low frequency)

_None yet._

## Resolved

_None yet. Format:_

> ### F0. <one-line symptom> — N papers, peak sev X — ROUTE: Loop B (skill) — ✅ RESOLVED (PR #NNN)
> - **Symptom (agent's words):** "<why_insufficient / wishlist text>"
> - **Evidence:** `<id>` (resolution=gave_up, after=0.71 vs before=0.69), `<id>`, …
> - **Signal:** unclear_skill_notes(blocker) + stuck_point(gave_up)
> - **Fix:** <what changed> — re-dogfooded `<id>`,`<id>` twice → GOOD_ENOUGH.
