#!/usr/bin/env bash
# Phase-3 acceptance gate (tectonic round-trip oracle).
#
# Runs the corpus sweep with `byetex doctor` attribution, then fails (exit 1)
# iff a paper that currently compiles (baseline `known_pass`) has regressed to
# BYETEX_FAIL. Compile-rate is the GATE; fidelity is the DRIVER (measured by
# scripts/visual_test.py, not gated here) — see docs/scorecard.md.
#
# Usage:
#   ./scripts/acceptance.sh
# Env (forwarded to corpus_sweep.sh):
#   BYETEX_BIN         byetex binary to test (else builds release)
#   BYETEX_CORPUS_DIR  corpus root to sweep
#   ACCEPTANCE_BASELINE  baseline JSON (default scripts/acceptance_baseline.json)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${ACCEPTANCE_BASELINE:-$REPO_ROOT/scripts/acceptance_baseline.json}"

if [[ ! -f "$BASELINE" ]]; then
  echo "acceptance: baseline not found: $BASELINE" >&2
  exit 2
fi

# Run the attributed sweep and tee it so the verdicts are visible in the log.
sweep_log="$(mktemp "${TMPDIR:-$REPO_ROOT}/.acceptance_sweep.XXXXXX")"
trap 'rm -f "$sweep_log"' EXIT
"$REPO_ROOT/scripts/corpus_sweep.sh" --with-oracle | tee "$sweep_log"

echo "─── acceptance gate ───"
python3 "$REPO_ROOT/scripts/acceptance_check.py" --baseline "$BASELINE" < "$sweep_log"
