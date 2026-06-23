#!/usr/bin/env bash
# Fidelity regression gate (Layer 1 — deterministic, no agent).
#
# Runs scripts/visual_test.py to produce a fresh index.json, then compares it
# against scripts/fidelity_baseline.json via scripts/fidelity_check.py. Fails
# (exit 1) iff render fidelity regresses. Pass `--update-baseline` to overwrite
# the baseline from the fresh run instead of gating.
#
# Compile-rate is gated by scripts/acceptance.sh; this gates RENDER fidelity
# (the DRIVER) — run it before a release. Extra args pass through to
# visual_test.py (e.g. `--papers <id> ...`, `--truth-source tectonic`).
#
# Env:
#   FIDELITY_BASELINE  baseline JSON (default scripts/fidelity_baseline.json)
#   FIDELITY_OUT       visual_test.py output dir (default tests/visual)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASELINE="${FIDELITY_BASELINE:-$REPO_ROOT/scripts/fidelity_baseline.json}"
OUT="${FIDELITY_OUT:-$REPO_ROOT/tests/visual}"

UPDATE=0
VT_ARGS=()
for a in "$@"; do
  case "$a" in
    --update-baseline) UPDATE=1 ;;
    *) VT_ARGS+=("$a") ;;
  esac
done

# Produce a fresh index.json. visual_test.py needs numpy/Pillow for SSIM; if
# they aren't importable, run this via `uv run --with numpy --with pillow -- ...`.
# `${VT_ARGS[@]+…}` (not `"${VT_ARGS[@]}"`): under `set -u`, macOS's bash 3.2 treats an
# empty-array expansion as an unbound variable and aborts. This idiom expands to the
# elements when set, and to NOTHING when empty (no stray empty arg).
python3 "$REPO_ROOT/scripts/visual_test.py" --out "$OUT" ${VT_ARGS[@]+"${VT_ARGS[@]}"}
INDEX="$OUT/index.json"

if [[ ! -f "$INDEX" ]]; then
  echo "fidelity: no index.json produced at $INDEX" >&2
  exit 2
fi

if [[ "$UPDATE" == "1" ]]; then
  python3 "$REPO_ROOT/scripts/fidelity_check.py" --current "$INDEX" --emit-baseline "$BASELINE"
  exit 0
fi

echo "─── fidelity gate ───"
python3 "$REPO_ROOT/scripts/fidelity_check.py" --current "$INDEX" --baseline "$BASELINE"
