param(
    [string]$Version = "latest",
    [string]$InstallDir = [System.IO.Path]::Combine($env:ProgramFiles, "Crush")
)

$ErrorActionPreference = "Stop"

$repo = "crushcontainer/crush"
$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }

if ($Version -eq "latest") {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $Version = $release.tag_name
}

$url = "https://github.com/$repo/releases/download/$Version/crush-windows-$arch.zip"

Write-Host "Installing Crush $Version..." -ForegroundColor Green

$tempDir = Join-Path $env:TEMP "crush-install"
New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
$zipPath = Join-Path $tempDir "crush.zip"

Invoke-WebRequest -Uri $url -OutFile $zipPath
Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Move-Item -Path (Join-Path $tempDir "crush.exe") -Destination (Join-Path $InstallDir "crush.exe") -Force

Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
    $env:Path += ";$InstallDir"
}

Write-Host "Installed to $InstallDir\crush.exe" -ForegroundColor Green
Write-Host ""
Write-Host "Run 'crush --help' to get started." -ForegroundColor Green
