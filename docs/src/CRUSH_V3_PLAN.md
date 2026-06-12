# Crush v0.3.0 Implementation Plan

Three focus areas: global PATH self-installation, detector correctness + coverage,
and a readable `crush inspect` output. Work through tasks in order — dependencies
are noted where they exist.

**Rules:**
- Do NOT add `Co-Authored-By: Claude` or any AI trailer to git commits.
- Workspace root `Cargo.toml` is at repo root; prefer workspace-level deps.
- Cross-compile target: `x86_64-pc-windows-gnu` on Linux VPS (safe-meet).
- After all tasks: bump version to `0.3.0` in workspace `Cargo.toml`, build,
  tag, and release.

---

## PART A — Global PATH Installation

### Task A1 — Add `winreg` to workspace and crush-cli dependencies

**File:** `Cargo.toml` (workspace root)

Add to `[workspace.dependencies]`:
```toml
winreg = "0.52"
```

**File:** `crates/crush-cli/Cargo.toml`

Add under `[target.'cfg(target_os = "windows")'.dependencies]`:
```toml
winreg = { workspace = true }
```

---

### Task A2 — Add `crush install` subcommand to CLI

**File:** `crates/crush-cli/src/main.rs`

**Step 1 — Add struct and enum variant.**

Find the `Commands` enum and add:
```rust
/// Install crush to a system directory and add it to PATH
Install,
```

**Step 2 — Implement the handler.**

Find the `match` on `Commands::` and add the following arm. It must be placed
BEFORE the `Commands::Help` or catch-all arm.

```rust
Commands::Install => {
    cmd_install().await?;
}
```

**Step 3 — Add the `cmd_install` function** somewhere near the bottom of main.rs,
before `main()`:

```rust
async fn cmd_install() -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine current exe path: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        install_windows(&current_exe)?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        install_unix(&current_exe)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows(current_exe: &std::path::Path) -> anyhow::Result<()> {
    use std::ffi::OsString;

    // Target: %LOCALAPPDATA%\crush\bin\crush.exe
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| format!("{}\\AppData\\Local", std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
    let install_dir = std::path::PathBuf::from(&local_app_data).join("crush").join("bin");
    let install_path = install_dir.join("crush.exe");

    std::fs::create_dir_all(&install_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create install dir {:?}: {}", install_dir, e))?;

    // Copy the executable (skip copy if already running from the install path)
    if current_exe != install_path {
        std::fs::copy(current_exe, &install_path)
            .map_err(|e| anyhow::anyhow!("Failed to copy crush.exe to {:?}: {}", install_path, e))?;
    }

    // Read current HKCU\Environment\Path
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let env_key = hkcu.open_subkey_with_flags("Environment", winreg::enums::KEY_READ | winreg::enums::KEY_WRITE)
        .map_err(|e| anyhow::anyhow!("Failed to open HKCU\\Environment: {}", e))?;

    let current_path: String = env_key.get_value("Path").unwrap_or_default();
    let install_dir_str = install_dir.to_string_lossy().to_string();

    // Only add if not already present
    let already_in_path = current_path
        .split(';')
        .any(|p| p.trim().eq_ignore_ascii_case(&install_dir_str));

    if !already_in_path {
        let new_path = if current_path.is_empty() {
            install_dir_str.clone()
        } else if current_path.ends_with(';') {
            format!("{}{}", current_path, install_dir_str)
        } else {
            format!("{};{}", current_path, install_dir_str)
        };
        env_key.set_value("Path", &new_path)
            .map_err(|e| anyhow::anyhow!("Failed to write PATH to registry: {}", e))?;

        // Broadcast WM_SETTINGCHANGE so Explorer and new terminals pick up the change
        // without requiring a logoff/reboot.
        broadcast_setting_change();
    }

    println!("crush installed to: {}", install_path.display());
    if already_in_path {
        println!("PATH already contains {}  (no change needed)", install_dir_str);
    } else {
        println!("Added {} to user PATH.", install_dir_str);
        println!("Open a new terminal and run: crush --version");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_setting_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG};
    use windows_sys::Win32::Foundation::LPARAM;
    let env_wide: Vec<u16> = "Environment\0".encode_utf16().collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env_wide.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            1000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn install_unix(current_exe: &std::path::Path) -> anyhow::Result<()> {
    // Prefer ~/.local/bin (no sudo needed, on PATH in modern distros)
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("$HOME not set"))?;
    let install_dir = std::path::PathBuf::from(&home).join(".local").join("bin");
    let install_path = install_dir.join("crush");

    std::fs::create_dir_all(&install_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create {:?}: {}", install_dir, e))?;

    std::fs::copy(current_exe, &install_path)
        .map_err(|e| anyhow::anyhow!("Failed to copy to {:?}: {}", install_path, e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_path, perms)?;
    }

    println!("crush installed to: {}", install_path.display());
    // Check if install_dir is in PATH
    let path_env = std::env::var("PATH").unwrap_or_default();
    let in_path = path_env.split(':').any(|p| p == install_dir.to_string_lossy().as_ref());
    if !in_path {
        println!("NOTE: {} is not in your PATH.", install_dir.display());
        println!("Add this line to ~/.bashrc or ~/.zshrc:");
        println!("  export PATH=\"$HOME/.local/bin:$PATH\"");
    } else {
        println!("Open a new terminal and run: crush --version");
    }
    Ok(())
}
```

