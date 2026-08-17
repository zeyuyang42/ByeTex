#!/usr/bin/env bash
# corpus_clean.sh — reset corpus/ to one flat, canonical layout.
#
# Canonical layout this script enforces:
#   corpus/manifest.json        (the only committed file)
#   corpus/<id>/source/         pristine inputs (tex, figs, .bib, 00README.json)
#   corpus/<id>/source.tar.gz   the fetched archive
#   corpus/_out/<id>/           ALL generated artifacts (gitignored, regenerable)
#
# It removes the historical chaos: the stale corpus/online/ mirror, the
# corpus/inhouse/ template leftovers, stray cross-workspace symlinks, editor
# junk, and generated artifacts scattered into source/ dirs. Inputs are never
# touched — deletion is pattern-based, so source.tar.gz and 00README.json stay.
#
# Idempotent: re-running is a no-op once the corpus is clean.
#
# Usage:
#   ./scripts/corpus_clean.sh              # clean in place
#   ./scripts/corpus_clean.sh --dry-run    # print what would be removed, change nothing
#   ./scripts/corpus_clean.sh --purge-out  # also wipe corpus/_out/ (forces full regen)
#
# Env overrides:
#   BYETEX_CORPUS_DIR  corpus root to clean instead of corpus/ (matches corpus_sweep.sh)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS="${BYETEX_CORPUS_DIR:-$REPO_ROOT/corpus}"

DRY_RUN=false
PURGE_OUT=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run|-n) DRY_RUN=true; shift ;;
    --purge-out)  PURGE_OUT=true; shift ;;
    --*) echo "Unknown flag: $1" >&2; exit 1 ;;
    *)   echo "Unexpected argument: $1" >&2; exit 1 ;;
  esac
done

# Guard: refuse to run anywhere that isn't a corpus root.
if [[ ! -f "$CORPUS/manifest.json" ]]; then
  echo "error: $CORPUS/manifest.json not found — refusing to clean (set BYETEX_CORPUS_DIR?)" >&2
  exit 1
fi

# rm wrapper honoring --dry-run; prints every path it touches.
zap() {  # <path>...
  for p in "$@"; do
    [[ -e "$p" || -L "$p" ]] || continue
    echo "  rm $p"
    $DRY_RUN || rm -rf "$p"
  done
}

# An arXiv id top-level dir looks like 2605.22159 / 2606.12397.
ID_RE='^[0-9]{4}\.[0-9]{4,6}$'

# Papers the manifest claims. The manifest is the ONLY committed part of the
# corpus, so anything it lists is tracked data — never junk — regardless of what
# metadata files happen to sit inside the directory.
MANIFEST_IDS="$(python3 - "$CORPUS/manifest.json" <<'PY'
import json, sys
try:
    papers = json.load(open(sys.argv[1])).get("papers", [])
except Exception:
    sys.exit(0)  # unreadable manifest -> empty list; the rules below stay as they were
if isinstance(papers, dict):
    ids = papers.keys()
else:
    ids = [p.get("id") for p in papers if isinstance(p, dict)]
print("\n".join(str(i) for i in ids if i))
PY
)"

in_manifest() {  # <dir-name>
  [[ -n "$MANIFEST_IDS" ]] && grep -qxF "$1" <<<"$MANIFEST_IDS"
}

echo "Cleaning corpus root: $CORPUS"
$DRY_RUN && echo "(dry run — no changes)"

# 1. Structural: drop the stale mirror, inhouse leftovers, and any non-id dir.
echo "[structural] stray top-level dirs"
for entry in "$CORPUS"/*/; do
  name="$(basename "$entry")"
  [[ "$name" == "_out" ]] && continue            # generated tree, handled below
  [[ "$name" =~ $ID_RE ]] && continue            # an arXiv paper — keep
  [[ -f "$entry/source/00README.json" ]] && continue  # non-arXiv paper (corpus_add_local.py) — keep
  # A paper the manifest lists is tracked data, even if its `00README.json` is
  # missing. Without this check the readme was load-bearing for SURVIVAL, and a
  # `source: local` paper — hand-authored, not re-downloadable, gitignored — was
  # deleted by a routine `--purge-out` run purely because it lacked one
  # (beamer-demo, 2026-08-17; recovered only because its rendered truth.pdf
  # happened to still be in tests/visual). Warn loudly instead of deleting.
  if in_manifest "$name"; then
    echo "  WARN keeping $name — listed in manifest.json but has no source/00README.json;" >&2
    echo "       add one (see corpus_add_local.py) so the harness can find its toplevel." >&2
    continue
  fi
  zap "$entry"                                    # online/, inhouse/, anything else
done

# 2. Stray symlinks anywhere under corpus (no legit symlink lives inside corpus;
#    the worktree symlink workflow links corpus IN from outside, never the reverse).
echo "[symlinks] stray links under corpus"
while IFS= read -r -d '' link; do
  zap "$link"
done < <(find "$CORPUS" -mindepth 1 -type l -print0)

# 3. Generated artifacts scattered into paper dirs (skip the _out tree entirely).
echo "[artifacts] generated files in paper dirs"
while IFS= read -r -d '' f; do
  zap "$f"
done < <(find "$CORPUS" -path "$CORPUS/_out" -prune -o -type f \
           \( -name '*.typ' -o -name '*.warnings.json' \
              -o -name '*.agent_brief.md' -o -name '*.doctor.json' \) -print0)

echo "[artifacts] generated dirs in paper dirs"
while IFS= read -r -d '' d; do
  zap "$d"
done < <(find "$CORPUS" -path "$CORPUS/_out" -prune -o -type d \
           \( -name '*.typst-project' -o -name '.tectonic-out-*' \) -print0)

# 4. Editor / agent junk.
echo "[junk] .DS_Store / .claude"
while IFS= read -r -d '' j; do
  zap "$j"
done < <(find "$CORPUS" \( -name '.DS_Store' -o -name '.claude' \) -print0)

# 5. Optionally wipe the regenerable output tree.
if $PURGE_OUT; then
  echo "[_out] purging generated output tree"
  zap "$CORPUS/_out"
fi

echo "Done."
