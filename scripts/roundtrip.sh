#!/usr/bin/env bash
# roundtrip.sh — round-trip fidelity check for an ARBITRARY LaTeX input.
#
# Unlike corpus_sweep.sh (arXiv-only; compares against arXiv's canonical PDF),
# this renders the *source itself* with tectonic as the "truth", so it works
# for any local LaTeX — not just papers that have an arXiv canonical PDF.
#
#   1. byetex doctor <input>                  Stage-0 attribution (skips if no tectonic)
#   2. byetex convert --project + typst       -> generated.pdf  (our output)
#   3. tectonic <input>                        -> reference.pdf  (the truth)
#   4. rasterize both + structural compare     (reuses scripts/visual_test.py)
#   5. composite.png + roundtrip.json + printed metrics
#
# Usage:
#   scripts/roundtrip.sh <input.tex> [--out DIR]
#
# Env overrides: BYETEX_BIN (skip the cargo build), BYETEX_TECTONIC_BIN.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPTS_DIR="$REPO_ROOT/scripts"

# ── args ──────────────────────────────────────────────────────────────────────
INPUT=""; OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT="$2"; shift 2 ;;
    --*)   echo "Unknown flag: $1" >&2; exit 1 ;;
    *)     INPUT="$1"; shift ;;
  esac
done
[[ -z "$INPUT" ]] && { echo "usage: scripts/roundtrip.sh <input.tex> [--out DIR]" >&2; exit 1; }
[[ -f "$INPUT" ]] || { echo "input not found: $INPUT" >&2; exit 1; }

# Absolutize the input and derive defaults.
INPUT="$(cd "$(dirname "$INPUT")" && pwd)/$(basename "$INPUT")"
STEM="$(basename "${INPUT%.tex}")"
[[ -z "$OUT" ]] && OUT="$REPO_ROOT/target/roundtrip/$STEM"
mkdir -p "$OUT"

TECTONIC="${BYETEX_TECTONIC_BIN:-tectonic}"
if ! command -v "$TECTONIC" >/dev/null 2>&1; then
  echo "roundtrip needs tectonic to render the reference PDF — install it or set BYETEX_TECTONIC_BIN" >&2
  exit 1
fi

# ── byetex binary (build release unless BYETEX_BIN points at one) ─────────────
BYETEX="${BYETEX_BIN:-$REPO_ROOT/target/release/byetex}"
if [[ -z "${BYETEX_BIN:-}" && ! -x "$BYETEX" ]]; then
  echo "Building byetex (release)…" >&2
  cargo build --release -p byetex-cli --manifest-path "$REPO_ROOT/Cargo.toml" >&2
fi

# ── 1. Stage-0 oracle (best-effort attribution) ──────────────────────────────
echo "→ doctor (input validation)"
"$BYETEX" doctor "$INPUT" || true
doctor_sidecar="${INPUT%.tex}.doctor.json"
[[ -f "$doctor_sidecar" ]] && cp "$doctor_sidecar" "$OUT/doctor.json"

# ── 2. Convert + compile our output ──────────────────────────────────────────
echo "→ byetex convert --project"
"$BYETEX" convert --project "$INPUT" --project-out "$OUT/gen" --force >/dev/null 2>&1 || true
if [[ ! -f "$OUT/gen/main.typ" ]]; then
  echo "FAIL: byetex produced no main.typ" >&2; exit 1
fi
echo "→ typst compile"
if ! typst compile "$OUT/gen/main.typ" "$OUT/generated.pdf" 2>"$OUT/typst.log"; then
  echo "FAIL: generated Typst did not compile (see $OUT/typst.log)" >&2
  exit 1
fi

# ── 3-5. Render reference (truth), compare, composite — reuse visual_test.py ──
echo "→ tectonic render + structural compare"
uv run --with requests --with Pillow python - "$INPUT" "$OUT" "$STEM" "$SCRIPTS_DIR" <<'PY'
import json, sys
from pathlib import Path

inp, out, stem, scripts_dir = sys.argv[1:5]
sys.path.insert(0, scripts_dir)
import visual_test as vt

out = Path(out)
ref = out / "reference.pdf"
if not vt.render_reference_tectonic(Path(inp), ref):
    print("FAIL: tectonic could not render the reference PDF", file=sys.stderr)
    sys.exit(2)

gen = out / "generated.pdf"
truth_pages = vt.rasterize_pdf(ref, out / "pages" / "truth", 100)
typst_pages = vt.rasterize_pdf(gen, out / "pages" / "typst", 100)

# Permissive thresholds: roundtrip reports metrics, it does not gate.
s = vt.pdf_structure_compare(
    ref, gen, len(truth_pages), len(typst_pages),
    page_min=0.0, page_max=99.0,
    jaccard_min=0.0, word_recall_min=0.0, heading_recall_min=0.0,
)
vt.build_composite(truth_pages, typst_pages, out / "composite.png", stem)
(out / "roundtrip.json").write_text(json.dumps(s, indent=2) + "\n")

print(
    f"  word_jaccard={s['word_jaccard']:.2f} "
    f"word_recall={s['word_recall']:.2f} "
    f"heading_recall={s['heading_recall']:.2f} "
    f"pages={len(typst_pages)}/{len(truth_pages)}"
)
print(f"  composite:  {out / 'composite.png'}")
print(f"  metrics:    {out / 'roundtrip.json'}")
PY

echo "✓ round-trip complete → $OUT"