**Step 4 — Add `windows_sys` features needed for `WM_SETTINGCHANGE`.**

In `crates/crush-cli/Cargo.toml`, in the existing `windows-sys` features list, add:
```toml
"Win32_UI_WindowsAndMessaging",
"Win32_Foundation",
```
(Keep existing features; just append these two.)

---

### Task A3 — Add `install.ps1` PowerShell install script

**File:** `scripts/install.ps1` (create new file)

```powershell
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
```

**File:** `scripts/install.sh` (create new file)

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="Chidi09/crush"
VERSION="${1:-latest}"

if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
fi

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
    x86_64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH" && exit 1 ;;
esac

ASSET="crush-${VERSION}-${OS}-${ARCH}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"

TEMP=$(mktemp)
echo "Downloading Crush ${VERSION}..."
curl -fsSL "$URL" -o "$TEMP"
chmod +x "$TEMP"

echo "Installing..."
"$TEMP" install

rm -f "$TEMP"
echo "Done. Run: crush --version"
```

---

### Task A4 — Update `crush update` to call `install` after download

**File:** `crates/crush-cli/src/main.rs`

Find the `Commands::Update` handler. After the binary is downloaded and the
atomic rename is complete, add a call:

```rust
// After successful download and replace, re-run install to update PATH entry
// (idempotent — install_windows/install_unix handles existing PATH correctly)
if let Err(e) = cmd_install().await {
    eprintln!("Warning: self-update succeeded but PATH install failed: {}", e);
}
```

This means `crush update` downloads the new binary AND ensures the install
directory is up to date — both operations in one command.

---

## PART B — Detector Enhancements

### Task B1 — Fix Go framework detection (BROKEN bug)

**File:** `crates/crush-build/src/detect.rs`

**Problem:** `try_go` at line ~354 checks `root.join("gin").exists()` — this
looks for a *directory* named `gin` in the project root. Go dependencies live in
the module cache, never the project root. The correct check is to search
`go.mod` for the import path string.

**Replace the `framework` assignment in `try_go`:**

```rust
// OLD (buggy):
let framework = if root.join("gin").exists() || Self::file_contains(root.join("go.mod"), "gin").unwrap_or(false) {
    "Gin"
} else if Self::file_contains(root.join("go.mod"), "echo").unwrap_or(false) {
    "Echo"
} else if Self::file_contains(root.join("go.mod"), "fiber").unwrap_or(false) {
    "Fiber"
} else if Self::file_contains(root.join("go.mod"), "chi").unwrap_or(false) {
    "Chi"
} else { "" };

