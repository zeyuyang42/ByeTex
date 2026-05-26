#!/usr/bin/env bash
# corpus_sweep.sh — re-convert all arXiv corpus papers and report compile status.
#
# Usage:
#   ./scripts/corpus_sweep.sh              # full sweep, terse (PASS/FAIL + first error)
#   ./scripts/corpus_sweep.sh --errors N   # show up to N error lines per failure (default 1)
#   ./scripts/corpus_sweep.sh --summary    # only print the PASS/FAIL/SKIP totals
#   ./scripts/corpus_sweep.sh 2605.22485   # run only one paper
#
# The script rebuilds the release binary if any Rust source is newer than the binary.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BYETEX="$REPO_ROOT/target/release/byetex"
CORPUS="$REPO_ROOT/corpus/online/arxiv"

# ── flags ────────────────────────────────────────────────────────────────────
MAX_ERRORS=1
SUMMARY_ONLY=false
FILTER=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --errors) MAX_ERRORS="$2"; shift 2 ;;
    --summary) SUMMARY_ONLY=true; shift ;;
    --*) echo "Unknown flag: $1" >&2; exit 1 ;;
    *)  FILTER="$1"; shift ;;
  esac
done

# ── build ────────────────────────────────────────────────────────────────────
needs_build=false
[[ ! -f "$BYETEX" ]] && needs_build=true
if ! $needs_build && find "$REPO_ROOT/crates" -name '*.rs' -newer "$BYETEX" | grep -q .; then
  needs_build=true
fi
if $needs_build; then
  echo "Building byetex (release)…" >&2
  cargo build --release -p byetex-cli --manifest-path "$REPO_ROOT/Cargo.toml" >&2
fi

# ── sweep ────────────────────────────────────────────────────────────────────
pass=0; fail=0; skip=0

for paper_dir in "$CORPUS"/*/; do
  # Skip non-directories (e.g. manifest.json appears as a glob match on some shells)
  [[ -d "$paper_dir" ]] || continue
  paper_id=$(basename "$paper_dir")
  [[ -n "$FILTER" && "$paper_id" != "$FILTER" ]] && continue

  src_dir="$paper_dir/source"
  proj_dir="$paper_dir/source.typst-project"
  readme="$src_dir/00README.json"

  if [[ ! -f "$readme" || ! -d "$proj_dir" ]]; then
    skip=$((skip+1)); continue
  fi

  top_tex=$(python3 - <<PYEOF 2>/dev/null
import json
d = json.load(open('$readme'))
srcs = [s for s in d.get('sources', []) if s.get('usage') == 'toplevel']
print(srcs[0]['filename'] if srcs else '')
PYEOF
)

  if [[ -z "$top_tex" || ! -f "$src_dir/$top_tex" ]]; then
    skip=$((skip+1)); continue
  fi

  stem="${top_tex%.tex}"

  # Convert
  rm -f "$src_dir/$stem.typ"
  (cd "$src_dir" && "$BYETEX" convert "$top_tex" > /dev/null 2>&1) || true

  if [[ ! -f "$src_dir/$stem.typ" ]]; then
    $SUMMARY_ONLY || echo "FAIL(no_typ) $paper_id"
    fail=$((fail+1)); continue
  fi

  cp "$src_dir/$stem.typ" "$proj_dir/main.typ"

  # Compile
  typst_out=$(cd "$proj_dir" && typst compile main.typ main.pdf 2>&1) || true
  errors=$(echo "$typst_out" | grep "^error:" | head -"$MAX_ERRORS")

  if [[ -z "$errors" ]]; then
    $SUMMARY_ONLY || echo "PASS $paper_id"
    pass=$((pass+1))
  else
    if ! $SUMMARY_ONLY; then
      first=$(echo "$errors" | head -1)
      echo "FAIL $paper_id: $first"
    fi
    fail=$((fail+1))
  fi
done

echo "---"
echo "PASS: $pass  FAIL: $fail  SKIP: $skip  TOTAL: $((pass+fail+skip))"
