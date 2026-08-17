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
# visual_test.py (e.g. `--all`, `--papers <id> ...`, `--truth-source tectonic`).
#
# COVERAGE: pass `--all` for a release gate. Without it visual_test.py measures
# only the 5 PINNED papers — 7% of a 71-paper baseline — and fidelity_check then
# SKIPS the corpus-score comparison, because a mean over 5 papers is not
# comparable to a mean over 71. The bare command is a quick check, not a gate;
# it prints a PARTIAL RUN warning naming every paper it did not measure.
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

# Produce a fresh index.json.
#
# DEPENDENCIES — the layout tier is SILENT when they are missing, so this matters:
#   requests, Pillow          required (visual_test imports them at module level)
#   numpy, scikit-image       SSIM; absent -> mean_ssim None
#   pymupdf                   the T1 GEOMETRIC PROFILE (margins, columns, font
#                             tiers, leading, ink); absent -> every layout_*
#                             property is None on every paper
#
# A missing pymupdf does not fail anything and does not look like an error: the
# tier just reports nothing, which is indistinguishable from a clean corpus. The
# gate prints "LAYOUT: not computed on N/M papers" when that happens — heed it.
#
#   uv run --with requests --with Pillow --with numpy --with scikit-image \
#          --with pymupdf -- ./scripts/fidelity_gate.sh --all
#
# pymupdf is DEV-ONLY and deliberately NOT in scripts/requirements.txt: it is
# AGPL-licensed, and it must never be linked into the shipped Rust binary or
# become a runtime dependency of ByeTex itself. It is a measurement tool for this
# harness only.
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
