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

> **Round 13 (2026-07-01, v0.6.70)** — dogfood of the hardest 3 (`2605.31499` GOOD_ENOUGH,
> `2605.22821` NEEDS_FIX, `2605.22728` NEEDS_FIX). **Headline: the leak scanner is now
> load-bearing** — `2605.22728`'s agent ran `byetex diagnose main.typ`, found 21 real leaks
> (3 `\sectionX`-glued headings, 2 `\begin{align}`+`aligned`/`smash`/`text`, 1 `\begin{proof}`,
> 5 interval `\[..\)`, 4 `\hspace` in citations) and fixed them for **+0.032 fidelity**
> (0.804→0.836). This confirms N1 (#454) and #455 landed. New/recurring findings:
>
> ### N2. `byetex diagnose \[..\]` scanner false-positived unit/prose literals — sev 2 — ✅ RESOLVED (#455 v0.6.70 + this PR v0.6.71)
> - **Symptom:** `2605.31604` `[text tokens, …]` and `2605.31499` `[SNR \[dB\]]` (a table cell)
>   were flagged as "possible leaked LaTeX `\[..\]` marker", but byetex escapes literal `[..]`
>   as `\[..\]` and Typst renders that correctly — false positives that (per N1) now send the
>   dogfood agent chasing ghosts, since agents are routed to this scanner for leaks.
> - **Fix:** #455 skipped whitespace-bearing prose; this PR (v0.6.71) additionally skips compact
>   *alphabetic* literals (`\[dB\]`/`\[IU\]`), flagging only digit/symbol markers (`\[1\]`/`\[*\]`)
>   and math-signal spans. Corpus `2605.31*` sample: `\[..\]` diagnostics 141 → 43 (~70%).
>
> ### N3. leak scanner FINDS leaks; `byetex-using-warnings-json` has no REPAIR recipe — sev 3 (major, recurring, 2/3 papers) — OPEN (Loop-B, but root is deferred)
> - **Symptom:** all 21 of `2605.22728`'s leak diagnostics point to `byetex-using-warnings-json`,
>   which only triages warning *categories* — it gives no recipe to convert a leaked
>   `\begin{align}`+`\begin{aligned}`+`\smash`+`\text` block, a `\begin{proof}`, a `\sectionX`-glued
>   heading, or `\hspace{..}` in a citation to Typst. Agent had to read the LaTeX source and hand-write.
> - **Routing note:** the *underlying* leaks are the deferred L1 bug-A mis-parse (ERROR node → raw-copy
>   gaps; parser-swap / Phase D territory, per [[project-lowering-ir]]). A skill recipe would only
>   band-aid; the deterministic fix is the parser swap. Candidate Loop-B interim: add a "leaked-block
>   first-aid" recipe section to the skill (wrap leaked `\begin{align}…\end{align}` in `$ … $`, split
>   `\sectionTitle` → `= Title`, `\hspace{x}` → `#h(x)`). Needs user steer vs. waiting for Phase D.
>
> ### N4. `diagnose --project` (wipes edits) vs `diagnose file.typ` (safe scan) reads as a contradiction — sev 3 (major) — OPEN (Loop-B, L4)
> - **Symptom (`2605.22821`):** the sandbox rule "never run `byetex diagnose` (it wipes edits)" —
>   which is about `diagnose --project`/`.tex` re-materialization — read as contradicting
>   `byetex-getting-started`'s "`diagnose paper.typ` preserves edits" (a pure body scan). The agent
>   couldn't tell they're different invocations and grepped by hand instead of using the leak scanner.
>   Note `2605.22728`'s agent DID run `diagnose main.typ` successfully, so it's inconsistent, not universal.
> - **Fix candidate (L4):** clarify in the dogfood sandbox scaffolding / getting-started that the
>   "never run diagnose" rule targets `--project` re-conversion, and `diagnose <file>.typ` is a safe,
>   encouraged in-place leak scan.
>
> **Also recurring (all 3 papers): the LaTeX→Typst DENSITY gap** (8v6, 35v26, 29v20 pages) — deferred
> (per-class `StyleProfile`/margins regress naively; = M3/H2). No agent could touch it via the surface.
> Plus `2605.22821`: minted code-figure dropped (no minted→Typst recipe), `\bpe;` custom-macro leak.

