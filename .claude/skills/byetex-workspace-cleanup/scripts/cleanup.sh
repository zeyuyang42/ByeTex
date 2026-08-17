#!/usr/bin/env bash
# ByeTex post-session workspace cleanup — audit by default; --apply MOVES to trash.
#
# Nothing here ever deletes. `--apply` moves candidates into
# `.cleanup-trash/<timestamp>/`, preserving their relative paths, and then
# verifies that everything it moved actually arrived. Emptying that folder is a
# human decision, made after looking at it — which is the whole point: a mistake
# is recoverable right up until you run the `rm` yourself.
#
#   ./cleanup.sh                 # audit: sizes + safety checks, moves NOTHING
#   ./cleanup.sh --apply         # move to trash, after the caller has confirmed
#   ./cleanup.sh --only a,b      # restrict to categories
#   ./cleanup.sh --verify        # integrity check only
#   ./cleanup.sh --restore <ts>  # put a trash batch back where it came from
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && git rev-parse --show-toplevel 2>/dev/null)"
[[ -z "$REPO_ROOT" ]] && { echo "error: not inside a git repo" >&2; exit 1; }
cd "$REPO_ROOT" || exit 1
[[ -f corpus/manifest.json ]] || { echo "error: $REPO_ROOT is not the ByeTex repo" >&2; exit 1; }

TRASH_ROOT="$REPO_ROOT/.cleanup-trash"
APPLY=false; ONLY=""; VERIFY_ONLY=false; RESTORE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply)   APPLY=true; shift ;;
    --only)    ONLY="$2"; shift 2 ;;
    --verify)  VERIFY_ONLY=true; shift ;;
    --restore) RESTORE="$2"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

wants() { [[ -z "$ONLY" ]] || [[ ",$ONLY," == *",$1,"* ]]; }
sz()    { du -sh "$@" 2>/dev/null | tail -1 | cut -f1; }
tot()   { du -shc "$@" 2>/dev/null | tail -1 | cut -f1; }

# ── things that are never candidates ────────────────────────────────────────
# corpus/<id>/source        pristine inputs; some papers are `"source": "local"`,
#                           hand-authored and NOT re-downloadable
# tests/visual/*/truth.pdf  tectonic+biber renders, expensive, reused as the
#                           comparison truth on every fidelity run
# .truth-deps               pinned biber 2.17 + fonts that produce those renders
# .git                      obviously

# ═══════════════════════════════════════════════════════════════════════════
# Integrity — the same numbers before and after. If they move, something left
# that should not have.
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

show_trash() {
  [[ -d "$TRASH_ROOT" ]] || return 0
  local batches; batches=$(find "$TRASH_ROOT" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | sort)
  [[ -z "$batches" ]] && return 0
  echo "── trash awaiting YOUR review (nothing here has been deleted) ──"
  while IFS= read -r b; do
    printf "  %-24s %-8s %s item(s)\n" "$(basename "$b")" "$(sz "$b")" \
      "$(wc -l < "$b/MANIFEST.txt" 2>/dev/null | tr -d ' ')"
  done <<< "$batches"
  printf "  total held: %s\n" "$(sz "$TRASH_ROOT")"
}

# ── restore ─────────────────────────────────────────────────────────────────
if [[ -n "$RESTORE" ]]; then
  B="$TRASH_ROOT/$RESTORE"
  [[ -d "$B" ]] || { echo "no such trash batch: $B" >&2; exit 1; }
  n=0
  while IFS=$'\t' read -r rel orig; do
    [[ -z "$rel" ]] && continue
    src="$B/$rel"
    # Fall back to a repo-relative path for batches written before origins were
    # recorded, so an older trash batch is still restorable.
    dest="${orig:-$REPO_ROOT/$rel}"
    [[ -e "$src" ]] || continue
    mkdir -p "$(dirname "$dest")"
    if mv "$src" "$dest"; then echo "  restored $dest"; n=$((n+1)); fi
  done < "$B/MANIFEST.txt"
  echo "restored $n item(s) from $RESTORE"
  echo "note: worktrees come back as plain directories — re-register with \`git worktree add\`."
  exit 0
fi

if $VERIFY_ONLY; then
  echo "── integrity ──"; integrity | head -4
  echo "── harness ──"
  [[ -x target/release/byetex ]] && ./target/release/byetex --version || echo "  no release binary (cargo build --release)"
  echo; show_trash
  exit 0
fi

echo "════ ByeTex workspace cleanup — $($APPLY && echo 'APPLY (moving to trash)' || echo 'AUDIT (nothing moved)') ════"
echo "repo: $REPO_ROOT   total: $(sz .)"
echo
echo "── integrity BEFORE ──"
BEFORE="$(integrity | tail -1)"; integrity | head -4
echo

CANDIDATES=()
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
# 1. WORKTREES — the biggest win, and the only category that can hold unsaved
#    work, so it is gated hardest.
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
      # A squash merge rewrites the commit, so the branch tip is NOT an ancestor
      # of main even though the work landed. Fall back to comparing content.
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

