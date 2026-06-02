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

- Use `./scripts/corpus_sweep.sh` to verify corpus pass-rate after fixes.
- The script uses `byetex convert --project` to regenerate full projects including bib preprocessing.
- **Acceptance gate:** run `BYETEX_BIN=<your binary> ./scripts/acceptance.sh` before merging — it fails (exit 1) if a known-passing paper regresses to BYETEX_FAIL (baseline: `scripts/acceptance_baseline.json`). When a fix flips a paper, promote it from `known_fail` to `known_pass` in that baseline.