// NEW (correct):
let go_mod_content = fs::read_to_string(root.join("go.mod")).unwrap_or_default();
let framework = if go_mod_content.contains("github.com/gin-gonic/gin") {
    "Gin"
} else if go_mod_content.contains("github.com/labstack/echo") {
    "Echo"
} else if go_mod_content.contains("github.com/gofiber/fiber") {
    "Fiber"
} else if go_mod_content.contains("github.com/go-chi/chi") {
    "Chi"
} else if go_mod_content.contains("google.golang.org/grpc") {
    "gRPC"
} else { "" };
```

Also fix the `port` for Go: `echo` and `chi` typically use 8080, `gin` uses
8080 — currently port is hardcoded to `8080` regardless, which is fine. Leave
port as-is.

---

### Task B2 — Fix Node.js build command: detect pnpm and yarn

**File:** `crates/crush-build/src/detect.rs`

**Problem:** `infer_node_build` always outputs `npm run build` or `npm install`
even when the project uses `pnpm` (`pnpm-lock.yaml`) or `yarn` (`yarn.lock`).

**Replace `infer_node_build`:**

```rust
fn infer_node_build(&self, json: &serde_json::Value, root: &Path, has_ts: bool, has_deno: bool) -> String {
    if has_deno {
        return "deno task build".to_string();
    }
    if root.join("bun.lockb").exists() {
        return "bun run build".to_string();
    }

    // Detect package manager from lockfile
    let pm = if root.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    };

    let scripts = json["scripts"].as_object();
    if scripts.map(|s| s.contains_key("build")).unwrap_or(false) {
        return format!("{} run build", pm);
    }
    if scripts.map(|s| s.contains_key("start")).unwrap_or(false)
        && !scripts.map(|s| s.contains_key("build")).unwrap_or(false)
    {
        return format!("{} start", pm);
    }
    if root.join("vite.config.ts").exists() || root.join("vite.config.js").exists() {
        return format!("{} run build", pm);
    }
    if has_ts && root.join("tsconfig.json").exists() {
        return format!("{} run build", pm);
    }
    format!("{} install", pm)
}
```

Also update `infer_node_entry` to use `pnpm`/`yarn` for the start command by
passing the same `pm` string, or just leave entry detection as-is (entry point
is a file path, not a run command).

---

### Task B3 — Add SvelteKit and Astro framework detection

**File:** `crates/crush-build/src/detect.rs`

**In `detect_node_framework`**, add these cases BEFORE the existing `if` chain
(they're more specific and should take priority over generic "Express" etc.):

```rust
fn detect_node_framework(&self, json: &serde_json::Value, root: &Path) -> (String, f32) {
    let deps = Self::merge_deps(json);
    let has_file = |name: &str| root.join(name).exists();

    // More specific checks first
    if has_file("svelte.config.js") || has_file("svelte.config.ts")
        || deps.iter().any(|d| d == "@sveltejs/kit") {
        return ("SvelteKit".to_string(), 0.05);
    }
    if has_file("astro.config.mjs") || has_file("astro.config.ts") || has_file("astro.config.js")
        || deps.iter().any(|d| d == "astro") {
        return ("Astro".to_string(), 0.05);
    }
    if deps.iter().any(|d| d == "solid-js") && deps.iter().any(|d| d == "@solidjs/start") {
        return ("SolidStart".to_string(), 0.05);
    }
    if has_file("qwik.config.ts") || deps.iter().any(|d| d == "@builder.io/qwik") {
        return ("Qwik".to_string(), 0.04);
    }
    // Existing checks follow unchanged:
    if deps.iter().any(|d| d == "next") || has_file("next.config.js") || has_file("next.config.ts") {
        return ("Next.js".to_string(), 0.05);
    }
    // ... (keep rest of existing chain)
```

Also add the port mappings in `detect_port_framework`:
```rust
"SvelteKit" => 5173,
"Astro"     => 4321,
"SolidStart"=> 3000,
"Qwik"      => 5173,
```

---

### Task B4 — Improve PHP detection: detect Laravel

**File:** `crates/crush-build/src/detect.rs`

**Replace `try_php`:**

```rust
fn try_php(&self, root: &Path) -> Option<Detection> {
    if !root.join("composer.json").exists() { return None; }

    let is_laravel = root.join("artisan").exists()
        || Self::file_contains(root.join("composer.json"), "laravel/framework").unwrap_or(false);
    let is_symfony = Self::file_contains(root.join("composer.json"), "symfony/framework-bundle").unwrap_or(false);

    let (framework, entry, port, confidence) = if is_laravel {
        ("Laravel", "public/index.php", 8000, 0.92f32)
    } else if is_symfony {
        ("Symfony", "public/index.php", 8000, 0.90)
    } else {
        ("", "public/index.php", 8080, 0.85)
    };

    Some(Detection {
        project_name: root.file_name().unwrap_or_default().to_string_lossy().to_string(),
        runtime_type: RuntimeType::Php,
        runtime_version: VersionResolver::resolve(root, None),
        framework_name: framework.to_string(),
        framework_detected: !framework.is_empty(),
        build_command: "composer install --no-dev --optimize-autoloader".to_string(),
        entry_point: entry.to_string(),
        port,
        confidence,
        ..Default::default()
    })
}
```

---

### Task B5 — Improve Python detection: add uv, pdm, conda

**File:** `crates/crush-build/src/detect.rs`

**In `try_python`**, add detection for `uv.lock`, `pdm.lock`, and `environment.yml`:

Add near the top of `try_python`:
```rust
let has_uv = root.join("uv.lock").exists();
let has_pdm = root.join("pdm.lock").exists();
let has_conda = root.join("environment.yml").exists() || root.join("environment.yaml").exists();
```

Replace the `build_cmd` assignment:
```rust
let build_cmd = if has_uv {
    "uv sync --no-dev".to_string()
} else if has_pdm {
    "pdm install --prod".to_string()
} else if has_poetry {
    "poetry install --no-dev".to_string()
} else if has_requirements {
    "pip install -r requirements.txt".to_string()
} else if has_pyproject {
    "pip install -e .".to_string()
} else if has_conda {
    "conda env create -f environment.yml".to_string()
} else {
    "pip install -r requirements.txt".to_string()
};
```

Update the `if !has_pyproject && !has_requirements && !has_setup { return None; }` guard to also allow conda:
```rust
let has_conda = root.join("environment.yml").exists() || root.join("environment.yaml").exists();
if !has_pyproject && !has_requirements && !has_setup && !has_conda { return None; }
```

---

### Task B6 — Add base image mapping: use versioned slim/alpine images

**File:** `crates/crush-build/src/detect.rs`

After `best` is selected in `detect()` (around line 86 after the env scan), add a call to set the correct base image:

```rust
best.base_image = Self::resolve_base_image(&best);
```

Add `base_image: String` to the `Detection` struct (and its `Default` impl, defaulting to `"ubuntu:22.04".to_string()`).

Add the `resolve_base_image` method:

```rust
fn resolve_base_image(d: &Detection) -> String {
    let ver = d.runtime_version.trim();
    // Extract major version only (e.g. "20.11.0" → "20", "3.11.2" → "3.11")
    let major = ver.split('.').next().unwrap_or(ver);
    let major_minor = {
        let parts: Vec<&str> = ver.splitn(3, '.').collect();
        if parts.len() >= 2 { format!("{}.{}", parts[0], parts[1]) } else { major.to_string() }
    };

    match d.runtime_type {
        RuntimeType::Node | RuntimeType::TypeScript => {
            format!("node:{}-alpine", major)
        }
        RuntimeType::Bun => {
            format!("oven/bun:{}", major)
        }
        RuntimeType::Deno => {
            format!("denoland/deno:{}", ver)
        }
        RuntimeType::Python => {
            format!("python:{}-slim", major_minor)
        }
        RuntimeType::Rust => {
            "rust:alpine".to_string()  // Rust builds produce a static binary; use scratch or alpine
        }
        RuntimeType::Go => {
            format!("golang:{}-alpine", major_minor)
        }
        RuntimeType::Java => {
            format!("eclipse-temurin:{}-jre-alpine", major)
        }
        RuntimeType::DotNet => {
            format!("mcr.microsoft.com/dotnet/aspnet:{}", major_minor)
        }
        RuntimeType::Ruby => {
            format!("ruby:{}-alpine", major_minor)
        }
        RuntimeType::Php => {
            format!("php:{}-fpm-alpine", major_minor)
        }
        RuntimeType::Elixir => {
            format!("elixir:{}-alpine", major_minor)
        }
        RuntimeType::Swift => {
            format!("swift:{}-slim", major_minor)
        }
        RuntimeType::Generic => {
            "ubuntu:22.04".to_string()
        }
    }
}
```

**File:** `crates/crush-build/src/pipeline.rs`

In `resolve_base`, instead of hardcoding `"ubuntu:22.04"`, use the base image
from the Detection if available. The pipeline currently uses
`stage.image.as_deref().unwrap_or("ubuntu:22.04")` — this remains correct
because the Crushfile stage already carries the `image` field. When no Crushfile
exists and stages are synthesised from Detection (Task 15 from v2 plan), the CLI
sets `stage.image = Some(detection.base_image.clone())`. If that's not done yet,
add it in the CLI's `Commands::Default` / `Commands::Build` stage synthesis block.

---

### Task B7 — Add `base_image` to `Detection` output in `crush detect`

**File:** `crates/crush-cli/src/main.rs`

Find the `Commands::Detect` handler. It currently prints detection results.
Add the base image to the output:

```rust
println!("  Base image:  {}", detection.base_image);
```

Place it after the existing runtime/version line.

---

### Task B8 — Improve confidence display and add `crush detect --json` flag

**File:** `crates/crush-cli/src/main.rs`

Find `DetectArgs` struct. Add:
```rust
/// Output raw JSON instead of formatted table
#[arg(long)]
json: bool,
```

In the `Commands::Detect` handler, after building `detection`:
```rust
if args.json {
    println!("{}", serde_json::to_string_pretty(&detection)?);
    return Ok(());
}
```

---

## PART C — Inspector Enhancement

### Task C1 — Add `--format` flag to `InspectArgs`

**File:** `crates/crush-cli/src/main.rs`

Find `InspectArgs`. Add:
```rust
/// Output format: "pretty" (default) or "json"
#[arg(long, default_value = "pretty")]
format: String,
```

---

### Task C2 — Implement formatted `crush inspect` output for containers

**File:** `crates/crush-cli/src/main.rs`

The current `Commands::Inspect` for containers does `serde_json::to_string_pretty`
and prints raw JSON. Replace the container branch with formatted output:

```rust
// In the Commands::Inspect handler, container branch:
let json_path = data_dir.join("containers").join(&args.id).join("container.json");
if !json_path.exists() {
    eprintln!("Error: container '{}' not found", args.id);
    std::process::exit(1);
}
let content = tokio::fs::read_to_string(&json_path).await?;
let container: crush_types::Container = serde_json::from_str(&content)?;

if args.format == "json" {
    println!("{}", serde_json::to_string_pretty(&container)?);
    return Ok(());
}

// Formatted output
let status_str = match container.status {
    crush_types::ContainerStatus::Running => "running",
    crush_types::ContainerStatus::Stopped => "exited",
    crush_types::ContainerStatus::Paused  => "paused",
    crush_types::ContainerStatus::Created => "created",
    crush_types::ContainerStatus::Creating => "creating",
};

let created_ts = container.created_at
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| {
        let secs = d.as_secs();
        format!("{}", chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S UTC"))
    })
    .unwrap_or_else(|_| "unknown".to_string());

println!("Container: {}", container.id);
println!("  Name:     {}", container.name);
println!("  Status:   {}", status_str);
println!("  Image:    {}", container.image);
println!("  Created:  {}", created_ts);
if let Some(pid) = container.pid {
    println!("  PID:      {}", pid);
}

if !container.ports.is_empty() {
    println!("  Ports:");
    for p in &container.ports {
        println!("    {}:{} -> :{}/{}",
            if p.host_ip.is_empty() || p.host_ip == "0.0.0.0" { "*".to_string() } else { p.host_ip.clone() },
            p.host_port,
            p.container_port,
            match p.protocol { crush_types::Protocol::Tcp => "tcp", crush_types::Protocol::Udp => "udp" }
        );
    }
}

if !container.mounts.is_empty() {
    println!("  Mounts:");
    for m in &container.mounts {
        let mode = if m.read_only { "ro" } else { "rw" };
        let kind = if m.is_tmpfs { "tmpfs" } else { "bind" };
        println!("    {} -> {} ({}, {})", m.host_path.display(), m.container_path.display(), kind, mode);
    }
}

if container.memory_limit_bytes.is_some() || container.cpu_shares.is_some() {
    println!("  Resources:");
    if let Some(mem) = container.memory_limit_bytes {
        println!("    Memory limit: {}", format_bytes(mem));
    }
    if let Some(cpu) = container.cpu_shares {
        println!("    CPU shares:   {}", cpu);
    }
    if let Some(pids) = container.pids_limit {
        println!("    PIDs limit:   {}", pids);
    }
}

if let Some(health) = &container.health {
    let h_str = match health {
        crush_types::HealthStatus::Healthy   => "healthy",
        crush_types::HealthStatus::Unhealthy => "unhealthy",
        crush_types::HealthStatus::Starting  => "starting",
    };
    println!("  Health:   {}", h_str);
    if let Some(cmd) = &container.health_cmd {
        println!("    Check: {}", cmd);
    }
}

if let Some(policy) = &container.restart_policy {
    println!("  Restart:  {} (count: {})", policy, container.restart_count.unwrap_or(0));
}
```

Add the `format_bytes` helper function near the other formatting helpers:
```rust
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.1} GB", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1} MB", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.1} KB", bytes as f64 / KB as f64) }
    else { format!("{} B", bytes) }
}
```

---

### Task C3 — Formatted `crush inspect` for images

**File:** `crates/crush-cli/src/main.rs`

In the image branch of `Commands::Inspect`:

```rust
if args.format == "json" {
    println!("{}", serde_json::to_string_pretty(&image)?);
    return Ok(());
}
println!("Image: {}", image.tag);
println!("  ID:           {}", &image.id[..16.min(image.id.len())]);
println!("  Digest:       {}", image.digest);
println!("  Architecture: {}/{}", image.os, image.architecture);
println!("  Size:         {}", format_bytes(image.size_bytes));
println!("  Layers:       {}", image.layers.len());
if !image.entrypoint.is_empty() {
    println!("  Entrypoint:   {}", image.entrypoint.join(" "));
}
if !image.cmd.is_empty() {
    println!("  Cmd:          {}", image.cmd.join(" "));
}
if !image.env.is_empty() {
    println!("  Env:");
    for e in &image.env {
        println!("    {}", e);
    }
}
```

---

## PART D — Version bump and release

### Task D1 — Bump version to 0.3.0

**File:** `Cargo.toml` (workspace root)

Change:
```toml
version = "0.2.0"
```
to:
```toml
version = "0.3.0"
```

---

### Task D2 — Fix any compiler warnings from v0.2.0

**File:** `crates/crush-cli/src/main.rs`

The v0.2.0 build emitted these warnings:
1. `unused variable: tag` at line ~3031 — prefix with `_`: `let _tag = ...`
2. `function which_docker is never used` at line ~2958 — prefix with `_` or delete
   the function if it's not called anywhere.
3. `struct StatelessEngine is never constructed` in `crates/crush-cli/src/runtime.rs`
   — delete the file or add `#[allow(dead_code)]` at the top.

