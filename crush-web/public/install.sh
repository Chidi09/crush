#!/bin/sh
set -eu

# Crush installer. Downloads the prebuilt Linux x86_64 binary from the latest
# GitHub release and drops it on your PATH. Release assets are version-pinned
# raw binaries named  crush-<version>-linux-x86_64  (no tarball), so this script
# resolves the version first, then fetches that exact asset.

REPO="Chidi09/crush"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
VERSION="${VERSION:-latest}"

if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | head -n1 | cut -d'"' -f4)
fi
[ -n "$VERSION" ] || { echo "Could not resolve latest version."; exit 1; }

UNAME_S=$(uname -s)
UNAME_M=$(uname -m)

case "$UNAME_M" in
  x86_64 | amd64) ARCH="x86_64" ;;
  *)
    echo "No prebuilt binary for architecture: $UNAME_M"
    echo "Build from source: cargo install --git https://github.com/$REPO crush-cli"
    exit 1
    ;;
esac

case "$UNAME_S" in
  Linux) ;;
  Darwin)
    echo "No prebuilt macOS binary yet (signed bundles coming soon)."
    echo "Build from source: cargo install --git https://github.com/$REPO crush-cli"
    exit 1
    ;;
  *)
    echo "Unsupported OS: $UNAME_S"
    exit 1
    ;;
esac

ASSET="crush-${VERSION}-linux-${ARCH}"
URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET"

echo "Installing Crush $VERSION (linux/$ARCH)..."
TMP=$(mktemp)
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP" "$INSTALL_DIR/crush"
else
  echo "Elevating with sudo to write to $INSTALL_DIR..."
  sudo mv "$TMP" "$INSTALL_DIR/crush"
fi

echo "Installed to $INSTALL_DIR/crush"
echo ""
echo "Run 'crush --help' to get started."