# visual_test.py rewrites all of these on every run. truth.pdf is excluded.
plan visual-renders "page rasters, composites, converted PDFs (truth.pdf KEPT)" \
  tests/visual/*/pages tests/visual/*/composite.png tests/visual/*/typst.pdf
# target/release stays — the harness shells out to that binary.
plan target-debug "cargo test debug artifacts" target/debug
plan target-cross "cross-compile targets (release-time only)" \
  target/aarch64-apple-darwin target/x86_64-apple-darwin \
  target/x86_64-pc-windows-gnu target/aarch64-unknown-linux-musl \
  target/x86_64-unknown-linux-musl
plan repo-tmp     "repo scratch dir" tmp
plan corpus-out   "generated conversion output" corpus/_out
plan claude-scratch "stale review/grading scratch" .claude/review .claude/review-* .claude/scratch_*
DS_FILES=()
while IFS= read -r f; do DS_FILES+=("$f"); done < <(find . -name .DS_Store -not -path './.git/*' -not -path './.cleanup-trash/*' 2>/dev/null)
plan ds-store     "macOS .DS_Store files" ${DS_FILES[@]+"${DS_FILES[@]}"}
plan dist         "built release binaries (re-downloadable from GitHub releases)" dist

if [[ ${#CANDIDATES[@]} -eq 0 && ${#WORKTREE_SAFE[@]} -eq 0 ]]; then
  echo "  (nothing to reclaim)"
fi

if [[ ${#WORKTREE_BLOCKED[@]} -gt 0 ]]; then
  echo
  echo "── BLOCKED (not touched) ──"
  for b in "${WORKTREE_BLOCKED[@]}"; do echo "  ! $b"; done
fi

echo
show_trash

if ! $APPLY; then
  echo
  echo "AUDIT ONLY — nothing moved. Re-run with --apply once the user has confirmed."
  exit 0
fi

# ═══════════════════════════════════════════════════════════════════════════
# APPLY — move, never delete
# ═══════════════════════════════════════════════════════════════════════════
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
TRASH="$TRASH_ROOT/$STAMP"
mkdir -p "$TRASH" || { echo "cannot create $TRASH" >&2; exit 1; }
: > "$TRASH/MANIFEST.txt"
MOVED=0; FAILED=0

stash() {  # <path> [rel-override]
  local src="$1" rel="${2:-}" dest orig
  [[ -e "$src" || -L "$src" ]] || return 0
  [[ -z "$rel" ]] && rel="${src#./}"
  # Record where it CAME FROM, not just where it landed. Worktrees live outside
  # the repo, so a trash-relative path alone would restore them into the repo
  # instead of back to their sibling directory.
  orig="$(cd "$(dirname "$src")" 2>/dev/null && pwd)/$(basename "$src")"
  dest="$TRASH/$rel"
  mkdir -p "$(dirname "$dest")"
  if mv "$src" "$dest" 2>/dev/null; then
    printf '%s\t%s\n' "$rel" "$orig" >> "$TRASH/MANIFEST.txt"; MOVED=$((MOVED+1))
  else
    echo "  FAILED to move $src" >&2; FAILED=$((FAILED+1))
  fi
}

echo
echo "── moving to $TRASH ──"
# Worktrees first. Their symlinks point INTO this checkout — at the only copies
# of the corpus sources and truth renders — so unlink them before the directory
# moves. A relative symlink that travels to a new depth would otherwise resolve
# somewhere unintended.
for wt in ${WORKTREE_SAFE[@]+"${WORKTREE_SAFE[@]}"}; do
  n=$(find "$wt" -type l 2>/dev/null | wc -l | tr -d ' ')
  [[ "$n" != 0 ]] && { find "$wt" -type l -delete 2>/dev/null; echo "  unlinked $n symlink(s) in $(basename "$wt")"; }
  stash "$wt" "_worktrees/$(basename "$wt")"
done
if [[ ${#WORKTREE_SAFE[@]} -gt 0 ]]; then
  git worktree prune   # their directories are gone from the registered location
fi

for p in ${CANDIDATES[@]+"${CANDIDATES[@]}"}; do stash "$p"; done

if wants git-refs; then
  git fetch --prune -q origin 2>/dev/null
  for b in $(git branch --merged origin/main --format='%(refname:short)' 2>/dev/null | grep -v '^main$'); do
    git branch -d "$b" >/dev/null 2>&1 && echo "  deleted merged branch $b (recoverable via reflog / origin)"
  done
fi

echo "  moved $MOVED item(s)$([[ $FAILED != 0 ]] && echo ", $FAILED FAILED")"

# ── did everything actually arrive? ─────────────────────────────────────────
echo
echo "── verifying the move ──"
LOST=0
while IFS=$'\t' read -r rel _orig; do
  [[ -z "$rel" ]] && continue
  [[ -e "$TRASH/$rel" ]] || { echo "  MISSING FROM TRASH: $rel" >&2; LOST=$((LOST+1)); }
done < "$TRASH/MANIFEST.txt"
if [[ "$LOST" == 0 ]]; then
  echo "  all $MOVED item(s) present in the trash batch — nothing was deleted"
else
  echo "  $LOST item(s) recorded but NOT found in trash — investigate before continuing" >&2
fi

echo
echo "── integrity AFTER ──"
AFTER="$(integrity | tail -1)"; integrity | head -4
echo
if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "STOP: integrity numbers CHANGED ($BEFORE -> $AFTER). Restore with:" >&2
  echo "      $0 --restore $STAMP" >&2
  exit 1
fi
if [[ "$LOST" != 0 || "$FAILED" != 0 ]]; then exit 1; fi

echo "OK: irreplaceable data unchanged."
echo
echo "════ NOTHING WAS DELETED ════"
echo "  held in: $TRASH  ($(sz "$TRASH"))"
echo "  repo now: $(sz .)   (space is reclaimed only once you empty the trash)"
echo
echo "  Review it, then delete it YOURSELF when you are satisfied:"
echo "      rm -rf '$TRASH'"
echo "  Changed your mind?  $0 --restore $STAMP"
