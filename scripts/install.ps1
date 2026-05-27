# Crush one-line installer for Windows
# Usage: iwr https://github.com/Chidi09/crush/releases/latest/download/crush-latest-windows-x86_64.exe -OutFile crush.exe; .\crush.exe install

param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

$Repo = "Chidi09/crush"
$BinName = "crush.exe"

if ($Version -eq "latest") {
    $Release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $Release.tag_name
}

$AssetName = "crush-$Version-windows-x86_64.exe"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$AssetName"
$TempFile = Join-Path $env:TEMP "crush-install.exe"

Write-Host "Downloading Crush $Version..."
Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile

Write-Host "Installing..."
& $TempFile install

Remove-Item $TempFile -Force
Write-Host "Done. Open a new terminal and run: crush --version"
