# Autonomous-dev loop (operator's manual)

This is the playbook the **orchestrator** (a Claude Code main loop, Opus) follows to
keep ByeTex improving on its own: *higher conversion fidelity, fewer corner-case
warnings, robust tests* — while continuously proving the **agent surface** (skills /
CLI) is good enough by dogfooding it with a fresh model.

It is two interlocking loops plus a corpus-expansion gate:

- **Loop A — converter dev** raises the deterministic floor (emitter fixes, fewer
  warnings). Work comes from `scripts/fidelity_audit.py`, fidelity outliers, and items
  Loop B routes here. Done by the orchestrator under strict TDD.
- **Loop B — agent-surface dogfood** raises the reachable ceiling. A **fresh Sonnet**
  agent repairs a seeded conversion using only the byetex surface; where it struggles
  becomes the agent-surface backlog, fixed by improving a skill / tool / diagnostic.
- **Visual grader** (the "visual agent") grades render fidelity after any
  render-affecting change — a check no metric catches.
- **Corpus-expansion gate**: when there's no work left, propose use cases and **ask the
  user** before pulling more data.

## Setup (once per checkout)

```bash
cargo build --release                       # the byetex binary the loop drives
export BYETEX_BIN="$PWD/target/release/byetex"   # see "Which binary" below
bash scripts/sync_agents.sh                 # agents/*.md → .claude/agents/ (gitignored)
bash scripts/setup_truth_deps.sh            # tectonic deps: pinned biber + fonts for the TRUTH render
```

**Which binary gets measured.** `dogfood.py` resolves `$BYETEX_BIN` → the repo's
`target/release/byetex` → **hard error**; `PATH` is never searched. Every verb prints the
resolved path + `--version` on stderr and warns on two kinds of staleness: a version that
disagrees with `Cargo.toml`, and (the common one) a binary **older than `crates/**/*.rs`**
— an unbuilt working-tree edit, where the version still matches.

This is not paranoia. It *used* to fall through to `byetex` on PATH, picked up the
install.sh copy in `~/.local/bin` (**v0.3.0**, ~40 releases old), and seeded a whole
dogfood round from it — producing `blocker`-severity findings against skills that were
describing the current converter perfectly accurately, plus a skewed `fidelity_before`
and hardest-3 ranking. Note the *agent* half is the other exposure: it reaches skills via
`$BYETEX skills read`, and **skills are served by the binary**, so step 6 must hand it an
absolute path (`dogfood.py byetex-path`) rather than let it call a bare `byetex`.

**Rebuild before dogfooding, and if a report claims "the converter dropped X", re-run that
construct through the current binary before routing it.** (`visual_test.py` / the fidelity
gate build the repo binary themselves, so they were never exposed to this.)

**Truth-first rule.** The fidelity DRIVER compares ByeTex output against the *truth* — the
paper's original LaTeX rendered with tectonic. `scripts/setup_truth_deps.sh` provisions what
that render needs (a biber pinned to tectonic's biblatex, plus fonts like Roboto Slab / Carlito
that bespoke thesis classes require) into a gitignored `.truth-deps/`. Without it, font/biber
papers silently land in `truth_render_failed` and the driver goes blind on them. **Never accept a
corpus paper without a verified truth render** — fix the dep or record why it can't build; never
let it become a silent unmeasured "pass."

`.claude/agents/` is gitignored, so the canonical agent defs live in `agents/` and are
synced in by `scripts/sync_agents.sh`. The Agent tool resolves `subagent_type` from
`.claude/agents/`.

> **Important:** Claude Code loads `.claude/agents/` **at session start**. Run
> `scripts/sync_agents.sh` *before* you open the `/loop` session — if you add or change
> an agent def mid-session, restart the session (or the dogfood `subagent_type` won't
> resolve). As a fallback the orchestrator can dispatch a built-in `general-purpose`
> agent on `model: sonnet` with the `agents/byetex-dogfood-tester.md` body inlined in the
> prompt — the sandbox isolation (no converter internals present) keeps the test honest
> either way.

