#!/usr/bin/env bash
# scripts/release.sh — bump version, build Windows binary, publish GitHub release,
# then push to CI so Linux artifacts are added automatically.
#
#   scripts/release.sh patch          0.8.1 → 0.8.2
#   scripts/release.sh minor          0.8.1 → 0.9.0
#   scripts/release.sh major          0.8.1 → 1.0.0
#   scripts/release.sh 1.2.3          exact version
#   scripts/release.sh patch --dry    preview only
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

DRY=0; [ "${2:-}" = "--dry" ] && DRY=1
BUMP="${1:?usage: release.sh patch|minor|major|<x.y.z> [--dry]}"

# ── read current version ──────────────────────────────────────────────────
CURRENT=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
[ -z "$CURRENT" ] && { echo "error: couldn't read version from Cargo.toml"; exit 1; }
IFS='.' read -r MAJ MIN PAT <<< "$CURRENT"

case "$BUMP" in
    patch)    NEW="$MAJ.$MIN.$((PAT + 1))" ;;
    minor)    NEW="$MAJ.$((MIN + 1)).0" ;;
    major)    NEW="$((MAJ + 1)).0.0" ;;
    [0-9]*.*) NEW="$BUMP" ;;
    *)        echo "error: unknown bump type '$BUMP'"; exit 1 ;;
esac

TAG="v$NEW"
REPO="Chidi09/crush"

echo "  current: $CURRENT"
echo "      new: $NEW  ($TAG)"
[ "$DRY" -eq 1 ] && { echo "(dry run — no changes made)"; exit 0; }

# ── guard ─────────────────────────────────────────────────────────────────
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "error: tag $TAG already exists"; exit 1
fi
if ! git diff --cached --quiet || ! git diff --quiet; then
    echo "error: uncommitted changes — commit or stash first"
    git status --short; exit 1
fi
if ! gh auth status &>/dev/null; then
    echo "error: gh not authenticated — run: gh auth login"; exit 1
fi

# ── bump version ──────────────────────────────────────────────────────────
echo ""
echo "==> bumping $CURRENT → $NEW"
sed -i "s/^version = \"${CURRENT}\"/version = \"${NEW}\"/" Cargo.toml

TAURI_CONF="crates/crush-gui/src-tauri/tauri.conf.json"
[ -f "$TAURI_CONF" ] && sed -i "s/\"version\": \"${CURRENT}\"/\"version\": \"${NEW}\"/" "$TAURI_CONF"

echo "    regenerating Cargo.lock..."
cargo metadata --format-version 1 > /dev/null 2>&1 || cargo fetch --quiet 2>/dev/null || true

FILES=(Cargo.toml Cargo.lock)
[ -f "$TAURI_CONF" ] && FILES+=("$TAURI_CONF")
git add "${FILES[@]}"
git commit -m "chore: bump version to $NEW"
git tag -a "$TAG" -m "Release $TAG"
echo "    committed and tagged $TAG"

# ── cross-compile Windows binary ──────────────────────────────────────────
echo ""
echo "==> cross-compiling Windows binary..."
source ~/.cargo/env 2>/dev/null || true
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo build --release --target x86_64-pc-windows-gnu -p crush-cli

WIN_BIN="target/x86_64-pc-windows-gnu/release/crush-cli.exe"
WIN_OUT="crush-${NEW}-windows-x86_64.exe"
x86_64-w64-mingw32-strip "$WIN_BIN" -o "/tmp/$WIN_OUT"
echo "    built $WIN_OUT ($(du -sh /tmp/$WIN_OUT | cut -f1))"

# ── push to origin ────────────────────────────────────────────────────────
echo ""
echo "==> pushing to origin..."
git push origin main "$TAG"

# ── create GitHub release (draft until CI adds Linux artifacts) ───────────
echo ""
echo "==> creating GitHub release $TAG..."
gh release create "$TAG" \
    --repo "$REPO" \
    --title "Crush $NEW" \
    --draft \
    --notes "$(cat <<NOTES
## Crush $NEW

### Install

**Linux** — artifacts being built, will appear shortly:
\`\`\`bash
curl -fsSL https://crush-web-six.vercel.app/install.sh | sh
\`\`\`

**Windows** — download \`crush-${NEW}-windows-x86_64.exe\` below and add to PATH:
\`\`\`powershell
\$dest = "\$env:USERPROFILE\\.local\\bin"
New-Item -ItemType Directory -Force -Path \$dest | Out-Null
Invoke-WebRequest -Uri (gh release view $TAG --repo $REPO --json assets -q '.assets[] | select(.name | endswith(".exe")) | .url') -OutFile "\$dest\\crush.exe"
\`\`\`

See the [changelog](https://crush-web-six.vercel.app/changelog) for what's new.
NOTES
)" \
    "/tmp/$WIN_OUT"

echo "    draft release created with Windows binary"

# ── trigger CI release build (Linux artifacts + auto-publish) ────────────
echo ""
echo "==> triggering CI release build..."
git push ci "$TAG"

echo ""
echo "✅  $TAG dispatched"
echo "    Windows binary: already in the GitHub release"
echo "    Linux artifacts: CI building now → will be uploaded + release published"
echo "    Track build:    tail -f /srv/ci/logs/release-${NEW//\//-}.log"
echo "    GitHub:         https://github.com/$REPO/releases/tag/$TAG"
