#!/usr/bin/env bash
# Build the ByeTex release binaries locally (CI release is unreliable). Reproduces the
# 5 assets from the release.yml matrix from a macOS host, using cargo-zigbuild for the
# Linux-musl + Windows targets. Windows is built as -gnu (not -msvc) via zig, matching
# the v0.3.0 release. Usage: scripts/build_release_local.sh v0.5.20
set -euo pipefail
TAG="${1:?usage: build_release_local.sh <vX.Y.Z>}"
cd "$(dirname "$0")/.."
ROOT="$PWD"
DIST="$ROOT/dist"
rm -rf "$DIST"; mkdir -p "$DIST"

# target -> build tool (zig for cross musl/windows, plain cargo for apple)
NATIVE="aarch64-apple-darwin x86_64-apple-darwin"
ZIG="x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu"

build_one() {
  local triple="$1" tool="$2"
  echo "=== building $triple ($tool) ==="
  if [[ "$tool" == zig ]]; then
    cargo zigbuild -p byetex --release --target "$triple" --features mcp
  else
    cargo build -p byetex --release --target "$triple" --features mcp
  fi
  local name="byetex-${TAG}-${triple}"
  local stage="$DIST/$name"
  mkdir -p "$stage"
  if [[ "$triple" == *windows* ]]; then
    cp "target/$triple/release/byetex.exe" "$stage/"
    ( cd "$DIST" && 7z a "${name}.zip" "$name" >/dev/null || zip -rq "${name}.zip" "$name" )
  else
    cp "target/$triple/release/byetex" "$stage/"
    ( cd "$DIST" && tar czf "${name}.tar.gz" "$name" )
  fi
}

for t in $NATIVE; do build_one "$t" cargo; done
for t in $ZIG;    do build_one "$t" zig;   done

( cd "$DIST" && shasum -a 256 byetex-*.tar.gz byetex-*.zip > SHA256SUMS )
echo "=== artifacts ==="
ls -la "$DIST"/*.tar.gz "$DIST"/*.zip "$DIST"/SHA256SUMS
echo "BUILD_OK"
