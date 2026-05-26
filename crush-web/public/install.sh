#!/bin/sh
set -eu

REPO="Chidi09/crush"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
VERSION="${VERSION:-latest}"

if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
fi

UNAME_S=$(uname -s)
UNAME_M=$(uname -m)

case "$UNAME_S" in
  Linux)  OS="linux" ;;
  Darwin) OS="macos" ;;
  *)      echo "Unsupported OS: $UNAME_S"; exit 1 ;;
esac

case "$UNAME_M" in
  x86_64 | amd64) ARCH="x86_64" ;;
  aarch64 | arm64) ARCH="aarch64" ;;
  *) echo "Unsupported arch: $UNAME_M"; exit 1 ;;
esac

URL="https://github.com/$REPO/releases/download/$VERSION/crush-$OS-$ARCH.tar.gz"

echo "Installing Crush $VERSION ($OS/$ARCH)..."
curl -fsSL "$URL" | tar xz -C "$INSTALL_DIR" crush
echo "Installed to $INSTALL_DIR/crush"
echo ""
echo "Run 'crush --help' to get started."
