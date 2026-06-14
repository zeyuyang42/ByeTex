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
- **Fidelity gate (render quality, the DRIVER):** before a release, run `./scripts/fidelity_gate.sh` — it renders the corpus via `scripts/visual_test.py` and fails (exit 1) if the corpus `fidelity_score` or a paper's `word_recall` regresses vs `scripts/fidelity_baseline.json`. Promote with `./scripts/fidelity_gate.sh --update-baseline` when a change legitimately improves fidelity. For vision-graded regressions, `scripts/findings_diff.py` diffs the `byetex-visual-grading` findings vs a committed set. Also a manual `fidelity` CI job (Actions → Run workflow). Compile is the gate; fidelity is the driver — see `docs/scorecard.md`.