**Optional — reduce permission prompts.** A live `/loop` runs many commands; to avoid
approving each, add these to `.claude/settings.local.json` `permissions.allow` (the
loop edits no permissions itself — this is your opt-in):

```jsonc
"Bash(uv run *)",                 // dogfood.py / fidelity_audit.py / visual_test.py
"Bash(git worktree *)",           // one worktree per fix
"Bash(bash scripts/sync_agents.sh)",
"Bash(./scripts/acceptance.sh *)", "Bash(./scripts/fidelity_gate.sh *)",
"Bash(./scripts/corpus_sweep.sh *)",
"Bash(./target/release/byetex *)" // all byetex verbs (not just convert)
// cargo *, gh pr *, git add/commit/push are typically already allowed
```

## Start the loop

The driver is a **live `/loop` session**. Start it with:

> `/loop` — Run the ByeTex autonomous-dev cycle per docs/autonomous-dev.md.

The orchestrator runs one cycle (below), then self-paces the next tick with
`ScheduleWakeup` (lean 1200–1800s between ticks; a tick is heavy, so don't poll
faster). It stops and asks the user only at the corpus-expansion gate or an ambiguous
regression.

---

## One cycle

### 1. Measure
```bash
uv run --with requests --with Pillow python scripts/fidelity_audit.py   # → docs/fidelity-nonvisual-audit.{md,json}
uv run --with pymupdf python scripts/layout_loop.py rank --n 5 --json   # → the worst LAYOUT drift
```
Then read: `docs/fidelity-nonvisual-audit.md` (converter-gap + warning ranks),
`scripts/fidelity_baseline.json` (lowest-fidelity outliers), `scripts/acceptance_baseline.json`
(any `known_fail`), `docs/agent-surface-backlog.md` (open Loop-B items), and
`docs/layout-backlog.jsonl` (open layout items).

`rank` answers a question the other signals cannot: **where the text sits**.
Everything else measures *what* text is present, which is why a paper with a 42%
too-wide text column and halved margins scores `word_recall 0.924` and passes.
Papers whose tier never ran surface as `severity: null, reason: "unmeasured"` —
they are not clean, they are unmeasured, and they include the corpus's worst
geometry.

### 2. Select ONE highest-value item (one item per cycle → one PR)
- An open **agent-surface** item, a **converter-gap / warning-noise** item, or a
  **layout-drift** item from `rank`.
- Prefer items with the widest paper evidence. A converter gap that also blocks the
  dogfood agent is highest value (fixes both loops at once).
- A `drift_class` that ALSO appears as a converter gap in the audit outranks either
  alone — two independent signals pointing at one emitter is the strongest evidence
  available, and the fix closes both.
- Read `severity` as an ordering, not a magnitude. It is the largest excess over a
  property's own floor, and floors differ in kind: some are measured, some are the
  0.005 resolution floor. `page_trim` on a slide deck reaches 757 because its floor
  is tiny, not because it is 8× worse than a paper at 93.

### 3. Fix in a fresh worktree, strict TDD (red test first)
```bash
git worktree add -b fix/<slug> ../ByeTex-<slug>
# fresh worktrees have no corpus payload (gitignored) — symlink it in:
for d in "$PWD"/corpus/*/; do ln -sfn "$d" "../ByeTex-<slug>/corpus/$(basename "$d")"; done
ln -sfn "$PWD/tests/visual" "../ByeTex-<slug>/tests/visual"
```
- **Loop A**: add the failing snapshot/unit test in `crates/`, watch it fail, implement
  the emitter change, watch it pass. To *reduce a warning*, either handle the construct
  or downgrade a genuinely-benign warning (e.g. `\newpage` drop) so the noise drops.
- **Loop B**: edit `skills/<name>/SKILL.md`, add a CLI flag, or enrich
  `diagnostics.json`. Skill-only edits can't change compile/fidelity — they're verified
  by re-dogfooding, not the gates.

### 4. Gate locally (this IS the auto-merge condition)
```bash
cargo test --workspace                                  # green
BYETEX_BIN=../ByeTex-<slug>/target/release/byetex ./scripts/acceptance.sh   # exit 0
./scripts/fidelity_gate.sh                              # exit 0 (no fidelity regression)
```
- Promote a baseline only when a change *legitimately* improves it
  (`./scripts/fidelity_gate.sh --update-baseline`, or edit `acceptance_baseline.json`
  to move a flipped paper `known_fail` → `known_pass`).
