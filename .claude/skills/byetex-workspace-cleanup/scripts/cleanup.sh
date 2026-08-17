#!/usr/bin/env bash
# ByeTex post-session workspace cleanup — audit by default, delete only on --apply.
#
# Reclaims build/render artifacts that accumulate during a long dev session. Every
# category here either regenerates on the next harness run, rebuilds from source,
# or is re-downloadable. Nothing irreplaceable is ever a candidate.
#
#   ./cleanup.sh                 # audit: sizes + safety checks, deletes NOTHING
#   ./cleanup.sh --apply         # delete, after the caller has confirmed
#   ./cleanup.sh --only a,b      # restrict to categories
#   ./cleanup.sh --verify        # post-cleanup integrity check only
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && git rev-parse --show-toplevel 2>/dev/null)"
[[ -z "$REPO_ROOT" ]] && { echo "error: not inside a git repo" >&2; exit 1; }
cd "$REPO_ROOT" || exit 1
[[ -f corpus/manifest.json ]] || { echo "error: $REPO_ROOT is not the ByeTex repo" >&2; exit 1; }

APPLY=false; ONLY=""; VERIFY_ONLY=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply)  APPLY=true; shift ;;
    --only)   ONLY="$2"; shift 2 ;;
    --verify) VERIFY_ONLY=true; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

wants() { [[ -z "$ONLY" ]] || [[ ",$ONLY," == *",$1,"* ]]; }
sz()    { du -sh "$@" 2>/dev/null | tail -1 | cut -f1; }
tot()   { du -shc "$@" 2>/dev/null | tail -1 | cut -f1; }

