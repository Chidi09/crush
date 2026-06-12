#!/usr/bin/env bash
# Local release pipeline — builds the CLI release binary and the Tauri GUI
# bundles for Linux, then collects artifacts + checksums into one folder.
#
#   scripts/release-local.sh v0.8.1            CLI + GUI
#   scripts/release-local.sh v0.8.1 cli-only   CLI only (much faster)
#
# Artifacts land in /srv/ci/releases/<version>/.
set -u
set -o pipefail

VERSION="${1:?usage: release-local.sh <version> [cli-only]}"
SCOPE="${2:-full}"

cd "${CI_WORKDIR:-$(dirname "$0")/..}"
REPO_ROOT="$PWD"
OUT="/srv/ci/releases/${VERSION}"
mkdir -p "$OUT"

echo "==> release ${VERSION} (${SCOPE}) from $(git rev-parse --short HEAD 2>/dev/null || echo 'worktree')"

# ---- CLI -------------------------------------------------------------
echo "==> building CLI (release)"
cargo build --release -p crush-cli || exit 1
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
cp "$TARGET_DIR/release/crush-cli" "$OUT/crush-${VERSION}-linux-x86_64"
strip "$OUT/crush-${VERSION}-linux-x86_64" 2>/dev/null || true

# ---- GUI -------------------------------------------------------------
if [ "$SCOPE" != "cli-only" ]; then
    echo "==> building GUI (tauri: deb + appimage)"
    (
        cd crates/crush-gui
        pnpm install --frozen-lockfile || exit 1
        # Config targets nsis/msi (Windows); on this Linux box we override.
        # deb is required; AppImage is best-effort (needs ~2GB free for
        # AppDir staging, so it's the first casualty of a tight disk).
        pnpm tauri build --bundles deb || exit 1
        APPIMAGE_EXTRACT_AND_RUN=1 pnpm tauri build --bundles appimage \
            || echo "WARN: AppImage bundling failed (non-fatal); .deb still shipped"
    ) || exit 1

    find "$TARGET_DIR/release/bundle/deb" -name '*.deb' -newer "$OUT" -exec cp {} "$OUT/" \; 2>/dev/null
    find "$TARGET_DIR/release/bundle/appimage" -name '*.AppImage' -newer "$OUT" -exec cp {} "$OUT/" \; 2>/dev/null
    # Fallback if -newer matched nothing (first run)
    [ -n "$(ls "$OUT"/*.deb 2>/dev/null)" ] || cp "$TARGET_DIR"/release/bundle/deb/*.deb "$OUT/" 2>/dev/null
    [ -n "$(ls "$OUT"/*.AppImage 2>/dev/null)" ] || cp "$TARGET_DIR"/release/bundle/appimage/*.AppImage "$OUT/" 2>/dev/null
fi

# ---- checksums + manifest --------------------------------------------
(
    cd "$OUT"
    rm -f SHA256SUMS
    sha256sum * > SHA256SUMS 2>/dev/null
)

echo ""
echo "================ release ${VERSION} ================"
ls -lh "$OUT" | tail -n +2 | awk '{printf "  %8s  %s\n", $5, $9}'
echo "===================================================="
echo "artifacts: $OUT"
