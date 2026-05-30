#!/usr/bin/env bash
#
# release-desktop.sh — build the Crush desktop installers without GitHub.
#
# Runs on the VPS. Ships the current source to the Windows laptop as a git
# bundle (over scp), invokes the Windows build (scripts/build-desktop.ps1) over
# ssh, then pulls the finished NSIS/MSI installers back into ./dist on the VPS.
#
# The desktop (Tauri) app CANNOT be built on this Linux VPS — it needs the host
# WebView + Windows bundler. This script is the relay: VPS source -> laptop build
# -> installers back on the VPS.
#
# ── Prerequisites ────────────────────────────────────────────────────────────
#   * The laptop is reachable from the VPS over SSH (direct, Tailscale, or a
#     reverse tunnel) AND has OpenSSH enabled. If the laptop is NOT reachable
#     from the VPS, skip this script and just run scripts/build-desktop.ps1
#     directly on the laptop.
#   * On the laptop: VS 2022 BuildTools (C++), nvm4w Node 20, rustup MSVC.
#   * LAPTOP_REPO should be a DEDICATED build checkout, not your live dev tree —
#     the bundle sync does `git reset --hard`.
#
# ── Usage ────────────────────────────────────────────────────────────────────
#   LAPTOP_SSH=x1@laptop ./scripts/release-desktop.sh
#
#   Override anything via env:
#     LAPTOP_SSH    ssh target or ~/.ssh/config alias        (required)
#     LAPTOP_REPO   repo path on the laptop (Windows path)   (default below)
#     LAPTOP_TMP    temp dir on the laptop for the bundle    (default below)
#     DIST_DIR      where installers land on the VPS         (default ./dist)
#     GIT_REF       ref to ship                              (default: current HEAD)
#
set -euo pipefail

# ── Config ───────────────────────────────────────────────────────────────────
LAPTOP_SSH="${LAPTOP_SSH:-}"
LAPTOP_REPO="${LAPTOP_REPO:-C:/Users/X1/CHIDIS WORKSPACE/Crush}"
LAPTOP_TMP="${LAPTOP_TMP:-C:/temp}"
GIT_REF="${GIT_REF:-HEAD}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${DIST_DIR:-$REPO_ROOT/dist}"

if [[ -z "$LAPTOP_SSH" ]]; then
    echo "ERROR: set LAPTOP_SSH (e.g. LAPTOP_SSH=x1@laptop $0)" >&2
    exit 2
fi

cd "$REPO_ROOT"
SHA="$(git rev-parse --short "$GIT_REF")"
VERSION="$(grep -m1 '"version"' crates/crush-gui/src-tauri/tauri.conf.json | sed -E 's/.*"version": *"([^"]+)".*/\1/')"
BUNDLE="/tmp/crush-${SHA}.bundle"
REMOTE_BUNDLE="${LAPTOP_TMP}/crush-${SHA}.bundle"

echo "==> Crush desktop release"
echo "    version : ${VERSION}"
echo "    source  : ${GIT_REF} (${SHA})"
echo "    laptop  : ${LAPTOP_SSH}:${LAPTOP_REPO}"
echo "    dist    : ${DIST_DIR}"

# ── 1. Bundle the source (full history of the target ref, no GitHub needed) ──
echo "==> [1/4] Creating git bundle"
git bundle create "$BUNDLE" "${GIT_REF}" main 2>/dev/null || git bundle create "$BUNDLE" "${GIT_REF}"

# ── 2. Ship it to the laptop ────────────────────────────────────────────────
echo "==> [2/4] Copying bundle to laptop"
ssh "$LAPTOP_SSH" "mkdir \"${LAPTOP_TMP}\" 2>NUL & echo ok" >/dev/null 2>&1 || true
scp "$BUNDLE" "${LAPTOP_SSH}:${REMOTE_BUNDLE}"

# ── 3. Build on the laptop (PowerShell drives the VS dev shell + tauri build) ─
echo "==> [3/4] Building on laptop (this can take several minutes)"
PS_CMD="powershell -ExecutionPolicy Bypass -File \"${LAPTOP_REPO}\\scripts\\build-desktop.ps1\" -RepoPath \"${LAPTOP_REPO}\" -BundlePath \"${REMOTE_BUNDLE}\""
# Capture output so we can read back the CRUSH_DESKTOP_OUTDIR marker.
BUILD_LOG="$(ssh "$LAPTOP_SSH" "$PS_CMD" | tee /dev/stderr)"
REMOTE_OUT="$(printf '%s\n' "$BUILD_LOG" | grep -oE 'CRUSH_DESKTOP_OUTDIR=.*' | tail -1 | cut -d= -f2- | tr -d '\r')"
[[ -z "$REMOTE_OUT" ]] && REMOTE_OUT="${LAPTOP_REPO}/dist"

# ── 4. Pull the installers back ─────────────────────────────────────────────
echo "==> [4/4] Fetching installers from ${REMOTE_OUT}"
mkdir -p "$DIST_DIR"
scp "${LAPTOP_SSH}:${REMOTE_OUT}/*.exe" "$DIST_DIR/" 2>/dev/null || echo "    (no .exe found)"
scp "${LAPTOP_SSH}:${REMOTE_OUT}/*.msi" "$DIST_DIR/" 2>/dev/null || echo "    (no .msi found)"

rm -f "$BUNDLE"
echo "==> Done. Installers in ${DIST_DIR}:"
ls -lh "$DIST_DIR" 2>/dev/null | grep -E '\.(exe|msi)$' || echo "    (nothing — check the build log above)"