# ── things that must never be deleted ───────────────────────────────────────
# corpus/<id>/source  pristine inputs; some papers are `"source": "local"`,
#                     hand-authored and NOT re-downloadable (see beamer-demo)
# tests/visual/*/truth.pdf  tectonic+biber renders, expensive and reused as the
#                     comparison truth on every run
# .truth-deps         pinned biber 2.17 + fonts that produce those renders
# .git                obviously
NEVER=(corpus/*/source "tests/visual/*/truth.pdf" .truth-deps .git)

# ═══════════════════════════════════════════════════════════════════════════
# Integrity check — run before AND after; a cleanup that changes these numbers
# has destroyed something it should not have.
# ═══════════════════════════════════════════════════════════════════════════
integrity() {
  local truth corpus_dirs missing
  truth=$(ls tests/visual/*/truth.pdf 2>/dev/null | wc -l | tr -d ' ')
  corpus_dirs=$(ls -d corpus/*/ 2>/dev/null | grep -cv '_out' | tr -d ' ')
  missing=$(python3 - <<'PY'
import json, os
try:
    m = json.load(open("corpus/manifest.json"))
    papers = m.get("papers", [])
    ids = papers.keys() if isinstance(papers, dict) else [p.get("id") for p in papers]
    print(len([i for i in ids if i and not os.path.isdir(f"corpus/{i}")]))
except Exception:
    print("?")
PY
)
  printf "  truth.pdf renders      %s\n" "$truth"
  printf "  corpus paper dirs      %s\n" "$corpus_dirs"
  printf "  manifest papers MISSING from disk  %s   <- must be 0\n" "$missing"
  printf "  .truth-deps            %s\n" "$([[ -d .truth-deps ]] && sz .truth-deps || echo MISSING)"
  echo "$truth:$corpus_dirs:$missing"
}

if $VERIFY_ONLY; then
  echo "── integrity ──"; integrity >/dev/null; integrity | head -4
  echo "── harness ──"
  [[ -x target/release/byetex ]] && ./target/release/byetex --version || echo "  no release binary (rebuild with cargo build --release)"
  exit 0
fi

echo "════ ByeTex workspace cleanup — $($APPLY && echo 'APPLY (deleting)' || echo 'AUDIT (no deletions)') ════"
echo "repo: $REPO_ROOT   total: $(sz .)"
echo
echo "── integrity BEFORE ──"
BEFORE="$(integrity | tail -1)"; integrity | head -4
echo

CANDIDATES=()   # paths to remove
plan() {        # <category> <label> <paths...>
  local cat="$1" label="$2"; shift 2
  wants "$cat" || return 0
  local found=()
  for p in "$@"; do [[ -e "$p" || -L "$p" ]] && found+=("$p"); done
  [[ ${#found[@]} -eq 0 ]] && return 0
  printf "  %-16s %-8s %s\n" "$cat" "$(tot "${found[@]}")" "$label"
  CANDIDATES+=("${found[@]}")
}

# ═══════════════════════════════════════════════════════════════════════════
# 1. WORKTREES — the biggest win (22G in the session that motivated this skill),
#    and the only category that can lose work, so it is gated hardest.
# ═══════════════════════════════════════════════════════════════════════════
WORKTREE_SAFE=(); WORKTREE_BLOCKED=()
if wants worktrees; then
  while read -r wt _; do
    [[ "$wt" == "$REPO_ROOT" ]] && continue
    [[ -d "$wt" ]] || continue
    name=$(basename "$wt")
    dirty=$(git -C "$wt" status --porcelain 2>/dev/null | grep -cv '^??' | tr -d ' ')
    untracked=$(git -C "$wt" status --porcelain 2>/dev/null | grep -c '^??' | tr -d ' ')
    br=$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null)
    reason=""
    [[ "$dirty" != 0 ]] && reason="$dirty uncommitted change(s)"
    [[ "$untracked" != 0 ]] && reason="${reason:+$reason; }$untracked untracked file(s)"
    if [[ -z "$reason" && "$br" != "HEAD" ]]; then
      # Merged? Either the tip is an ancestor of main, or (squash-merged) the
      # files it touched are byte-identical in main. A squash merge rewrites the
      # commit, so ancestry alone would wrongly flag it as unmerged.
      if ! git merge-base --is-ancestor "$br" origin/main 2>/dev/null; then
        files=$(git diff --name-only "origin/main...$br" 2>/dev/null)
        if [[ -n "$files" ]] && [[ -n "$(git diff origin/main "$br" -- $files 2>/dev/null)" ]]; then
          reason="branch '$br' has content not in origin/main"
        fi
      fi
    fi
    if [[ -n "$reason" ]]; then WORKTREE_BLOCKED+=("$name — $reason")
    else WORKTREE_SAFE+=("$wt"); fi
  done < <(git worktree list)
fi

echo "── candidates ──"
if [[ ${#WORKTREE_SAFE[@]} -gt 0 ]]; then
  printf "  %-16s %-8s %s\n" "worktrees" "$(tot "${WORKTREE_SAFE[@]}")" "${#WORKTREE_SAFE[@]} merged worktree(s), no uncommitted work"
fi

# 2. Rasters/composites/converted PDFs — visual_test.py rewrites all of these on
#    every run. truth.pdf is deliberately excluded (see NEVER).
plan visual-renders "page rasters, composites, converted PDFs (truth.pdf KEPT)" \
  tests/visual/*/pages tests/visual/*/composite.png tests/visual/*/typst.pdf
# 3. Build artifacts. target/release stays — the harness shells out to it.
plan target-debug "cargo test debug artifacts" target/debug
plan target-cross "cross-compile targets (release-time only)" \
  target/aarch64-apple-darwin target/x86_64-apple-darwin \
  target/x86_64-pc-windows-gnu target/aarch64-unknown-linux-musl \
  target/x86_64-unknown-linux-musl
# 4. Scratch that accumulates and is never read again.
plan repo-tmp     "repo scratch dir" tmp
plan corpus-out   "generated conversion output" corpus/_out
plan claude-scratch "stale review/grading scratch" .claude/review .claude/review-* .claude/scratch_*
# 5. Local copies of published release binaries.
plan dist         "built release binaries (re-downloadable from GitHub releases)" dist

if [[ ${#CANDIDATES[@]} -eq 0 && ${#WORKTREE_SAFE[@]} -eq 0 ]]; then
  echo "  (nothing to reclaim)"
fi

if [[ ${#WORKTREE_BLOCKED[@]} -gt 0 ]]; then
  echo
  echo "── BLOCKED (not touched) ──"
  for b in "${WORKTREE_BLOCKED[@]}"; do echo "  ! $b"; done
fi

if ! $APPLY; then
  echo
  echo "AUDIT ONLY — nothing deleted. Re-run with --apply once the user has confirmed."
  exit 0
fi

# ═══════════════════════════════════════════════════════════════════════════
# APPLY
# ═══════════════════════════════════════════════════════════════════════════
echo
echo "── applying ──"
# Worktrees first, and their symlinks BEFORE the worktree itself: this repo's
# worktree workflow symlinks corpus/ and tests/visual/ INTO the main checkout, so
# the links point at the only copies of the corpus sources and truth renders.
# Unlinking explicitly makes that safe by construction rather than by trusting
# how a given rm implementation treats symlinks during recursion.
for wt in ${WORKTREE_SAFE[@]+"${WORKTREE_SAFE[@]}"}; do
  n=$(find "$wt" -type l 2>/dev/null | wc -l | tr -d ' ')
  [[ "$n" != 0 ]] && { find "$wt" -type l -delete 2>/dev/null; echo "  unlinked $n symlink(s) in $(basename "$wt")"; }
  git worktree remove --force "$wt" 2>/dev/null && echo "  removed worktree $(basename "$wt")" \
    || echo "  FAILED to remove $(basename "$wt")"
done
[[ ${#WORKTREE_SAFE[@]} -gt 0 ]] && git worktree prune

for p in ${CANDIDATES[@]+"${CANDIDATES[@]}"}; do rm -rf "$p"; done
[[ ${#CANDIDATES[@]} -gt 0 ]] && echo "  removed ${#CANDIDATES[@]} artifact path(s)"

if wants git-refs; then
  git fetch --prune -q origin 2>/dev/null
  for b in $(git branch --merged origin/main --format='%(refname:short)' 2>/dev/null | grep -v '^main$'); do
    git branch -d "$b" >/dev/null 2>&1 && echo "  deleted merged branch $b"
  done
fi

echo
echo "── integrity AFTER ──"
AFTER="$(integrity | tail -1)"; integrity | head -4
echo
if [[ "$BEFORE" == "$AFTER" ]]; then
  echo "OK: irreplaceable data unchanged. total now $(sz .)"
else
  echo "STOP: integrity numbers CHANGED ($BEFORE -> $AFTER). Something was destroyed;" >&2
  echo "      investigate before running anything else." >&2
  exit 1
fi
