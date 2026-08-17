---
name: byetex-workspace-cleanup
description: Reclaim disk in the ByeTex repo by clearing out build and render artifacts left over from a dev session — git worktrees, target/debug, cross-compile targets, tests/visual rasters and composites, corpus/_out, tmp/, stale review scratch, dist/, and merged branches. Use this whenever the user asks to clean up, free disk, tidy the workspace, remove worktrees, or says a session is finished and wants the leftovers gone — and proactively at the end of a long session that created worktrees or ran corpus/fidelity sweeps, since those routinely leave 20G+ behind. Never deletes: it moves candidates to a .cleanup-trash/ folder and asks the user to empty it themselves. Always presents an audit and waits for explicit confirmation first.
---

# ByeTex workspace cleanup

A long session in this repo leaves a lot behind: a worktree per PR (each with its
own `target/` and often its own 1.5G copy of `tests/visual`), debug builds, page
rasters, generated conversions. The session this skill came out of had grown the
workspace to ~31G, of which ~30G was reclaimable.

The job is to give that space back **without ever destroying something that cannot
be regenerated**. Two things in this repo look like caches and are not:

- **`corpus/<id>/source/`** — the pristine paper inputs. Most are re-downloadable
  from arXiv, but some are `"source": "local"`: hand-authored, gitignored, and
  gone forever if deleted. One (`beamer-demo`) was lost exactly this way and only
  recovered because its rendered PDF happened to survive elsewhere.
- **`tests/visual/*/truth.pdf`** — tectonic+biber renders that every fidelity
  comparison measures against. Regenerating all 65 costs a long run and needs
  `.truth-deps` provisioned. Keep both.

Everything else in the candidate list either regenerates on the next harness run,
rebuilds from source, or is re-downloadable.

## Nothing here deletes

`--apply` **moves** candidates into `.cleanup-trash/<timestamp>/` and then verifies
that every item it moved actually arrived. It never runs `rm`. Emptying the trash
is the user's own action, taken after they have looked at it.

That is deliberate. A cleanup script that deletes is one bad glob away from
destroying something irreplaceable — which is exactly how `corpus/beamer-demo/`
was lost, and it was recoverable only by luck. Moving instead makes every mistake
reversible right up until a human types `rm` themselves.

```bash
.claude/skills/byetex-workspace-cleanup/scripts/cleanup.sh              # audit
.claude/skills/byetex-workspace-cleanup/scripts/cleanup.sh --apply      # move to trash
.claude/skills/byetex-workspace-cleanup/scripts/cleanup.sh --only worktrees,target-debug
.claude/skills/byetex-workspace-cleanup/scripts/cleanup.sh --verify     # integrity + trash status
.claude/skills/byetex-workspace-cleanup/scripts/cleanup.sh --restore <timestamp>
```

The manifest records where each item came from, so `--restore` puts things back at
their original paths — including worktrees, which live outside the repo.

## The routine

1. **Audit first, always.** Show the categories with their sizes and say what each
   costs to regenerate. This is where you find out a worktree has uncommitted work
   or a category is far bigger than expected. Let the user choose — they may want
   `target/debug` gone but the cross-compile targets kept.
2. **Get explicit confirmation**, then run `--apply`.
3. **Report the verification**: items moved, all present in trash, integrity
   numbers unchanged.
4. **Ask the user to empty the trash themselves.** Give them the exact path and
   command; do not run it for them, and do not offer to. Space is only reclaimed
   at that point, so say so plainly rather than reporting a saving that has not
   happened yet: "48M held in trash, freed once you delete it."

Report real numbers. "9.4G → 943M" tells the user more than "cleaned up".

## Categories

| Category | Typical | Cost to regenerate |
|---|---|---|
| `worktrees` | 22G | Re-create with `git worktree add` |
| `target-debug` | 5.6G | Next `cargo test --workspace` rebuilds it |
| `visual-renders` | 1.3G | Next `visual_test.py`/`fidelity_gate.sh` run rewrites rasters, composites and converted PDFs anyway |
| `target-cross` | 515M | A zig cross-rebuild, only needed when cutting a release |
| `repo-tmp` | 575M | Nothing — old scratch |
| `corpus-out` | 384M | Any corpus sweep regenerates it |
| `claude-scratch` | 127M | Nothing — old review/grading artifacts |
| `dist` | 40M | Re-downloadable from the GitHub release (verify the assets are actually published first) |
| `git-refs` | — | Prunes stale remote-tracking refs, deletes merged local branches |

`target/release` is deliberately never a candidate: `visual_test.py` and the
acceptance sweep shell out to that binary.

## Why worktrees are gated hardest

They are the only category that can contain unsaved work, and this repo's worktree
workflow makes them unusually dangerous to delete casually:

- **Symlinks point inward.** Worktrees symlink `corpus/` and `tests/visual/` into
  the main checkout, so a worktree directory contains links to the only copies of
  the corpus sources and truth renders. The script unlinks them explicitly before
  the directory moves — a relative symlink that travels to a new depth would
  otherwise resolve somewhere unintended once it is sitting in the trash.
- **A squash-merged branch is not an ancestor of main.** Checking ancestry alone
  reports it as unmerged. The script falls back to comparing the files the branch
  touched against `origin/main`; identical content means the work landed.
- Anything with uncommitted or untracked files is listed as BLOCKED and left alone.
  Resolve those by hand — do not force past them.

## Verify afterwards

The script re-runs its integrity check automatically and fails loudly if the
numbers moved: truth-render count, corpus paper count, and manifest papers missing
from disk (must be 0). Beyond that, confirm the harness still works:

```bash
./scripts/tests/run_all.sh          # expect 15 passed
./target/release/byetex --version   # binary intact
```

If `--apply` reports changed integrity numbers it exits non-zero and prints the
`--restore` command. Nothing is gone at that point — put the batch back, then work
out which pattern was too broad before running it again.

## Related maintenance

- `./scripts/corpus_clean.sh --dry-run` resets a messy corpus. It refuses to delete
  manifest-listed papers, but preview it before running it for real.
- Regenerating truth renders needs `./scripts/setup_truth_deps.sh` (pinned biber +
  fonts). If a paper's truth render starts failing, that is usually why.
