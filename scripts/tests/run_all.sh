#!/usr/bin/env bash
# Collector for scripts/tests/ — runs every test with the deps it declares.
#
# Why a shell loop and not pytest
# ───────────────────────────────
# The 12 tests here are standalone scripts in a hand-rolled convention (local
# `check(cond, desc)` → `fails: list[str]` → `sys.exit(1)` at EOF). Adopting
# pytest means either rewriting all of them or maintaining two conventions
# forever — and it would not solve the actual problem, which is that these tests
# need DIFFERENT third-party dependencies. `scripts/requirements.txt` is not
# installed system-wide (PEP 668 blocks it); each test is run under
# `uv run --with …`. pytest has no answer for that; a shell loop does.
#
# What was missing was never a test framework. It was a collector.
#
# The convention
# ──────────────
# Every test declares its dependencies on line 2 as a machine-readable header:
#
#     # requires: none                 → bare python3, no network, no install
#     # requires: requests Pillow      → uv run --with requests --with Pillow
#
# A test that `# requires: none` is wired into CI. Anything needing a package —
# and especially anything needing pymupdf (AGPL, dev-only, deliberately absent
# from scripts/requirements.txt) or a typst binary — must SKIP CLEANLY (print
# SKIP, exit 0) when its dependency is missing, never fail.
#
# Usage:
#   scripts/tests/run_all.sh              # everything
#   scripts/tests/run_all.sh --pure       # only `# requires: none` (the CI subset)
#   scripts/tests/run_all.sh layout       # only tests whose name matches `layout`
set -uo pipefail

cd "$(dirname "$0")/../.." || exit 2
TESTS_DIR="scripts/tests"

PURE_ONLY=0
FILTER=""
for arg in "$@"; do
  case "$arg" in
    --pure) PURE_ONLY=1 ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) FILTER="$arg" ;;
  esac
done

# Read the `# requires:` header from the first 5 lines. A file without one is
# reported as UNDECLARED rather than guessed at — a wrong guess produces an
# ImportError that reads like a broken test.
requires_of() {
  sed -n '1,5p' "$1" | sed -n 's/^# requires:[[:space:]]*//p' | head -1
}

declare -a NAMES=() STATES=() NOTES=()
rc=0

for f in "$TESTS_DIR"/*_test.py "$TESTS_DIR"/*_test.sh; do
  [ -e "$f" ] || continue
  base="$(basename "$f")"
  [ -n "$FILTER" ] && [[ "$base" != *"$FILTER"* ]] && continue

  req="$(requires_of "$f")"
  if [ -z "$req" ]; then
    NAMES+=("$base"); STATES+=("UNDECLARED"); NOTES+=("add a '# requires:' header")
    rc=1
    continue
  fi
  if [ "$PURE_ONLY" = 1 ] && [ "$req" != "none" ]; then
    NAMES+=("$base"); STATES+=("skip"); NOTES+=("needs: $req")
    continue
  fi

  if [[ "$f" == *.sh ]]; then
    cmd=(bash "$f")
  elif [ "$req" = "none" ]; then
    cmd=(python3 "$f")
  else
    cmd=(uv run)
    for pkg in $req; do cmd+=(--with "$pkg"); done
    cmd+=(python "$f")
  fi

  echo "── $base  (requires: $req)"
  out="$("${cmd[@]}" 2>&1)"
  status=$?
  if [ $status -eq 0 ]; then
    # A test that skipped itself is not a pass — say so, or a permanently
    # skipped test reads as green coverage forever.
    if grep -qE '^SKIP' <<<"$out"; then
      NAMES+=("$base"); STATES+=("SKIP"); NOTES+=("$(grep -m1 -E '^SKIP' <<<"$out")")
    else
      NAMES+=("$base"); STATES+=("pass"); NOTES+=("")
    fi
  else
    NAMES+=("$base"); STATES+=("FAIL"); NOTES+=("exit $status")
    printf '%s\n' "$out" | tail -25
    rc=1
  fi
done

echo
printf '%-38s %-10s %s\n' "TEST" "RESULT" "NOTE"
printf '%-38s %-10s %s\n' "──────────────────────────────────────" "──────────" "────"
for i in "${!NAMES[@]}"; do
  printf '%-38s %-10s %s\n' "${NAMES[$i]}" "${STATES[$i]}" "${NOTES[$i]}"
done

n_fail=0; n_skip=0; n_pass=0
for s in "${STATES[@]}"; do
  case "$s" in pass) ((n_pass++)) ;; FAIL|UNDECLARED) ((n_fail++)) ;; *) ((n_skip++)) ;; esac
done
echo
echo "$n_pass passed, $n_skip skipped, $n_fail failed"
exit $rc
