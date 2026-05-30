# v0.8.0-alpha spike — run from repo root in Developer PowerShell for VS 2022
$ErrorActionPreference = "Stop"

# Load VS 2022 BuildTools environment
& "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1"
rustup default stable-x86_64-pc-windows-msvc | Out-Null

$root = "C:\Users\X1\CHIDIS WORKSPACE\Crush"
$guiDir = "$root\crates\crush-gui"

Write-Host "=== 1. Create Rust crate ===" -ForegroundColor Cyan
New-Item -ItemType Directory -Path "$guiDir\src-tauri\src" -Force | Out-Null
New-Item -ItemType Directory -Path "$guiDir\src-tauri\icons" -Force | Out-Null

Write-Host "=== 2. Init SvelteKit frontend ===" -ForegroundColor Cyan
Set-Location $guiDir
pnpm create svelte@latest . -- --template skeleton --types typescript --no-add-ons 2>&1
if ($LASTEXITCODE -ne 0) { Write-Host "pnpm create svelte returned $LASTEXITCODE, might already exist" -ForegroundColor Yellow }

Write-Host "=== 3. Install Tauri CLI + Tailwind + Tauri API ===" -ForegroundColor Cyan
pnpm install 2>&1
pnpm add -D @tauri-apps/cli@latest tailwindcss@latest @tailwindcss/vite 2>&1
pnpm add @tauri-apps/api@latest 2>&1

Write-Host "=== 4. Init Tauri (non-interactive) ===" -ForegroundColor Cyan
# Write tauri.conf.json manually
$tauriConf = @'
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Crush",
  "version": "0.8.0",
  "identifier": "run.crush.app",
  "app": {
    "windows": [{
      "title": "Crush",
      "width": 1200,
      "minWidth": 900,
      "height": 760,
      "minHeight": 600,
      "decorations": true,
      "transparent": false,
      "theme": "Dark",
      "center": true
    }],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"],
    "windows": {
      "webviewInstallMode": { "type": "downloadBootstrapper" },
      "nsis": { "displayLanguageSelector": false }
    }
  }
}
'@
$tauriConf | Out-File -FilePath "$guiDir\src-tauri\tauri.conf.json" -Encoding utf8

# Write capabilities
New-Item -ItemType Directory -Path "$guiDir\src-tauri\capabilities" -Force | Out-Null
$capabilities = @'
{
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
'@
$capabilities | Out-File -FilePath "$guiDir\src-tauri\capabilities\default.json" -Encoding utf8

Write-Host "=== 5. Copy design files ===" -ForegroundColor Cyan
Copy-Item "$root\crush-web\src\styles.css" -Destination "$guiDir\src\app.css" -Force
Copy-Item "$root\crush-web\tailwind.config.ts" -Destination "$guiDir\" -Force
Copy-Item "$root\crush-web\public\logo.svg" -Destination "$guiDir\src-tauri\icons\" -Force

Write-Host "=== 6. Create build.rs ===" -ForegroundColor Cyan
$buildRs = "fn main() { tauri_build::build() }"
$buildRs | Out-File -FilePath "$guiDir\src-tauri\build.rs" -Encoding utf8

Write-Host "=== 7. Write Rust Cargo.toml ===" -ForegroundColor Cyan
# We'll write this properly in the next step

Write-Host "=== DONE ====" -ForegroundColor Green
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Write src-tauri/Cargo.toml with workspace deps"
Write-Host "  2. Write main.rs, state.rs, commands/"
Write-Host "  3. Write Svelte layout + pages"
Write-Host "  4. pnpm exec tauri dev"
