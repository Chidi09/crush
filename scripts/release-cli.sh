#!/usr/bin/env bash
#
# release-cli.sh — self-hosted CLI release pipeline (no GitHub Actions).
#
# Builds the `crush` CLI on the VPS for Linux x86_64 (native) and Windows x86_64
# (mingw cross), packages them, then creates/updates the GitHub Release for the
# current version tag and uploads the assets via the REST API.
#
# GitHub Actions is intentionally NOT used (billing). Releases + asset uploads
# work through the plain API with a token, no runner minutes required.
#
# macOS / aarch64 are not built here — this is an x86_64 Linux host. Build those
# on the laptop / a Mac and upload them to the same release if needed.
#
# ── Usage ────────────────────────────────────────────────────────────────────
#   GITHUB_TOKEN=ghp_xxx ./scripts/release-cli.sh
#   GITHUB_TOKEN=ghp_xxx ./scripts/release-cli.sh --build-only   # skip upload
#
set -euo pipefail

REPO="Chidi09/crush"
BIN_CRATE="crush-cli"
BIN_NAME="crush-cli"      # produced binary name
SHIP_NAME="crush"          # name users get inside the archive

BUILD_ONLY=false
[[ "${1:-}" == "--build-only" ]] && BUILD_ONLY=true

cd "$(git rev-parse --show-toplevel)"
source "$HOME/.cargo/env" 2>/dev/null || true

VERSION="$(grep '^version' Cargo.toml | head -1 | sed 's/.*= "\(.*\)"/\1/')"
TAG="v${VERSION}"
DIST="dist/cli"
mkdir -p "$DIST"

echo "==> Self-hosted CLI release  version=${VERSION}  tag=${TAG}"

# ── Build ────────────────────────────────────────────────────────────────────
echo "==> [linux]   cargo build --release (x86_64-unknown-linux-gnu)"
cargo build --release --target x86_64-unknown-linux-gnu -p "$BIN_CRATE"

echo "==> [windows] cargo build --release (x86_64-pc-windows-gnu, mingw)"
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo build --release --target x86_64-pc-windows-gnu -p "$BIN_CRATE"

# ── Package ──────────────────────────────────────────────────────────────────
echo "==> Packaging"
LINUX_BIN="target/x86_64-unknown-linux-gnu/release/${BIN_NAME}"
WIN_BIN="target/x86_64-pc-windows-gnu/release/${BIN_NAME}.exe"

strip "$LINUX_BIN" 2>/dev/null || true
x86_64-w64-mingw32-strip "$WIN_BIN" 2>/dev/null || true

# Raw binaries — `crush update` downloads these bytes and writes them directly as
# the executable, so the asset names must match the updater exactly:
#   windows -> crush-<ver>-windows-x86_64.exe
#   linux   -> crush-<ver>-linux-x86_64   (no extension)
LINUX_ASSET="crush-${VERSION}-linux-x86_64"
WIN_ASSET="crush-${VERSION}-windows-x86_64.exe"

cp "$LINUX_BIN" "$DIST/$LINUX_ASSET"
cp "$WIN_BIN"   "$DIST/$WIN_ASSET"
chmod +x "$DIST/$LINUX_ASSET"

# Checksums
( cd "$DIST" && sha256sum "$LINUX_ASSET" "$WIN_ASSET" > "crush-${VERSION}-SHA256SUMS.txt" )
echo "==> Built assets:"
ls -lh "$DIST" | grep -E "$VERSION"

if [[ "$BUILD_ONLY" == "true" ]]; then
    echo "==> --build-only: skipping upload."
    exit 0
fi

# ── Publish (REST API, no Actions) ───────────────────────────────────────────
: "${GITHUB_TOKEN:?set GITHUB_TOKEN to publish}"
API="https://api.github.com/repos/${REPO}"
UP="https://uploads.github.com/repos/${REPO}"
auth=(-H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/vnd.github+json")

echo "==> Ensuring release ${TAG} exists"
rel_id="$(curl -s "${auth[@]}" "${API}/releases/tags/${TAG}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("id",""))')"
if [[ -z "$rel_id" ]]; then
    echo "    creating release"
    rel_id="$(curl -s "${auth[@]}" -X POST "${API}/releases" \
        -d "$(python3 -c 'import json,sys; print(json.dumps({"tag_name":sys.argv[1],"name":"Crush "+sys.argv[2],"body":"Crush '"$VERSION"' — self-hosted build (Linux x86_64, Windows x86_64). macOS/ARM forthcoming."}))' "$TAG" "$VERSION")" \
        | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')"
fi
echo "    release id=$rel_id"

upload_asset() {
    local file="$1" name; name="$(basename "$file")"
    local ctype="$2"
    # delete existing asset with same name (idempotent re-runs)
    curl -s "${auth[@]}" "${API}/releases/${rel_id}/assets" \
        | python3 -c "import sys,json;[print(a['id']) for a in json.load(sys.stdin) if a['name']=='$name']" \
        | while read -r aid; do [[ -n "$aid" ]] && curl -s "${auth[@]}" -X DELETE "${API}/releases/assets/${aid}" >/dev/null; done
    echo "    uploading $name"
    curl -s "${auth[@]}" -H "Content-Type: ${ctype}" \
        --data-binary @"$file" "${UP}/releases/${rel_id}/assets?name=${name}" \
        | python3 -c 'import sys,json;d=json.load(sys.stdin);print("      ->",d.get("browser_download_url",d.get("message","?")))'
}

# Remove any stale archive-named assets from earlier runs (the updater wants raw binaries).
for stale in "crush-${VERSION}-linux-x86_64.tar.gz" "crush-${VERSION}-windows-x86_64.zip"; do
    curl -s "${auth[@]}" "${API}/releases/${rel_id}/assets" \
        | python3 -c "import sys,json;[print(a['id']) for a in json.load(sys.stdin) if a['name']=='$stale']" \
        | while read -r aid; do [[ -n "$aid" ]] && { echo "    removing stale $stale"; curl -s "${auth[@]}" -X DELETE "${API}/releases/assets/${aid}" >/dev/null; }; done
done

upload_asset "$DIST/$LINUX_ASSET" "application/octet-stream"
upload_asset "$DIST/$WIN_ASSET"   "application/octet-stream"
upload_asset "$DIST/crush-${VERSION}-SHA256SUMS.txt" "text/plain"

echo "==> Done: https://github.com/${REPO}/releases/tag/${TAG}"
