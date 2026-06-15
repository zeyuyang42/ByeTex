#!/usr/bin/env sh
# ByeTex installer — downloads a prebuilt `byetex` binary from GitHub Releases
# and installs it to a directory on (or near) your PATH.
#
#   curl -fsSL https://raw.githubusercontent.com/zeyuyang42/ByeTex/main/install.sh | sh
#
# Env:
#   BYETEX_VERSION  release tag to install (default: latest, e.g. v0.3.0)
#   BYETEX_BINDIR   install directory (default: $HOME/.local/bin)
#
# Windows: download the `.zip` from the Releases page instead.
set -eu

REPO="zeyuyang42/ByeTex"
BINDIR="${BYETEX_BINDIR:-${HOME}/.local/bin}"

err() { echo "byetex install: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || err "required tool not found: $1"; }
need uname
need tar
# curl or wget for downloads.
if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL "$1" -o "$2"; }
  fetch_stdout() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -qO "$2" "$1"; }
  fetch_stdout() { wget -qO- "$1"; }
else
  err "need curl or wget"
fi

# Map OS/arch to a release target triple (linux uses static musl builds).
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux)
    case "$arch" in
      x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
      aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
      *) err "unsupported Linux arch: $arch" ;;
    esac ;;
  Darwin)
    case "$arch" in
      x86_64) target="x86_64-apple-darwin" ;;
      arm64) target="aarch64-apple-darwin" ;;
      *) err "unsupported macOS arch: $arch" ;;
    esac ;;
  *) err "unsupported OS: $os (on Windows, download the .zip from the Releases page)" ;;
esac

# Resolve the version (latest release tag unless pinned).
ver="${BYETEX_VERSION:-}"
if [ -z "$ver" ]; then
  ver="$(fetch_stdout "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
fi
[ -n "$ver" ] || err "could not resolve the latest release tag; set BYETEX_VERSION"

name="byetex-${ver}-${target}"
url="https://github.com/${REPO}/releases/download/${ver}/${name}.tar.gz"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "byetex install: fetching ${url}"
fetch "$url" "$tmp/byetex.tar.gz" || err "download failed: $url"
tar xzf "$tmp/byetex.tar.gz" -C "$tmp" || err "extract failed"

bin="$tmp/${name}/byetex"
[ -f "$bin" ] || err "binary not found in archive (expected ${name}/byetex)"

mkdir -p "$BINDIR"
cp "$bin" "$BINDIR/byetex"
chmod 0755 "$BINDIR/byetex"
echo "byetex install: installed ${ver} → $BINDIR/byetex"

case ":$PATH:" in
  *":$BINDIR:"*) ;;
  *) echo "byetex install: add $BINDIR to your PATH, e.g. echo 'export PATH=\"$BINDIR:\$PATH\"' >> ~/.profile" ;;
esac

"$BINDIR/byetex" --version 2>/dev/null || true
echo "byetex install: done. Next: 'byetex convert paper.tex' or register the MCP server with 'byetex serve'."