> **Round 12 (2026-06-29, v0.6.68)** — VERIFICATION dogfood of `2605.22728` after the L1 fixes.
> **L1 underscore-key (#452) + begin/end-document drop (#453) VERIFIED LANDED:** auto-fidelity
> `fidelity_before` jumped **0.68 → 0.804** (+0.12), the agent's manual delta shrank 0.16→0.03
> (most content now auto-recovered), `\begin{document}` leak GONE, page count 34→29. Still
> NEEDS_FIX from DEFERRED residuals: deep ERROR-gap section/`align`/`proof` leaks (parser-swap
> territory), 5 algorithm placeholders, `\mathbbold{1}` custom alphabet. **New recurring Loop-B
> finding → N1.**
>
> ### N1. `warnings.json` doesn't surface garbled/partial-translation leaks — sev 3 (major, recurring) — ✅ ADDRESSED (PR #454, skill, verify next round)
> - **Symptom:** a leaked `\begin{align}`/`\section`/`\begin{proof}` wrapper renders as body garbage
>   but is NOT in `warnings.json` (which lists only clean drops). The agent consulted
>   `byetex-using-warnings-json` (the skill for leaks), found no guidance, and grepped `main.typ` by
>   hand — believing `diagnose` "only gives compile errors". **The leak-scanner ALREADY EXISTS**
>   (`byetex diagnose main.typ` does compile + a leaked-`\command`/`\[..\]` body scan, PR #307) and
>   getting-started documents it — but warnings-json (the skill agents reach for leaks) didn't.
> - **Fix (#454):** added a "Leaked LaTeX in the body (NOT in warnings.json)" section to
>   `byetex-using-warnings-json` pointing at `byetex diagnose main.typ`, plus a workflow caveat that
>   an empty `warnings.json` doesn't prove a leak-free body. Skill-only → verify by re-dogfood.

> **Round 11 (2026-06-29, v0.6.64→0.6.65)** — verification re-dogfood of the same hardest 3.
> **#443 (algnewcommand), #445 (authblk affil on clean papers), #447 (starred \hspace*/\tag*)
> ALL VERIFIED LANDED** — none reappeared. All 3 still `NEEDS_FIX` (residual hard items below).
> Confirmed: **L1 bug-A is the dominant blocker for 2605.22728** (0.68→0.845 by hand again — the
> `\begin{document}`/dropped-sections/`\begin{align}`-garbage symptoms are all the ERROR-node
> raw-copy region; affil there is inside it, so #445 can't reach it, as predicted).

### M1. Letter/symbol text accents (`\.` `\=` `\v` `\u` `\H` `\r` `\c` `\k`) dropped — sev 3 — ✅ RESOLVED (PR #448, v0.6.65)
- **Symptom (validated):** only `\'`/`\"`/`\^`/`` \` ``/`\~` were handled; the letter/symbol accents
  were dropped, so `TÜB\.{I}TAK` → `TÜBTAK` / `TÜB.ITAK` (dogfood 2605.31499, Turkish affiliation).
- **Fix (#448):** dispatch the family in both the body emitter and the author-block sanitizer;
  precomposed Unicode where it exists (`\.{I}`→İ, `\v{s}`→š, `\c{c}`→ç…) else a combining-mark
  fallback. Single-letter forms guarded against user redefinition; `\v{…}` only parses as an accent
  when brace-delimited (so `\vec` is untouched). 2605.31499 `TÜB.ITAK`→0, `TÜBİTAK`→2.

### M2. IEEEtran `\IEEEauthorrefmark` multi-affiliation author block collapses — sev 4 (major) — ROUTE: Loop A — ✅ RESOLVED (PR #449, v0.6.66)
- **Symptom:** `\IEEEauthorrefmark{n}` multi-affiliation author blocks collapsed to a single
  affiliation `[1]`, `X and Y` merged into one name, the affiliation legend dropped (2605.31499).
- **Fix (#449):** new `parse_ieee_refmark_authors` (class_map.rs) detects the inline-refmark form
  (`\IEEEauthorrefmark` present, no `\IEEEauthorblockN`), splits the author row on `,`/` and `,
  captures each name's refmark superscripts, parses the `\IEEEauthorrefmark{n}<affil>` legend, and
  attaches each author's PRIMARY (first) mark — so authors sharing an affiliation share a
  superscript. 2605.31499: 6 authors split correctly with 3 distinct affiliations (was all `[1]`).
- **Residual (documented limitation):** a secondary-only affiliation mark (an index that is never
  any author's *first* mark) isn't shown — the single-affiliation `Author` model can't hold a second
  one. Emails also not attached (dropped, not leaked). Both low-value; revisit only if re-flagged.

### M3. IEEEtran page density (8 vs 6 pp) + NeurIPS density — sev 2 — DEFERRED (same class as H2)
- IEEEtran conference / NeurIPS render looser than truth; naive margin tightening REGRESSES (font
  metrics differ) — same deferral logic as H2. Needs a real per-class StyleProfile density pass, not
  a skill number. Low priority until the content-leak class is fully cleared.

> **Round 10 (2026-06-28, v0.6.61)** — dogfood of the hardest 3 (`2605.22821`,
> `2605.22728`, `2605.31499`) after the math-font-decl fix (PR #442). All 3
> `NEEDS_FIX`. **L1 is a major, validated structural converter bug** (worst pick).

### L1. `2605.22728` — preamble→body parse breakdown drops 11/12 section headings + leaks `\begin{document}`/affiliation/`\begin{align}` — sev 5 (blocker) — ROUTE: Loop A — VALIDATED on fresh main
- **Symptom (validated on fresh main, v0.6.61):** source has **12** `\section`/`\subsection`/
  `\subsubsection` commands; the `.typ` emits only **1** heading. `\begin{document}` leaks as
  literal text (line 146), the affiliation block leaks `\[1\]…\[2\]…` (line 158), and an
  `align` body leaks raw (`z \{\begin{aligned}`, line 210-211). `\hspace{-0.1mm}` also leaks as
  literal `"hspace*"` / `\hspace{…}` text throughout. Compiles clean (invisible to the compile
  gate) but renders garbage — corpus's **lowest fidelity (0.685)**; the dogfood agent recovered it
  to **0.845 (+0.16)** by hand, proving the loss is real and recoverable.
- **This is the long-open F5 residual** ("`\begin{document}`+affiliation leak (2605.22728), and
  the pre-existing tree-sitter over-attachment where a `{…}`-led paragraph after a no-output
  command is swallowed"). The dropped-headings scale (11/12) is newly quantified.
- **Likely root cause:** the author/affiliation/`\begin{document}` region near the preamble→body
  boundary breaks tree-sitter attachment, and a large following span (incl. most `\section`s) is
  mis-attached/skipped. Investigate the node tree around `\begin{document}` + the custom author
  block; the `\section` commands warn as `unsupported_command` only because they're swallowed into
  a mis-parsed region, not because `\section` itself is unhandled (it is, elsewhere).
- **Highest-value next pick** — single paper but the deepest fidelity hole in the corpus; a real
  parse bug, not a missing recipe.
- **Bug B (authblk `\affil[n]{body}` leak) — ✅ RESOLVED (PR #445, v0.6.63):** the affil/email/orcid
  family now byte-scans the optional `[n]`+`{body}` and skips it; 2605.22724/.31394/.31009 affil
  leak → 0. (In 2605.22728 the affil sits inside bug-A's ERROR region so its affil leak persists
  until bug-A is fixed.)
- **Bug A — TRUE ROOT CAUSE (tick-7 deep-dive, 2026-06-29; SUPERSEDES the tick-2 hypothesis):**
  the tick-2 "preamble × `\setlength` × `\section`" bisect was a TEST-HARNESS ARTIFACT — `echo
  '\end{document}'` in the reproducer mangled `\e`→ESC (byte 0x1B), producing an *unclosed* document
  in every probe (hence every probe "leaked"). The `\setlength` block is innocent (clean
  printf-based reproducers don't leak). **Real cause:** tree-sitter-latex fails to form a
  `document_environment` for this complex math-heavy file → the PARSE ROOT itself becomes one big
  ERROR node (340 nested errors). The emitter's default walk DOES recurse into the root-ERROR's
  children (most content parses into correct `generic_environment`/`generic_command`/`theorem_definition`
  sub-nodes), but the **gaps between recognized children are raw-copied by `safe_copy`** — that's
  what leaks `\begin{document}` (never paired with `\end{document}`) and brace-strips section commands
  (`\section{Introduction}`→`\sectionIntroduction`). The scattered ERRORs are single stray `}` nodes.
  **Dominant contributor = UNDERSCORES:** replacing every `_` in the file (math subscripts, labels,
  refs) jumps recovered headings 1→8 (removing `\label` alone or refs alone does nothing). It is
  multi-causal though — begin-leak persists and only 8/12 headings recover even with underscores
  gone. **This is a parser-robustness problem → lowering-IR Phase D (parser-swap) or a dedicated
  tree-sitter underscore-normalization / emitter ERROR-gap-recovery effort, NOT a one-tick fix.**
  Candidate sub-fixes (each its own scoped tick): (a) never `safe_copy` a leaked `\begin{document}`/
  `\end{document}` marker (always-correct, removes the worst visible garbage); (b) broaden the IR
  underscore normalization beyond labels; (c) emitter ERROR-gap recovery that converts (not raw-copies)
  recognized commands in gaps.
- **MECHANISM PINNED (tick-7; user chose underscore-normalization):** the breaking underscores are
  in **cross-ref/cite/label command KEYS** — `\label{eq:rof_dual}`, `\eqref{eq:rof_optimality.1}`,
  `\cref{…}`, `\cite{…_…}`. A preprocess replacing `_`→`X` *only inside* these keys recovers headings
  **1→8** (identical to replacing ALL `_`; replacing whole `\label`, or refs alone, does NOT).
  tree-sitter mis-reads the key `_` as a subscript, cascading into the document-env parse failure.
  **This happens DURING parse, so the post-parse IR `normalize_truncated_labels` (PR #440, `\label`
  only) cannot un-break it** — confirmed: removing `\label` post-hoc doesn't recover sections.
- **✅ RESOLVED — underscore-key normalization (PR #452, v0.6.67):** `ir::neutralize_ref_key_underscores`
  pre-parse substitutes `_`→`\u{1f}` (same-byte-length sentinel) inside `\label|\ref|eqref|cref|Cref|
  autoref|pageref|labelcref|nameref|namecref|cpageref|vref|crefrange|Crefrange` brace keys; `self.src`
  is the modified source; `sanitize_label_key` restores `\u{1f}`→`_` (the central choke point both
  `<def>` and `@ref` AND the dangling-anchor dedup pass share → one consistent keyspace), plus a final
  output restore as a safety net. `\cite` keys + math subscripts untouched. **GOTCHA hit (caught by
  acceptance):** the first cut restored ONLY in the final output → `referenced_labels` (sentinel form)
  and emitted `<key>` (sentinel form) vs the dangling-anchor scan compared inconsistently → a defined
  label looked "missing" → DUPLICATE phantom anchor → 18 compile regressions. Fix: restore inside
  `sanitize_label_key` so ALL key comparisons (`dangling_ref_anchors`, figures.rs label checks) run in
  `_`-space. **Impact: corpus-wide** — 2605.22728 1→8 headings; ~30 papers gained complete captions /
  resolved `\ref`s / emitted label anchors the misparse had silently corrupted. acceptance 68/0,
  fidelity 0.833 (no regression).
- **✅ RESOLVED — sub-fix (a) leaked `\begin{document}`/`\end{document}` drop (PR #453, v0.6.68):**
  a loose `begin`/`end` `document` node (only produced when the document env fails to form) is now
  dropped instead of raw-copied; verbatim listings (string tokens, not `begin` nodes) preserved.
  2605.22728/.22786/.31203 lose the stray marker.
- **L1 bug-A STATUS: largely closed.** The two highest-impact pieces (underscore-key parse recovery
  #452, begin/end-document leak #453) shipped. RESIDUAL (lower value, deferred): 2605.22728 still
  recovers only ~8/12 sections — the remaining gap is the deeper *unisolated* cumulative parse
  interaction (synthetic underscore/align docs don't reproduce it); fully closing it is parser-swap
  (lowering-IR Phase D) territory, not worth a targeted tick. Revisit only if a dogfood re-flags it.

### L2. `\algnewcommand` macro-definition body leaks into the document body — sev 4 (major) — ROUTE: Loop A — ✅ RESOLVED (PR #443, v0.6.62)
- **Symptom (validated):** `\algnewcommand{\LeftComment}[1]{\Statex \(\triangleright\) #1}`
  (algorithmicx) — the macro *definition's body* leaked into the document as
  `\[1\] $gt.tri$ \#1`. `\algnewcommand`/`\algrenewcommand` weren't recognized as
  macro-definition forms (tree-sitter parses them as bare `generic_command`, like `\newcommandx`),
  so the prepass didn't consume the `{name}[n]{body}` and the body tokens leaked.
- **Fix (PR #443):** new `extract_algnewcommand_and_end` handles both `\algnewcommand{\name}[N]{body}`
  (braced name — scan from the `command_name` token's end, since tree-sitter absorbs the `{\name}`
  curly group) and bare `\algnewcommand\name{body}`; wired into the main prepass, the `\input`
  harvest, and the emit-time skip. **Verified deterministically:** 2605.31499 leak fragment
  `\[1\] $gt.tri$ \#1` → 0 occurrences; 3 TDD tests + 3 edge cases; gates green.

### L3. `byetex-tables-layout` lacks NeurIPS-specific density numbers — sev 3 (major skill note) — ROUTE: Loop B (2605.22821, recurring)
- **Symptom:** the skill correctly says NeurIPS/ICML/ICLR are single-column and to check
  `#set par()`/`#set text()` for density, but gives **no concrete values** — the agent had to read
  `neurips_2026.sty` to derive 5.5in textwidth → 1.5in side margins and 10/11pt leading. Recurs
  across rounds (H2). Add a NeurIPS/ICML density table (margins, text size, leading) to the skill.
- **Out-of-scope gave-ups (record, no fix):** the same run gave up on an EPS figure (no in-sandbox
  raster tool) and a `minted`-in-`subfigure` code block (needs tectonic) — both genuinely need
  external tooling; revisit only if frequent.

### L4. `byetex-getting-started` vs sandbox instruction conflict on `byetex diagnose` — sev 3 (major skill note) — ROUTE: Loop B (recurring, 2 papers)
- **Symptom:** getting-started says "`byetex diagnose paper.typ` re-scans an already-edited `.typ`
  in place (edits preserved)" but the dogfood sandbox procedure says "do NOT run `byetex diagnose`
  at all (it wipes edits)". Agents (2605.22728, 2605.22821) couldn't tell whether the in-place
  `.typ` form was safe. The prohibition only applies to `diagnose paper.tex` (re-convert wipes);
  the `.typ` form is safe. Clarify the distinction prominently in both surfaces. Recurring
  `missing_tool_wishlist`: a leaked-LaTeX fidelity scanner (`diagnose --scan-latex-leakage`)
  overlaps F4's "`warnings --fidelity` leak scanner" — would directly catch L1/L2-class leaks.

> **Round 8 (2026-06-24, v0.6.11)** — verification re-dogfood of `2605.22821`
> after the H1/H3 fixes. **H1 (#378), H3-expl3 (#379), H3-colour (#381) ALL
> VERIFIED LANDED**: the agent made ZERO mention of `langledo` / expl3 internals /
> `black#2`/`ForestGreen#2` this round (all were round-7 stuck points). New blocker
> J1 below; verdict still NEEDS_FIX (the two-column attempt — a REPEAT misdiagnosis,
> NeurIPS is single-column — plus an EPS figure dropped fidelity 0.78→0.776).

> **Round 9 (2026-06-24, v0.6.12)** — re-dogfood of `2605.22821` to verify J1.
> **J1 VERIFIED LANDED**: the agent read the rewritten skill, made NO two-column
> attempt, and correctly concluded NeurIPS is single-column ("no layout change
> needed"; was a `gave_up` blocker in round-8). It also attributed the 34-vs-26pp
> gap to the truth's 2-col submission format vs the preprint's 1-col — confirming
> H2 is NOT a converter bug. New item K1 below.

### K1. `\operatorname*{X}` (starred) drops its argument — sev 3 — ✅ RESOLVED (PR #385 starred + #386 inner-unwrap)
- **Symptom (validated on fresh main):** `\operatorname{argmin}` → `op("argmin")` ✓, but the
  STARRED `\operatorname*{argmin}` → the bare string `operatorname*` with the `{argmin}`
  argument DROPPED. Compiles, renders wrong (invisible to the compile gate).
- **Fix:** PR #385 dispatches `\operatorname*` → `op("…", limits: #true)`. Follow-up PR #386:
  `\operatorname{\mathrm{X}}` was emitting `op("\mathrm{argmin}")` (rendered the literal
  `\mathrm{argmin}` — `op()` quotes its arg verbatim); now a redundant `\mathrm`/`\text`/`\mbox`
  wrapper is unwrapped to `op("argmin")` (`unwrap_upright_wrapper`). 5 corpus papers; both verified.
- **NOT a bug (validated, do not chase):** the agent also reported `\bpe`
  (`\newcommand{\bpe}{\texttt{BPE}\xspace}`) leaking — does NOT reproduce on fresh main
  (`\bpe` → `#raw("BPE")` correctly). False finding.

### J1. `byetex-tables-layout` skill teaches a STALE two-column recipe — sev 5 (blocker) — ✅ RESOLVED (PR #383, verified round-9)
- **Symptom:** the skill (line 43) says "Two-column classes render the body wrapped in
  `#columns(2)[...]`" + "wrap a wide figure/table in `#place(...)`" with NO spanning syntax.
  The round-8 agent tried to manually two-column a NeurIPS paper, all 15 wide tables
  overflowed, gave up (blocker). **This is the loop's ORIGINAL never-done Loop-B item** (the
  first dry-run finding, pre-tick-1).
- **Why stale:** PR #247 ([[project-two-column-layout]]) replaced `#columns(2)[body]` (which
  blew a figure-heavy paper to 81pp) with page-level `#set page(columns: 2)` +
  `#place(scope: "parent", float: true)` spanning floats, AUTO-detected per DocClass
  (ACL/IEEEtran). The skill never caught up.
- **Fix:** rewrite the skill's Page-layout section — (a) the converter AUTO-emits page-level
  `#set page(columns: 2)`; agents must NOT manually wrap in `#columns(2)`; (b) starred floats
  span via `#place(scope: "parent", float: true)` (give the syntax); (c) **NeurIPS/ICML are
  SINGLE-column** — do not add columns (agent misdiagnosed twice; see H2). Skill-only → verify
  by re-dogfood.

> **Round 7 (2026-06-24, v0.6.8)** — first dogfood after PR #376 unblocked
> `select` (it had been returning only un-scoreable `truth_render_failed` books).
> Re-dogfooded the now-measurable hardest paper `2605.22821` (NeurIPS,
> word_recall 0.746). Verdict NEEDS_FIX, fidelity 0.78→0.78 (the page-density gap
> dominates and the agent couldn't fix it). Findings below, validated on a fresh
> `main` conversion.

### H1. Custom macro expanding to `\langle#1` concatenates into garbage (`langledo`) — sev 4 (major) — ✅ RESOLVED (PR #378, verified round-8)
- **Symptom:** `\newcommand{\tokenstring}[1]{...\langle#1\rangle}` used as
  `\tokenstring{do,g}` renders as the math identifier `langledo` (and `langleab`,
  `langlebc`, … — confirmed on fresh main: `grep -o 'langle[a-z]*'` → 8 variants).
  The macro-expansion buffer glues `\langle` to the following argument text with no
  token boundary, so the math symbol lookup never fires. Also the ROOT CAUSE of the
  garbled `ambiguous_math` snippets seen while investigating M1 (tick-52).
- **Fix sketch:** when expanding a macro body, a control word (`\langle`) followed by
  a parameter substitution must keep a token boundary (the LaTeX tokenizer ends
  `\langle` at the non-letter `#`/arg). Likely in the macro-expansion path
  (package_macros / expand) — ensure `\<letters>` tokens terminate before substituted
  arg text. **Highest-value next pick** — clean, generalizable, reproduces trivially.

### H2. NeurIPS page geometry not applied — sev 4 — ⚠️ DEFERRED (agent MISDIAGNOSIS; geometry real but fixing it alone REGRESSES the metric)
- **Symptom (as filed):** `2605.22821` emits `#set page(margin: (x: 1in, y: 1in))` not
  NeurIPS density (textwidth 5.5in → 1.5in side margins; textheight 9in/top 1in → 1in
  top/bottom; 10pt). DocClass::Neurips IS detected (`\usepackage[preprint]{neurips_2026}`)
  and already emits us-letter + 10pt; only the MARGINS are generic.
- **Tick-54 investigation (why deferred):** the agent's "2-column not detected → page
  inflation" is DOUBLY wrong — NeurIPS is SINGLE-column, and the page inflation is NOT
  geometry. Measured: byetex **34pp vs truth 26pp** (agent's "20" was also wrong) DESPITE
  a *wider* 6.5in text block. Narrowing to the correct 5.5in NeurIPS margins would *add*
  lines → MORE byetex pages → page_ratio WORSE. The real driver is leaked/over-rendered
  CONTENT (expl3 H3, colour residue, figure placeholders). **Correct order: fix the content
  leaks first; only then is the NeurIPS margin fix a net win.** Margin geometry is real but
  low-value until the content over-pagination is resolved.

### H3. expl3 helper macro + colour wrapper-newcommand leak — sev 3 — ✅ RESOLVED (PR #379 expl3 + #381 colour, verified round-8)
- **Symptom:** `\NewDocumentCommand{\AppendToList}{m}{ \clist_map_inline:nn … }` defined
  inside `\ExplSyntaxOn…Off` is harvested by the prepass (the region is skipped only for
  emission), so *calling* it after `\ExplSyntaxOff` spliced its pure-expl3 body
  (`\seq_gput_right:Nx`, `\tl_to_str:n`, + the arg) into the body as garbage (2605.22821).
- **Fix (#379):** `expand_user_macro` now detects an expl3 body via the `\name:argspec`
  signature (`body_is_expl3`) and DROPS the whole call + args with a warning (expl3 produces
  no document output). 2605.22821 expl3 leaks → 0. 2 TDD tests.
- **Residual (separate issue, follow-up):** colour META-macro residue `ForestGreen#2` /
  `purple#2` from nested `\newcommand{\m}[2]{\newcommand{#1}{{\color{…}#2}}}` (a
  `\newcommand` that defines a `\newcommand` with `#2`) still leaks — different root cause.

### H4. `byetex-using-warnings-json` doesn't distinguish preamble-only vs body drops — sev 3 (major skill note) — ROUTE: Loop B
- **Symptom:** when expl3 preamble code (`\clist`/`\seq`) is dropped AND leaks into the
  body, the warning routes to `byetex-using-warnings-json` which only explains the schema
  — "no guidance when preamble code leaks into body output." Pairs with H3 (the converter
  fix) — once H3 drops the residue, this is moot; if not, add a preamble-leak note.
- **NOTE (false alarm):** the round-7 agent also wished for "diagnose --incremental" and
  said getting-started "says not to run byetex diagnose at all" — VERIFIED a MISREAD;
  both getting-started and repair-loop cleanly say `byetex diagnose paper.typ`. F6 holds.

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

### F11. More deprecated Typst symbols in math — minor — ✅ RESOLVED (PR #374 + #375)
- Swept ALL Typst-0.14 math-symbol deprecations: `\otimes`/`\oplus`/`\ominus`/`\odot`/
  `\oslash` → `times.o`/`plus.o`/`minus.o`/`dot.o`/`slash.o` (#374; `slash.circle` was an
  invalid modifier), and `\llbracket`/`\rrbracket` → `bracket.l.stroked`/`bracket.r.stroked`
  (#375). An audit of all 366 emitted symbols vs the typst 0.14.2 compiler now reports
  ZERO deprecations. `angle.l/.r` → `chevron.l/.r` was already done (#280).

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
