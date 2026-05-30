<#
.SYNOPSIS
  Build the unified Crush installer (CLI + GUI, component-selectable) on Windows.

.DESCRIPTION
  Runs on the WINDOWS LAPTOP. Builds the CLI exe and the GUI (Tauri) exe, fetches
  the EnVar NSIS plugin and the WebView2 bootstrapper, then compiles
  installer/crush-setup.nsi with makensis into dist/Crush-Setup-<ver>.exe.

  The result is ONE installer whose Components page lets the user install the CLI,
  the GUI, or both.

.PARAMETER RepoPath
  Crush repo path. Default: the repo this script lives in.
.PARAMETER Version
  Version string for the installer. Default: read from src-tauri/tauri.conf.json.
.PARAMETER SkipBuild
  Skip the cargo/pnpm builds and just (re)compile the NSIS installer from existing exes.

.PREREQUISITES
  VS 2022 BuildTools (C++), nvm4w Node 20, pnpm, and NSIS (makensis). Internet access
  (to fetch the EnVar plugin + WebView2 bootstrapper, cached after first run).

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1
#>
param(
    [string]$RepoPath = "",
    [string]$Version  = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
function Step($m) { Write-Host "=== $m ===" -ForegroundColor Cyan }

if ([string]::IsNullOrEmpty($RepoPath)) {
    $RepoPath = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}
$guiDir   = Join-Path $RepoPath "crates\crush-gui"
$targetRel = Join-Path $RepoPath "target\release"
$distDir  = Join-Path $RepoPath "dist"
$buildDir = Join-Path $RepoPath "dist\_installer"   # scratch: plugin + bootstrapper
New-Item -ItemType Directory -Force -Path $distDir, $buildDir | Out-Null

if ([string]::IsNullOrEmpty($Version)) {
    $conf = Get-Content (Join-Path $guiDir "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
    $Version = $conf.version
}
Write-Host "Building Crush installer v$Version" -ForegroundColor Green

if (-not $SkipBuild) {
    # ── Toolchain ─────────────────────────────────────────────────────────────
    Step "Loading VS dev shell + Node toolchain"
    $vsLaunch = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1"
    if (Test-Path $vsLaunch) { & $vsLaunch -Arch amd64 -HostArch amd64 -SkipAutomaticLocation }
    $env:PATH = "C:\nvm4w\nodejs;C:\Users\X1\scoop\shims;$env:PATH"
    rustup default stable-x86_64-pc-windows-msvc | Out-Null

    # ── CLI exe ───────────────────────────────────────────────────────────────
    Step "Building CLI (crush-cli.exe)"
    Push-Location $RepoPath
    cargo build --release -p crush-cli
    if ($LASTEXITCODE -ne 0) { throw "CLI build failed" }
    Pop-Location

    # ── GUI exe (frontend must build first; tauri embeds it) ──────────────────
    Step "Building GUI (crush-gui.exe)"
    Push-Location $guiDir
    $env:CI = "true"
    pnpm install; if ($LASTEXITCODE -ne 0) { throw "pnpm install failed" }
    pnpm build;   if ($LASTEXITCODE -ne 0) { throw "frontend build failed" }
    Pop-Location
    Push-Location $RepoPath
    # --features custom-protocol is REQUIRED: without it Tauri builds in dev mode
    # and the webview loads http://localhost:1420 instead of the embedded assets.
    cargo build --release -p crush-gui --features custom-protocol
    if ($LASTEXITCODE -ne 0) { throw "GUI build failed" }
    Pop-Location
}

$cliExe = Join-Path $targetRel "crush-cli.exe"
$guiExe = Join-Path $targetRel "crush-gui.exe"
if (-not (Test-Path $cliExe)) { throw "Missing $cliExe (run without -SkipBuild)" }
if (-not (Test-Path $guiExe)) { throw "Missing $guiExe (run without -SkipBuild)" }

# ── EnVar NSIS plugin (for PATH editing) ─────────────────────────────────────
Step "Fetching EnVar NSIS plugin"
$pluginRoot = Join-Path $buildDir "EnVar"
$pluginDir  = Join-Path $pluginRoot "Plugins\x86-unicode"
if (-not (Test-Path (Join-Path $pluginDir "EnVar.dll"))) {
    $zip = Join-Path $buildDir "EnVar_plugin.zip"
    Invoke-WebRequest "https://nsis.sourceforge.io/mediawiki/images/7/7f/EnVar_plugin.zip" -OutFile $zip
    Expand-Archive -Path $zip -DestinationPath $pluginRoot -Force
}
if (-not (Test-Path (Join-Path $pluginDir "EnVar.dll"))) { throw "EnVar.dll not found after extract" }

# ── WebView2 evergreen bootstrapper (optional but recommended) ───────────────
Step "Fetching WebView2 bootstrapper"
$wv2 = Join-Path $buildDir "MicrosoftEdgeWebview2Setup.exe"
if (-not (Test-Path $wv2)) {
    try { Invoke-WebRequest "https://go.microsoft.com/fwlink/p/?LinkId=2124703" -OutFile $wv2 }
    catch { Write-Host "  (WebView2 bootstrapper download failed; installer will skip it)" -ForegroundColor Yellow }
}

# ── Compile the installer ────────────────────────────────────────────────────
Step "Compiling installer with makensis"
$makensis = (Get-Command makensis -ErrorAction SilentlyContinue).Source
if (-not $makensis) { $makensis = "C:\Program Files (x86)\NSIS\makensis.exe" }
if (-not (Test-Path $makensis)) { throw "makensis not found — install NSIS (https://nsis.sourceforge.io)" }

$nsi    = Join-Path $RepoPath "installer\crush-setup.nsi"
$outExe = Join-Path $distDir "Crush-Setup-$Version.exe"

$args = @(
    "/V3",
    "/X!addplugindir `"$pluginDir`"",
    "/DVERSION=$Version",
    "/DCLI_EXE=$cliExe",
    "/DGUI_EXE=$guiExe",
    "/DOUTFILE=$outExe"
)
if (Test-Path $wv2) { $args += "/DWV2_BOOT=$wv2" }
$args += $nsi

& $makensis @args
if ($LASTEXITCODE -ne 0) { throw "makensis failed ($LASTEXITCODE)" }

Step "DONE"
Write-Host "Installer: $outExe" -ForegroundColor Green
Write-Output "CRUSH_INSTALLER=$outExe"
