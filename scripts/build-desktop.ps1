<#
.SYNOPSIS
  Build the Crush desktop app (Tauri) installers on Windows.

.DESCRIPTION
  Runs on the WINDOWS LAPTOP. Sets up the VS 2022 BuildTools dev shell and the
  nvm4w Node 20 toolchain, then builds the SvelteKit frontend + Tauri bundle,
  producing NSIS (.exe) and MSI (.msi) installers.

  This is the workhorse — the Tauri bundle CANNOT be built on the Linux VPS
  (no host WebView / bundler). The VPS orchestrator (scripts/release-desktop.sh)
  ships the source here and invokes this script over SSH.

.PARAMETER RepoPath
  Path to the Crush repo on this machine. Default: C:\Users\X1\CHIDIS WORKSPACE\Crush

.PARAMETER OutDir
  Where to copy the finished installers. Default: <RepoPath>\dist

.PARAMETER BundlePath
  Optional path to a `git bundle` (shipped by the VPS). When given, the script
  hard-resets the repo to the bundle's `main` before building. OMIT this to just
  build whatever is currently checked out (non-destructive).

.EXAMPLE
  # Build the current working tree:
  powershell -ExecutionPolicy Bypass -File scripts\build-desktop.ps1

.EXAMPLE
  # Build from a source bundle the VPS pushed (used by release-desktop.sh):
  powershell -ExecutionPolicy Bypass -File scripts\build-desktop.ps1 -BundlePath C:\temp\crush-main.bundle
#>
param(
    [string]$RepoPath   = "C:\Users\X1\CHIDIS WORKSPACE\Crush",
    [string]$OutDir     = "",
    [string]$BundlePath = ""
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrEmpty($OutDir)) { $OutDir = Join-Path $RepoPath "dist" }

function Step($msg) { Write-Host "=== $msg ===" -ForegroundColor Cyan }

# ── 1. VS 2022 BuildTools dev shell (gives us cl.exe / linker for the ~520 crates)
Step "Loading VS 2022 BuildTools dev shell"
$vsLaunch = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1"
if (-not (Test-Path $vsLaunch)) { throw "VS BuildTools not found at $vsLaunch — install VS 2022 BuildTools (C++ workload)." }
& $vsLaunch -Arch amd64 -HostArch amd64 -SkipAutomaticLocation

# ── 2. Toolchain on PATH: nvm4w Node 20 + scoop shims (cargo/rustup). Order matters:
#       nvm4w node must win over any scoop Node 26 (which crashes vite — see toolchain notes).
Step "Pinning toolchain (nvm4w Node 20 + rustup MSVC)"
$env:PATH = "C:\nvm4w\nodejs;C:\Users\X1\scoop\shims;$env:PATH"
rustup default stable-x86_64-pc-windows-msvc | Out-Null
Write-Host ("node {0} / pnpm {1} / cargo {2}" -f (node --version), (pnpm --version), (cargo --version))

if (-not (Test-Path $RepoPath)) { throw "Repo not found at $RepoPath" }
Set-Location $RepoPath

# ── 3. Optional: sync to the source the VPS shipped (git bundle). Destructive reset.
if (-not [string]::IsNullOrEmpty($BundlePath)) {
    if (-not (Test-Path $BundlePath)) { throw "Bundle not found at $BundlePath" }
    Step "Syncing repo to shipped bundle ($BundlePath)"
    Write-Host "  WARNING: this hard-resets '$RepoPath' to the bundle's main branch." -ForegroundColor Yellow
    git fetch --no-tags "$BundlePath" "main:refs/remotes/bundle/main"
    git checkout -B main refs/remotes/bundle/main
    git reset --hard refs/remotes/bundle/main
}

$guiDir = Join-Path $RepoPath "crates\crush-gui"
Set-Location $guiDir

# ── 4. Frontend deps. CI=true so pnpm never blocks on the interactive purge prompt.
Step "Installing GUI dependencies (pnpm)"
$env:CI = "true"
pnpm install
if ($LASTEXITCODE -ne 0) { throw "pnpm install failed ($LASTEXITCODE)" }

# ── 5. Build the bundle. `pnpm tauri build` runs the frontend build (beforeBuildCommand)
#       then compiles the Rust side and emits NSIS + MSI installers.
Step "Building Tauri installers (this compiles the full release — needs several GB free)"
pnpm tauri build
if ($LASTEXITCODE -ne 0) { throw "tauri build failed ($LASTEXITCODE) — if 'os error 112', free disk space and retry." }

# ── 6. Collect artifacts. Tauri writes under the workspace target dir.
Step "Collecting installers -> $OutDir"
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$bundleRoots = @(
    (Join-Path $RepoPath "target\release\bundle"),
    (Join-Path $guiDir   "src-tauri\target\release\bundle")
) | Where-Object { Test-Path $_ }

$artifacts = $bundleRoots | ForEach-Object {
    Get-ChildItem -Path $_ -Recurse -Include *.exe, *.msi -ErrorAction SilentlyContinue
}
if (-not $artifacts) { throw "No .exe/.msi produced — check the build log above." }

foreach ($a in $artifacts) {
    Copy-Item $a.FullName -Destination $OutDir -Force
    Write-Host ("  {0}  ({1:N1} MB)" -f $a.Name, ($a.Length / 1MB)) -ForegroundColor Green
}

Step "DONE"
Write-Host "Installers are in: $OutDir" -ForegroundColor Green
# Machine-readable marker so the VPS orchestrator can locate the output dir:
Write-Output "CRUSH_DESKTOP_OUTDIR=$OutDir"