---

### Task D3 — Build, tag, and release

On the Linux VPS (safe-meet), run:

```sh
# Verify Linux build
cd /root/crush
git pull origin main
/root/.cargo/bin/cargo build --release -p crush-cli

# Cross-compile for Windows
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
  /root/.cargo/bin/cargo build --release --target x86_64-pc-windows-gnu -p crush-cli

# Strip and package
x86_64-w64-mingw32-strip target/x86_64-pc-windows-gnu/release/crush-cli.exe
cp target/x86_64-pc-windows-gnu/release/crush-cli.exe crush-0.3.0-windows-x86_64.exe

# Tag and publish
git tag v0.3.0
git push origin v0.3.0
gh release create v0.3.0 crush-0.3.0-windows-x86_64.exe \
  --title "Crush v0.3.0" \
  --notes "v0.3.0 — PATH self-install, detector improvements, readable inspect output

## What's new
- crush install: copies binary to LOCALAPPDATA\\crush\\bin and updates user PATH registry key (Windows), or ~/.local/bin (Linux). No admin required.
- install.ps1 / install.sh: one-liner scripts for fresh installs
- crush update now also updates the PATH install location
- Fix Go framework detection (was checking wrong path type — now scans go.mod)
- Fix Node build commands for pnpm and yarn projects (was always using npm)
- Add SvelteKit, Astro, SolidStart, Qwik framework detection
- Add Laravel and Symfony detection for PHP projects
- Add uv, pdm, conda support for Python projects
- Base image now uses versioned slim/alpine tags (node:20-alpine, python:3.11-slim, etc.)
- crush detect --json flag for machine-readable output
- crush inspect now shows formatted, human-readable output by default
- crush inspect --format json for raw JSON (previous behaviour)
- Version bumped to 0.3.0"
```

---

## Priority order

If time is limited, do these first:

1. **A1 + A2** (install command) — highest user impact
2. **B1** (Go framework bug fix) — correctness
3. **B2** (pnpm/yarn build commands) — high hit rate for real projects
4. **C1 + C2** (inspect formatting) — UX
5. **B3** (SvelteKit/Astro) — modern framework coverage
6. **B6** (base image mapping) — correctness for builds
7. **A3** (install scripts) — convenience
8. **B4, B5, B7, B8, C3, D2** — improvements
