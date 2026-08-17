# ByeTex Project — Claude Instructions

## Workflow Defaults

- **Always use a git worktree** when making code changes. Create a new worktree per bug/feature rather than working on the main branch directly. Use `git worktree add -b <branch-name> <path>`.
- **Always open a PR immediately** after fixing a bug or completing a feature. Do not accumulate multiple fixes on one branch without a PR.

## Testing

- Follow strict TDD: write a failing test first, watch it fail, then implement.
- Run `cargo test --workspace` before claiming any fix is complete.

## Scripts

- Use `uv run --with <pkg>` instead of `pip install` (PEP 668 blocks system Python).

## Corpus Sweep

- **Layout:** one dir per arXiv id. `corpus/<id>/source/` holds pristine inputs (tex, figures, `.bib`, `00README.json`) plus `source.tar.gz`; all generated artifacts go to the sibling `corpus/_out/<id>/`. `corpus/manifest.json` is the only committed file (the rest is gitignored). Reset a messy corpus with `./scripts/corpus_clean.sh` (idempotent; `--purge-out` also wipes `_out/`, `--dry-run` previews).
- Use `./scripts/corpus_sweep.sh` to verify corpus pass-rate after fixes.
- The script uses `byetex convert --project` to regenerate full projects including bib preprocessing.
- **Acceptance gate:** run `BYETEX_BIN=<your binary> ./scripts/acceptance.sh` before merging — it fails (exit 1) if a known-passing paper regresses to BYETEX_FAIL (baseline: `scripts/acceptance_baseline.json`). When a fix flips a paper, promote it from `known_fail` to `known_pass` in that baseline.
- **Fidelity gate (render quality, the DRIVER):** before a release, run `./scripts/fidelity_gate.sh --all` — it renders the corpus via `scripts/visual_test.py` and fails (exit 1) if the corpus `fidelity_score` or a paper's `word_recall` regresses vs `scripts/fidelity_baseline.json`. **`--all` is not optional for a release gate:** without it `visual_test.py` measures only the 5 PINNED papers, which is 7% of a 71-paper baseline, and the corpus-score comparison is then skipped entirely because the two populations differ. The bare command is for quick iteration; it prints a PARTIAL RUN warning naming what it skipped. Promote with `./scripts/fidelity_gate.sh --update-baseline` when a change legitimately improves fidelity. **Layout tier:** the gate also measures WHERE text sits — margins, columns, font tiers, leading, ink (`scripts/layout_metrics.py`). It needs `--with pymupdf` (dev-only, AGPL, never in `requirements.txt`); without it every `layout_*` property is silently `None`, which looks exactly like a clean corpus, so heed the `LAYOUT BLIND` warning. One property (`layout_body_font_ratio`) can fail a build; the rest are report-only. Thresholds live in `scripts/layout_floors.json` and are **measured, not chosen** — change them only by explicit PR (`scripts/layout_loop.py floors`; see `docs/layout-floors-2026-08-17.md`). Rank the worst-drifted papers with `scripts/layout_loop.py rank --n 5 --json`. For vision-graded regressions, `scripts/findings_diff.py` diffs the `byetex-visual-grading` findings vs a committed set. Also a manual `fidelity` CI job (Actions → Run workflow). Compile is the gate; fidelity is the driver — see `docs/scorecard.md`.
