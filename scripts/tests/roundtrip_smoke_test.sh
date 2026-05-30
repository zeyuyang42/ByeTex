#!/usr/bin/env bash
# Smoke test for scripts/roundtrip.sh — runs a full round-trip on a tiny,
# self-contained LaTeX file and asserts the expected artifacts are produced.
#
# Skips cleanly (exit 0) if any required tool is missing, mirroring the
# `typst`-not-on-PATH skip in compile_check.rs.
#
# Run: ./scripts/tests/roundtrip_smoke_test.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUNDTRIP="$REPO_ROOT/scripts/roundtrip.sh"

for tool in tectonic typst uv pdftoppm pdftotext; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "skip: roundtrip_smoke — '$tool' not on PATH"; exit 0
  fi
done

# Use a debug byetex if present to avoid a slow release build in the test.
BIN_OVERRIDE=""
[[ -x "$REPO_ROOT/target/debug/byetex" ]] && BIN_OVERRIDE="$REPO_ROOT/target/debug/byetex"

WORK="$REPO_ROOT/target/roundtrip-smoke.$$"     # under target/ (gitignored)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$WORK/in"
cat > "$WORK/in/doc.tex" <<'TEX'
\documentclass{article}
\begin{document}
\section{Introduction}
Round-trip smoke test. Some prose so the word overlap is meaningful.
\begin{equation}
E = mc^2
\end{equation}
\section{Conclusion}
Done.
\end{document}
TEX

OUT="$WORK/out"
if [[ -n "$BIN_OVERRIDE" ]]; then
  BYETEX_BIN="$BIN_OVERRIDE" bash "$ROUNDTRIP" "$WORK/in/doc.tex" --out "$OUT"
else
  bash "$ROUNDTRIP" "$WORK/in/doc.tex" --out "$OUT"
fi

fail=0
need() {  # <path> <description>
  if [[ -s "$1" ]]; then echo "ok: $2"; else echo "FAIL: missing/empty $2 ($1)"; fail=1; fi
}
need "$OUT/reference.pdf" "reference.pdf (tectonic render of the source)"
need "$OUT/generated.pdf" "generated.pdf (byetex -> typst)"
need "$OUT/composite.png" "composite.png (side-by-side)"
need "$OUT/roundtrip.json" "roundtrip.json (metrics)"

if [[ -f "$OUT/roundtrip.json" ]]; then
  if grep -q '"word_jaccard"' "$OUT/roundtrip.json"; then
    echo "ok: roundtrip.json carries a word_jaccard metric"
  else
    echo "FAIL: roundtrip.json has no word_jaccard"; fail=1
  fi
fi

if [[ $fail -ne 0 ]]; then echo "TEST FAILED"; exit 1; fi
echo "TEST PASSED"
