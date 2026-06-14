param(
    [string]$Version = "latest",
    [string]$InstallDir = [System.IO.Path]::Combine($env:LOCALAPPDATA, "Crush")
)

$ErrorActionPreference = "Stop"

# Crush installer. Downloads the prebuilt Windows x86_64 CLI binary from the
# latest GitHub release and puts it on your user PATH. The release asset is a
# raw .exe named  crush-<version>-windows-x86_64.exe  (version without the 'v'),
# so we resolve the tag, strip the leading 'v', then fetch that exact asset.

$repo = "Chidi09/crush"

if ($Version -eq "latest") {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $Version = $release.tag_name
}
if (-not $Version) { throw "Could not resolve latest version." }

$verNoV = $Version.TrimStart("v")
$asset = "crush-$verNoV-windows-x86_64.exe"
$url = "https://github.com/$repo/releases/download/$Version/$asset"

Write-Host "Installing Crush $Version..." -ForegroundColor Green

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$dest = Join-Path $InstallDir "crush.exe"
Invoke-WebRequest -Uri $url -OutFile $dest

# Add to the user PATH (no admin needed).
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    $env:Path += ";$InstallDir"
}

Write-Host "Installed to $dest" -ForegroundColor Green
Write-Host ""
Write-Host "Run 'crush --help' to get started (open a new terminal first)." -ForegroundColor Green