- **Render-affecting change** → also run the visual grader (step 6's grader) on the
  affected papers and `python scripts/findings_diff.py` vs the committed findings; a
  vision regression blocks.
- **Skill-only Loop-B change** → re-dogfood the item's evidence papers (step 5); they
  must reach `GOOD_ENOUGH` for the item to be Resolved.

**The layout tier in the gate.** `fidelity_gate.sh` gates one layout property —
`layout_body_font_ratio`, whose threshold is measured and whose already-failing
papers are listed as `known_bad`, so it is green today and fires only on NEW
breakage. Every other layout property prints as a report-only `LAYOUT DRIFT`
block. Two rules:

- Never merge a change that moves a layout property beyond its floor without
  explaining it in the PR body. A report-only notice is information, not permission.
- `scripts/layout_floors.json` changes **only by explicit PR**, never as a side
  effect of a run. An auto-updating floor tracks the regression and is worse than
  no floor. Re-measure with `layout_loop.py floors` after any typst or pymupdf
  upgrade and treat a moved floor as a finding.

If the gate prints `LAYOUT BLIND — T1 … not computed`, the run had no pymupdf and
the geometric tier measured nothing. That is not a clean corpus; re-run with
`uv run --with pymupdf`.

CI is unreliable and **red CI is acceptable** — the *local* gates above are the real
check. Never block on GitHub Actions.

### 4.5. Code-review the diff (every tick, before merge)
Run `/code-review` on the worktree diff (the `code-review` skill, effort `medium` for a
typical one-item fix; `high` for a larger/riskier change). It reviews for correctness
bugs and reuse/simplification cleanups.
- **Triage every finding.** Apply the real ones (re-run the step-4 gates after any
  edit). Genuinely-wrong or out-of-scope findings: note why and skip — do not
  perform-fix. A finding that reveals a real correctness bug is a **merge blocker**:
  fix it (or, if the fix is ambiguous, stop and ask the user) before step 5.
- This is a *quality* gate layered on the *correctness* gates above; it does not replace
  `cargo test` / acceptance / fidelity. Keep it to the diff, not a whole-repo audit.

### 4.6. Version bump for a user-visible change
If the tick's change is **user-visible** — a converter fix, an agent-surface change, a
feature — bump the workspace version in the same PR (`[workspace.package].version` in
`Cargo.toml` **and** the mirrored `byetex-core` path-dep `version =`
strings in each crate's `Cargo.toml`, plus `Cargo.lock`) and add a `CHANGELOG.md` entry
under the current `— unreleased` section.

**Default to a PATCH bump.** Almost every tick (a single converter/skill fix) is a
patch (`0.4.0 → 0.4.1 → 0.4.2 …`). Reserve a **minor** bump for a genuinely new
*capability* (a new CLI verb, a new supported construct class) — and only
once per such capability, not per follow-up fix. Never default to minor. Routine
internal refactors / bookkeeping (`[skip ci]` docs) don't bump. If several patch-worthy
fixes land before a release, each can bump its own patch, or group them under one
`— unreleased` section — judgment call, but when unsure, patch.

### 5. Open the PR and auto-merge on green
```bash
gh pr create --fill --base main
gh pr merge --squash --admin    # local gates green ⇒ merge (bypasses unreliable CI)
```
Commit/PR trailers per `CLAUDE.md`. If any gate failed and the fix is ambiguous, **stop
and ask the user** instead of merging.

### 6. Dogfood the hardest 3 (Loop B instrumentation)
```bash
uv run --with requests --with Pillow --with numpy --with scikit-image \
    python scripts/dogfood.py select --n 3 --json
```
For each id:
```bash
SB=$(uv run --with requests --with Pillow --with numpy --with scikit-image \
        python scripts/dogfood.py prepare <id>)     # last stdout line = sandbox path
BYETEX=$(python scripts/dogfood.py byetex-path)     # NEVER let the agent use bare `byetex`
```
Then spawn the **fresh Sonnet** tester via the **Agent tool**:
- `subagent_type: "byetex-dogfood-tester"`, `model: "sonnet"`
- `prompt`: `SANDBOX=<$SB>  PAPER_ID=<id>  BYETEX=<$BYETEX>` + "cd to SANDBOX, follow
  your procedure, emit the dogfood report JSON as the last fenced block."

**`BYETEX` must be an absolute path, and must never be empty.** The agent reaches skills
via `$BYETEX skills read`, and **skills are served by the binary** — so a bare `byetex`
resolved from the sandbox's `PATH` can hand a fresh agent an ancient surface. Deriving it
from `dogfood.py byetex-path` (rather than from `$BYETEX_BIN`, which may be unset) keeps
both halves of the loop — harness and agent — on the same binary.

Save the agent's final JSON block, then score it:
```bash
printf '%s' "<agent-final-json>" > "$SB/.dogfood/report.json"
uv run --with requests --with Pillow --with numpy --with scikit-image \
    python scripts/dogfood.py score "$SB" --report "$SB/.dogfood/report.json"
```
`score` recompiles the agent-edited `main.typ`, computes `fidelity_after`, cross-checks
the self-report, appends a record to `docs/agent-surface-backlog.jsonl`, and prints
`GOOD_ENOUGH` | `NEEDS_FIX`. Curate every `NEEDS_FIX` into `docs/agent-surface-backlog.md`
using the routing rubric.

### 6.5. Route a layout item (Loop A, geometry)

```bash
uv run --with pymupdf python scripts/layout_loop.py rank --n 5 --json
REC=$(uv run --with pymupdf python scripts/layout_loop.py record <paper_id> | tail -1)
uv run --with pymupdf python scripts/layout_loop.py triage <paper_id> --record "$REC" \
    --run-ts "$(date -u +%Y%m%dT%H%M%SZ)" --note "<what you concluded>"
```

`record` recomputes the tier for one paper from artifacts already on disk — a fast
recheck after an emitter fix, with no corpus run.

There are three layout tiers, not four. A fourth — anchor drift via SyncTeX, i.e.
where a `\label`ed element *lands* — was measured and rejected: tectonic 0.16.9
emits **one** SyncTeX box for a document with three labels, and 88% of its file
entries are unnamed, so essentially every label is unattributable. See
`docs/layout-t3-synctex-not-viable.md`, including the content-anchored design that
would work instead. `triage` appends a routed record
to `docs/layout-backlog.jsonl` and prints a bare `DRIFT` | `WITHIN_FLOOR`.

The record carries `drift_class`, `suspected_site` and `routing_reason`. Treat the
site as a **lead, not a diagnosis** — confirm it against the generated `.typ`
before writing a test.

### 7. Loop or stop
- Backlog non-empty → next tick (`ScheduleWakeup`).
- **All THREE backlogs empty AND the hardest 3 all `GOOD_ENOUGH`** → there is no work
  (`docs/agent-surface-backlog.md`, the converter-gap audit, and
  `docs/layout-backlog.jsonl` / a clean `rank`):
  propose the next **use cases / document types** to harden (e.g. beamer decks, CVs,
  books/theses per `docs/tier1-baseline-2026-06-15.md`) and **ASK the user before
  pulling more data** (`corpus_harvest.py --search` / `corpus_add_local.py`). Never
  expand the corpus unprompted.
- Three quiet ticks in a row → scale back to a quick gate check and report.
- **Self-audit:** if the last three ticks routed ZERO layout items, the tier is
  wallpaper. Either promote a property to a hard gate (`--gate-layout`, backed by
  measured floors and by manually confirming the papers it flags) or delete the
  report-only notice block. A signal nobody acts on is worse than no signal — it
  costs attention and buys the appearance of coverage.

---

## Routing rubric (friction signal → fix site)

| Signal in the dogfood report | Route |
|---|---|
| `typst compile` error, no `skill_name`, construct the converter *should* handle deterministically | **Loop A** (emitter bug; TDD + gates) |
| Skill offered but lacked the recipe / `unclear_skill_notes` severity `blocker`\|`major` | **Loop B** (improve that skill) |
| `missing_tool_wishlist` item recurring across ≥2 papers | **Loop B** (add CLI flag / diagnostic field) |
| `self_report_mismatch` (agent thought it compiled, it didn't) | **Loop B** (misleading error/diagnostic message) |
| `compiled` but low `fidelity_after`, **no** stuck point (agent never noticed) | **Loop B** (surface didn't *surface* it — strengthen `warnings.json` / grading) |
| Genuinely out-of-scope construct, agent escaped correctly (e.g. image-fallback for TikZ) | **No fix** (record; revisit only if frequent) |
| Tie (same construct → Loop A on one paper, Loop B on another) | **Loop A** — fix once deterministically beats teaching the agent N times |

### Layout drift (`drift_class` → fix site)

From `scripts/layout_loop.py DRIFT_CLASS_ROUTES`. Each row is a **lead**, not a
diagnosis — confirm against the generated `.typ` first.

| `drift_class` | Route | Why there |
|---|---|---|
| `page_trim` | `emit/preamble.rs`, `style_profile.rs` | page size comes from class options + the neutral preamble |
| `margin` | `style_profile.rs`, `emit/preamble.rs` | margins are set per DocClass in `#set page(...)` |
| `text_width` | `style_profile.rs`, `emit/preamble.rs` | measure = page size − margins; check both |
| `font_size` | `class_map.rs`, `style_profile.rs` | the `10pt`/`11pt`/`12pt` class option resolves to the body size |
| `type_scale` | `style_profile.rs`, `emit/typography.rs` | heading and body sizes are emitted as a scale |
| `small_tier` | `emit/typography.rs` | `\small` / `\footnotesize` / `\scriptsize` switches |
| `leading` | `emit/preamble.rs` | `#set par(leading:)` vs the class's baselineskip |
| `density` | `emit/preamble.rs`, `style_profile.rs` | lines/page follows leading + margins; fix those first |
| `columns` | `emit/preamble.rs`, `style_profile.rs` | `#set page(columns: 2)` + the spanning-title float |
| `ink` | `emit/figures.rs`, `emit/tables.rs` | ink density is dominated by dropped/resized floats |
| `anchors` | `ir.rs`, `emit/sections.rs` | `\label` keys normalised pre-parse, emitted as `<key>` |
| `ordering` | `emit.rs`, `emit/macros.rs` | dropped/duplicated content — check `\input` + unsupported commands |

## Verdict rule (`scripts/dogfood.py score`, per paper)

`GOOD_ENOUGH` iff: `compiled` AND no `self_report_mismatch` AND no stuck point with
`workaround`/`gave_up` AND no `blocker`/`major` skill gap AND `fidelity_after ≥
fidelity_before` AND `fidelity_after ≥ 0.75` (floor, tunable). Else `NEEDS_FIX`. A clean
compile reached only via a *workaround* is still `NEEDS_FIX` — that's the gap to close.

## Anti-thrash

- The dogfood agent self-limits (≤12 compiles, stop an error after 2 no-progress, ≤4
  fidelity-polish iterations). Set a timeout on the Agent tool call; a hung run still
  yields a backlog entry from `score`.
- Each fix **cites the backlog item id it closes** and **re-dogfoods the same evidence
  papers twice** before the item is marked Resolved (Sonnet variance control).
- One item per PR; the acceptance + fidelity gates are the regression backstop.

## Division of labor

**Orchestrator (Claude), each tick:** measure → pick one item → TDD fix in a worktree →
local gates → **`/code-review` the diff and apply real findings** → auto-merge on green →
dogfood the hardest 3 with a fresh Sonnet agent → route friction into the backlog → run
the visual grader on render changes → self-pace the next tick. Keeps `cargo test`,
acceptance, and fidelity green; one PR per fix.

**User:** start the loop (`/loop`); answer the corpus-expansion gate (which use cases to
add, approve pulling data); adjudicate ambiguous gate regressions and legitimate
re-baselining; keep the session alive; spot-review merged PRs post-hoc.
