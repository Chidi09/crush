<#
.SYNOPSIS
  Build the Crush Windows GUI installer and upload it to the GitHub release.

.DESCRIPTION
  One command — run this on your Windows laptop after `scripts/release.sh` has
  already bumped the version, tagged, and kicked off the Linux build on the VPS.

  It pulls the latest tag, builds the Tauri NSIS + MSI installers, then uploads
  them to the matching GitHub release.

.PARAMETER Tag
  The release tag to build and upload, e.g. "v0.8.2".
  Default: latest tag reachable from current HEAD.

.PARAMETER SkipBuild
  Re-upload already-built artifacts from .\dist without rebuilding.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\release-windows.ps1
  powershell -ExecutionPolicy Bypass -File scripts\release-windows.ps1 -Tag v0.8.2
#>
param(
    [string]$Tag       = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
function Step($m) { Write-Host "`n==> $m" -ForegroundColor Cyan }
function Ok($m)   { Write-Host "    $m" -ForegroundColor Green }
function Warn($m) { Write-Host "    WARNING: $m" -ForegroundColor Yellow }

# ── repo root (works whether called from repo dir or scripts\ dir) ─────────
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

# ── sync to latest ─────────────────────────────────────────────────────────
Step "Syncing repo"
git fetch --tags --quiet
if ($LASTEXITCODE -ne 0) { throw "git fetch failed — are you online?" }

if ([string]::IsNullOrEmpty($Tag)) {
    $Tag = git describe --tags --abbrev=0
}
Ok "target tag: $Tag"
git checkout $Tag --quiet
if ($LASTEXITCODE -ne 0) { throw "git checkout $Tag failed" }

# ── read version (strip leading v) ─────────────────────────────────────────
$Version = $Tag.TrimStart("v")
$DistDir = Join-Path $RepoRoot "dist"

# ── build ──────────────────────────────────────────────────────────────────
if (-not $SkipBuild) {
    Step "Building Tauri installers for $Tag"
    & (Join-Path $PSScriptRoot "build-desktop.ps1") -RepoPath $RepoRoot -OutDir $DistDir
    if ($LASTEXITCODE -ne 0) { throw "build-desktop.ps1 failed" }
} else {
    Warn "-SkipBuild set — using existing artifacts in $DistDir"
}

# ── find artifacts ─────────────────────────────────────────────────────────
Step "Locating installer artifacts"
$artifacts = Get-ChildItem -Path $DistDir -Include *.msi, *.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -notlike "crush-cli*" -and $_.Name -notlike "WebView2*" }

if (-not $artifacts) {
    throw "No installer artifacts found in $DistDir — run without -SkipBuild"
}
foreach ($a in $artifacts) { Ok "$($a.Name)  ($([math]::Round($a.Length/1MB, 1)) MB)" }

# ── upload to GitHub release ────────────────────────────────────────────────
Step "Uploading to GitHub release $Tag"

# gh must be on PATH and authenticated
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "gh CLI not found. Install from https://cli.github.com/ then run: gh auth login"
}
if (-not (gh auth status 2>&1 | Select-String "Logged in")) {
    throw "gh not authenticated. Run: gh auth login"
}

foreach ($a in $artifacts) {
    Write-Host "    uploading $($a.Name)..." -NoNewline
    gh release upload $Tag $a.FullName --repo Chidi09/crush --clobber
    if ($LASTEXITCODE -eq 0) { Ok " done" } else { Warn " upload failed for $($a.Name)" }
}

Step "Done"
Ok "Windows installers added to: https://github.com/Chidi09/crush/releases/tag/$Tag"
