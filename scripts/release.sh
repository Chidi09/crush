#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/release.sh              # build + tag as v$(version) + upload to GitHub Releases
#   ./scripts/release.sh --build-only # just build, skip release upload

BUILD_ONLY=false
[[ "${1:-}" == "--build-only" ]] && BUILD_ONLY=true

cd "$(git rev-parse --show-toplevel)"

echo "==> Pulling latest changes..."
git pull --ff-only

# Extract version from workspace Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*= "\(.*\)"/\1/')
TAG="v${VERSION}"
echo "==> Version: ${VERSION} (tag: ${TAG})"

echo "==> Cross-compiling for Windows..."
source ~/.cargo/env
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  cargo build --release --target x86_64-pc-windows-gnu -p crush-cli

BIN="target/x86_64-pc-windows-gnu/release/crush-cli.exe"
DIST="crush-${VERSION}-windows-x86_64.exe"

echo "==> Stripping binary..."
x86_64-w64-mingw32-strip "$BIN" -o "/root/$DIST"
cp "/root/$DIST" /root/crush.exe
echo "==> Built: $DIST ($(du -sh /root/$DIST | cut -f1))"

if [[ "$BUILD_ONLY" == "true" ]]; then
  echo "==> Build-only mode, skipping release upload."
  echo "    Download with:  scp safe-meet:/root/crush.exe ."
  exit 0
fi

if ! gh auth status &>/dev/null; then
  echo ""
  echo "ERROR: gh is not authenticated."
  echo "Run this on the VPS first:"
  echo "  gh auth login"
  echo "  (choose GitHub.com -> HTTPS -> paste a token)"
  echo ""
  echo "Or set a token env var: export GH_TOKEN=ghp_..."
  exit 1
fi

echo "==> Creating/updating GitHub Release ${TAG}..."
REPO="Chidi09/crush"

# Delete old release with same tag if it exists (makes this idempotent)
gh release delete "$TAG" --repo "$REPO" --yes 2>/dev/null || true
git tag -d "$TAG" 2>/dev/null || true
git push origin ":refs/tags/$TAG" 2>/dev/null || true

git tag "$TAG"
git push origin "$TAG"

gh release create "$TAG" \
  --repo "$REPO" \
  --title "Crush ${VERSION}" \
  --notes "Windows x86_64 build from $(date -u '+%Y-%m-%d %H:%M UTC').

## Install

Download \`crush-${VERSION}-windows-x86_64.exe\` below and place it on your PATH:

\`\`\`powershell
New-Item -ItemType Directory -Force -Path C:\crush
Invoke-WebRequest '<download-url>' -OutFile C:\crush\crush.exe
\$env:PATH += ';C:\crush'
[Environment]::SetEnvironmentVariable('PATH', \$env:PATH, 'User')
\`\`\`" \
  "/root/$DIST"

echo ""
echo "==> Released: https://github.com/${REPO}/releases/tag/${TAG}"
