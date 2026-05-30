#!/usr/bin/env bash
# corpus_sweep.sh — re-convert all arXiv corpus papers and report compile status.
#
# Usage:
#   ./scripts/corpus_sweep.sh              # full sweep, terse (PASS/FAIL + first error)
#   ./scripts/corpus_sweep.sh --errors N   # show up to N error lines per failure (default 1)
#   ./scripts/corpus_sweep.sh --summary    # only print the PASS/FAIL/SKIP totals
#   ./scripts/corpus_sweep.sh --with-oracle  # attribute each FAIL via `byetex doctor`:
#                                            #   INPUT_BROKEN (the source itself won't compile)
#                                            #   vs BYETEX_FAIL (valid source, our output broke)
#   ./scripts/corpus_sweep.sh 2605.22485   # run only one paper
#
# The script rebuilds the release binary if any Rust source is newer than the binary.
#
# Env overrides (used by scripts/tests/corpus_sweep_oracle_test.sh):
#   BYETEX_BIN         path to the byetex binary (its presence skips the cargo build)
#   BYETEX_CORPUS_DIR  corpus root to sweep instead of corpus/online/arxiv

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BYETEX="${BYETEX_BIN:-$REPO_ROOT/target/release/byetex}"
CORPUS="${BYETEX_CORPUS_DIR:-$REPO_ROOT/corpus/online/arxiv}"

# ── flags ────────────────────────────────────────────────────────────────────
MAX_ERRORS=1
SUMMARY_ONLY=false
WITH_ORACLE=false
FILTER=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --errors) MAX_ERRORS="$2"; shift 2 ;;
    --summary) SUMMARY_ONLY=true; shift ;;
    --with-oracle) WITH_ORACLE=true; shift ;;
    --*) echo "Unknown flag: $1" >&2; exit 1 ;;
    *)  FILTER="$1"; shift ;;
  esac
done

# ── build ────────────────────────────────────────────────────────────────────
# Skip the build entirely when BYETEX_BIN points at a binary (tests, CI cache).
if [[ -z "${BYETEX_BIN:-}" ]]; then
  needs_build=false
  [[ ! -f "$BYETEX" ]] && needs_build=true
  if ! $needs_build && find "$REPO_ROOT/crates" -name '*.rs' -newer "$BYETEX" | grep -q .; then
    needs_build=true
  fi
  if $needs_build; then
    echo "Building byetex (release)…" >&2
    cargo build --release -p byetex-cli --manifest-path "$REPO_ROOT/Cargo.toml" >&2
  fi
fi

# Attribute a single paper's failure via the Stage-0 oracle. Reads the loop's
# current paper context ($src_dir/$top_tex/$stem/$paper_id) and bumps the
# matching counter. Falls back to UNATTRIBUTED when the oracle can't run
# (e.g. tectonic not installed) — never silently blames ByeTex.
attribute_failure() {  # <first-error-message>
  local msg="$1" verdict=""
  (cd "$src_dir" && "$BYETEX" doctor "$top_tex" > /dev/null 2>&1) || true
  verdict=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get('verdict',''))" \
            "$src_dir/${stem}.doctor.json" 2>/dev/null || true)
  case "$verdict" in
    input_broken)
      $SUMMARY_ONLY || echo "INPUT_BROKEN $paper_id: $msg"
      input_broken=$((input_broken+1)) ;;
    ok|byetex_bug)
      $SUMMARY_ONLY || echo "BYETEX_FAIL $paper_id: $msg"
      byetex_fail=$((byetex_fail+1)) ;;
    *)
      $SUMMARY_ONLY || echo "FAIL(unattributed) $paper_id: $msg"
      unattributed=$((unattributed+1)) ;;
  esac
}

# ── sweep ────────────────────────────────────────────────────────────────────
pass=0; fail=0; skip=0
byetex_fail=0; input_broken=0; unattributed=0

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
  gen_proj="$src_dir/${stem}.typst-project"

  # Convert with --project so bib preprocessing and asset copying are
  # always fresh (prevents stale bib files from masking preprocessor
  # improvements). The generated project is a sibling of the source
  # dir; we prefer it over the curated source.typst-project/ for the
  # compile step.
  rm -rf "$gen_proj"
  (cd "$src_dir" && "$BYETEX" convert --project "$top_tex" > /dev/null 2>&1) || true

  if [[ ! -f "$gen_proj/main.typ" ]]; then
    if $WITH_ORACLE; then
      attribute_failure "(no main.typ produced)"
    else
      $SUMMARY_ONLY || echo "FAIL(no_typ) $paper_id"
      fail=$((fail+1))
    fi
    continue
  fi

  # Compile
  typst_out=$(cd "$gen_proj" && typst compile main.typ main.pdf 2>&1) || true
  errors=$(echo "$typst_out" | grep "^error:" | head -"$MAX_ERRORS")

  if [[ -z "$errors" ]]; then
    $SUMMARY_ONLY || echo "PASS $paper_id"
    pass=$((pass+1))
  else
    first=$(echo "$errors" | head -1)
    if $WITH_ORACLE; then
      attribute_failure "$first"
    else
      $SUMMARY_ONLY || echo "FAIL $paper_id: $first"
      fail=$((fail+1))
    fi
  fi
done

echo "---"
if $WITH_ORACLE; then
  echo "PASS: $pass  BYETEX_FAIL: $byetex_fail  INPUT_BROKEN: $input_broken  UNATTRIBUTED: $unattributed  SKIP: $skip  TOTAL: $((pass+byetex_fail+input_broken+unattributed+skip))"
else
  echo "PASS: $pass  FAIL: $fail  SKIP: $skip  TOTAL: $((pass+fail+skip))"
fi
