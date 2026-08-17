#!/usr/bin/env bash
# Test for `corpus_sweep.sh --with-oracle` failure attribution.
#
# Builds a synthetic mini-corpus and fake `byetex` / `typst` tools so the
# INPUT_BROKEN vs BYETEX_FAIL bucketing is verified deterministically —
# no real corpus payloads and no Tectonic install required.
#
# Run: ./scripts/tests/corpus_sweep_oracle_test.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SWEEP="$REPO_ROOT/scripts/corpus_sweep.sh"
WORK="$REPO_ROOT/target/corpus-oracle-test.$$"   # under target/ (gitignored)
trap 'rm -rf "$WORK"' EXIT

CORPUS="$WORK/corpus"
BIN="$WORK/bin"
mkdir -p "$CORPUS" "$BIN"

# ── fake byetex: handles `convert --project <top>` and `doctor <top>` ─────────
# Behavior is driven by directives in the source .tex so each paper is
# self-describing: `typst=fail` makes our generated output un-compilable;
# `doctor=input_broken` makes the oracle blame the input.
# Args are matched by SHAPE (the *.tex operand, the value after --project-out)
# rather than by position. Positional fakes silently rot the moment a flag is
# added to the real invocation: this one read `$3` as the toplevel and wrote to
# `<stem>.typst-project`, so once corpus_sweep.sh started passing
# `--project-out`, every paper produced "no main.typ" and the whole test drifted
# to PASS: 0 without anyone noticing.
cat > "$BIN/byetex" <<'FAKE'
#!/bin/sh
cmd="$1"; shift
top=""; out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --project-out) out="$2"; shift 2 ;;
    *.tex) top="$1"; shift ;;
    *) shift ;;
  esac
done
case "$cmd" in
  convert)
    [ -n "$out" ] || out="${top%.tex}.typst-project"
    mkdir -p "$out"
    if grep -q 'typst=fail' "$top"; then printf 'BREAK\n' > "$out/main.typ"
    else printf '= ok\n' > "$out/main.typ"; fi
    ;;
  doctor)
    stem="${top%.tex}"
    if grep -q 'doctor=input_broken' "$top"; then v=input_broken
    elif grep -q 'doctor=unavailable' "$top"; then v=tectonic_unavailable
    else v=ok; fi
    printf '{ "verdict": "%s" }\n' "$v" > "${stem}.doctor.json"
    ;;
esac
exit 0
FAKE
chmod +x "$BIN/byetex"

# ── fake typst: errors when the generated .typ contains the BREAK marker ─────
# Same shape-matching rule: the real call is
# `typst compile --no-pdf-tags main.typ main.pdf`, so a fake that reads `$2` as
# the input greps a FLAG, never sees the BREAK marker, and truncates the source
# instead of writing the PDF.
cat > "$BIN/typst" <<'FAKE'
#!/bin/sh
[ "$1" = "compile" ] || exit 0
inp=""; outp=""
for a in "$@"; do
  case "$a" in
    *.typ) inp="$a" ;;
    *.pdf) outp="$a" ;;
  esac
done
if [ -n "$inp" ] && grep -q BREAK "$inp"; then
  echo "error: simulated typst failure"; exit 1
fi
[ -n "$outp" ] && : > "$outp"
exit 0
FAKE
chmod +x "$BIN/typst"

# ── synthetic corpus papers ──────────────────────────────────────────────────
make_paper() {  # <id> <directives>
  local id="$1" directives="$2"
  local src="$CORPUS/$id/source"
  # NOTE: deliberately do NOT create a `source.typst-project/` marker — the
  # freshly-harvested corpus layout (corpus/<id>/source/) has none, so the
  # sweep must process papers without that gate.
  mkdir -p "$src"
  printf '%% SWEEP %s\n\\documentclass{article}\n' "$directives" > "$src/main.tex"
  printf '{"sources":[{"usage":"toplevel","filename":"main.tex"}]}\n' > "$src/00README.json"
}
make_paper good        "typst=pass"
make_paper inputbroken "typst=fail doctor=input_broken"
make_paper ourbug      "typst=fail doctor=ok"
make_paper noattr      "typst=fail doctor=unavailable"

run_sweep() { BYETEX_BIN="$BIN/byetex" BYETEX_CORPUS_DIR="$CORPUS" PATH="$BIN:$PATH" \
              bash "$SWEEP" "$@" 2>&1; }

fail=0
check() {  # <output> <pattern> <description>
  if echo "$1" | grep -q "$2"; then echo "ok: $3"
  else echo "FAIL: expected $3 (pattern: $2)"; fail=1; fi
}

# ── oracle mode: failures are attributed ─────────────────────────────────────
oracle_out=$(run_sweep --with-oracle --summary)
echo "--- oracle output ---"; echo "$oracle_out"; echo "---------------------"
check "$oracle_out" "PASS: 1"         "1 PASS (good paper)"
check "$oracle_out" "INPUT_BROKEN: 1" "1 INPUT_BROKEN (broken source, not ByeTex's fault)"
check "$oracle_out" "BYETEX_FAIL: 1"  "1 BYETEX_FAIL (valid source, our output broke)"
check "$oracle_out" "UNATTRIBUTED: 1" "1 UNATTRIBUTED (oracle could not run, e.g. no tectonic)"

# ── default mode: backward-compatible single FAIL bucket ─────────────────────
plain_out=$(run_sweep --summary)
echo "--- default output ---"; echo "$plain_out"; echo "----------------------"
check "$plain_out" "PASS: 1" "1 PASS (default mode unchanged)"
check "$plain_out" "FAIL: 3" "3 FAIL collapsed (no attribution without --with-oracle)"

# ── the generated tree is not a paper ────────────────────────────────────────
# The first sweep above created `$CORPUS/_out/`, so this second sweep is the
# only place the bug is reachable: `for paper_dir in "$CORPUS"/*/` matched it,
# found no source/00README.json, and counted it as a SKIPPED PAPER. Every real
# corpus summary was reporting SKIP and TOTAL one too high.
check "$plain_out" "SKIP: 0"  "the generated _out/ tree is not counted as a skipped paper"
check "$plain_out" "TOTAL: 4" "TOTAL counts the 4 synthetic papers, not _out/"

if [[ $fail -ne 0 ]]; then echo "TEST FAILED"; exit 1; fi
echo "TEST PASSED"
