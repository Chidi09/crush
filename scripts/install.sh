#!/usr/bin/env bash
set -euo pipefail

REPO="Chidi09/crush"
VERSION="${1:-latest}"

if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
esac

ASSET="crush-${VERSION}-${OS}-${ARCH}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"

TEMP=$(mktemp)
echo "Downloading Crush ${VERSION}..."
curl -fsSL "$URL" -o "$TEMP"
chmod +x "$TEMP"

echo "Installing..."
"$TEMP" install

rm -f "$TEMP"
echo "Done. Run: crush --version"
