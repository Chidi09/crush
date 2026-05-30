# Crush GUI setup script — run from repo root
# Load VS 2022 BuildTools environment
$vsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Launch-VsDevShell.ps1"
if (Test-Path $vsPath) {
    & $vsPath
}

Write-Host "=== Checking toolchain ==="
rustup default stable-x86_64-pc-windows-msvc 2>&1
cl.exe 2>&1 | Select-Object -First 1
cargo --version
node --version
pnpm --version

Write-Host "=== Step 1: Create Rust crate ==="
$guiDir = "crates/crush-gui"
if (-not (Test-Path "$guiDir/src-tauri")) {
    New-Item -ItemType Directory -Path "$guiDir/src-tauri/src" -Force
    New-Item -ItemType Directory -Path "$guiDir/src-tauri/icons" -Force
} else {
    Write-Host "  src-tauri already exists"
}

Write-Host "=== Step 2: Init SvelteKit frontend ==="
if (-not (Test-Path "$guiDir/src/routes")) {
    Push-Location $guiDir
    pnpm create svelte@latest . -- --template skeleton --types typescript --no-add-ons 2>&1
    Pop-Location
} else {
    Write-Host "  SvelteKit already exists"
}

Write-Host "=== Step 3: Install Tauri CLI + Tailwind ==="
Push-Location $guiDir
pnpm install 2>&1
pnpm add -D @tauri-apps/cli@latest tailwindcss@latest @tailwindcss/vite 2>&1
pnpm add @tauri-apps/api@latest 2>&1
Pop-Location

Write-Host "=== Step 4: Tauri init ==="
# We'll do this manually since tauri init is interactive

Write-Host "=== Step 5: Copy design files ==="
Copy-Item "crush-web/src/styles.css" -Destination "$guiDir/src/app.css" -Force
Copy-Item "crush-web/tailwind.config.ts" -Destination "$guiDir/" -Force
Copy-Item "crush-web/public/logo.svg" -Destination "$guiDir/src-tauri/icons/" -Force

Write-Host "=== DONE ==="
Write-Host "Next: pnpm exec tauri init from crates/crush-gui/"
