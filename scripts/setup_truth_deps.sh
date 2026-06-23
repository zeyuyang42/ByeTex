#!/usr/bin/env bash
# Provision the dependencies the TRUTH renderer (tectonic) needs so that every corpus
# paper's original LaTeX compiles to a reference PDF. Without this, papers that use a
# biblatex/biber backend or a bespoke font (e.g. Roboto Slab in the TU Delft thesis class)
# silently fail to render a truth — and the fidelity DRIVER goes blind on them
# (the "truth_render_failed" papers in scripts/fidelity_baseline.json).
#
# Idempotent. Installs into a repo-local `.truth-deps/` (gitignored) plus the OS font dir
# (tectonic resolves fonts via the system, like macOS ~/Library/Fonts). `render_reference_tectonic`
# in scripts/visual_test.py prepends `.truth-deps/bin` to PATH so it picks up the matching biber.
#
# Usage: scripts/setup_truth_deps.sh   (re-run any time; skips work already done)
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEPS="$REPO_ROOT/.truth-deps"
BIN="$DEPS/bin"; FONTS="$DEPS/fonts"
mkdir -p "$BIN" "$FONTS"
uname_s="$(uname -s)"

note() { printf '  %s\n' "$*"; }

# 1) tectonic — the reference engine. We don't auto-install it (it's a big toolchain);
#    just check and point the user at the install.
if command -v tectonic >/dev/null 2>&1; then
  note "tectonic: $(tectonic --version 2>&1 | head -1)"
else
  echo "ERROR: tectonic not found. Install it (macOS: 'brew install tectonic'; " >&2
  echo "       see https://tectonic-typesetting.org/) then re-run." >&2
  exit 1
fi

# 2) biber — tectonic shells out to an EXTERNAL biber for biblatex papers, and the version
#    must match tectonic's bundled biblatex (control-file format). Pin 2.17 (bcf 3.8).
BIBER="$BIN/biber"
if [ -x "$BIBER" ] && "$BIBER" --version 2>/dev/null | grep -q "2.17"; then
  note "biber: $("$BIBER" --version 2>&1 | head -1) (cached)"
else
  case "$uname_s" in
    Darwin) biber_asset="biber-darwin_x86_64.tar.gz" ;;   # runs via Rosetta on Apple Silicon
    Linux)  biber_asset="biber-linux_x86_64.tar.gz" ;;
    *) echo "ERROR: unsupported OS for biber auto-install: $uname_s" >&2; exit 1 ;;
  esac
  url="https://downloads.sourceforge.net/project/biblatex-biber/biblatex-biber/2.17/binaries/MacOS/$biber_asset"
  [ "$uname_s" = Linux ] && url="https://downloads.sourceforge.net/project/biblatex-biber/biblatex-biber/2.17/binaries/Linux/$biber_asset"
  note "downloading biber 2.17 ($biber_asset) …"
  tmp="$(mktemp -d "$DEPS/.biber.XXXXXX")"
  curl -fsSL "$url" -o "$tmp/biber.tgz"
  tar xzf "$tmp/biber.tgz" -C "$tmp"
  found="$(find "$tmp" -name biber -type f | head -1)"
  [ -n "$found" ] || { echo "ERROR: biber binary not found in archive" >&2; exit 1; }
  cp "$found" "$BIBER"; chmod +x "$BIBER"; rm -rf "$tmp"
  note "biber: $("$BIBER" --version 2>&1 | head -1)"
fi

# 3) Fonts — instantiate NAMED static weights from the Roboto Slab variable font so fontspec
#    finds "Roboto Slab Light"/"Roboto Slab Thin" (the TU Delft thesis class). Add more fonts
#    here as new corpus papers need them.
case "$uname_s" in
  Darwin) OSFONTS="$HOME/Library/Fonts" ;;
  Linux)  OSFONTS="$HOME/.local/share/fonts" ;;
esac
mkdir -p "$OSFONTS"
if [ -f "$FONTS/RobotoSlab-Light.ttf" ] && [ -f "$OSFONTS/RobotoSlab-Light.ttf" ]; then
  note "fonts: Roboto Slab (cached)"
else
  note "fetching + instantiating Roboto Slab static weights …"
  vf="$FONTS/RobotoSlab-VF.ttf"
  [ -f "$vf" ] || curl -fsSL \
    "https://github.com/google/fonts/raw/main/apache/robotoslab/RobotoSlab%5Bwght%5D.ttf" -o "$vf"
  uv run --with fonttools python - "$vf" "$FONTS" "$OSFONTS" <<'PY'
import sys
from fontTools import ttLib
from fontTools.varLib import instancer
vf, fonts_dir, os_dir = sys.argv[1], sys.argv[2], sys.argv[3]
for wght, label in [(100, "Thin"), (300, "Light"), (400, "Regular"), (700, "Bold")]:
    f = ttLib.TTFont(vf)
    instancer.instantiateVariableFont(f, {"wght": wght}, inplace=True, updateFontNames=True)
    for d in (fonts_dir, os_dir):
        f.save(f"{d}/RobotoSlab-{label}.ttf")
    print(f"  Roboto Slab {label}: family='{f['name'].getDebugName(1)}'")
PY
fi

# 3b) Carlito (Calibri-metric font) — used by the Oxford eng-thesis class (gh-maurovm).
carlito_complete=1
for w in Regular Bold Italic BoldItalic; do
  [ -f "$OSFONTS/Carlito-$w.ttf" ] || carlito_complete=0
done
if [ "$carlito_complete" = 1 ]; then
  note "fonts: Carlito (cached)"
else
  note "fetching Carlito …"
  for w in Regular Bold Italic BoldItalic; do
    # Download to a temp then move, so an interrupted fetch never leaves a partial set
    # that a re-run would treat as complete (all four are re-checked above).
    curl -fsSL "https://github.com/google/fonts/raw/main/ofl/carlito/Carlito-$w.ttf" \
      -o "$FONTS/Carlito-$w.ttf.part"
    mv "$FONTS/Carlito-$w.ttf.part" "$FONTS/Carlito-$w.ttf"
    cp "$FONTS/Carlito-$w.ttf" "$OSFONTS/Carlito-$w.ttf"
  done
  note "fonts: Carlito installed"
fi

echo "OK: truth deps ready in $DEPS (biber on PATH via render_reference_tectonic; fonts in $OSFONTS)"
