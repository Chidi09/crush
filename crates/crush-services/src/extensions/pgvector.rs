//! Build-on-first-use pgvector for the native PostgreSQL backend.
//!
//! When a project asks for `pgvector/pgvector:*` and we're routing it to
//! the host PG (instead of a container), we need the `vector.dll` and
//! `vector.control` to be present under that PG's install. Most users
//! don't have them. So: download the pgvector source, compile against
//! the host PG headers with MSVC, install into PG's `lib/` and
//! `share/extension/`.
//!
//! The install step writes to `C:\Program Files\PostgreSQL\<v>\` which
//! requires admin — we detect that and re-launch ourselves elevated for
//! just the install step.

use std::path::{Path, PathBuf};
use anyhow::{Result, Context, bail};

/// True if vector.control is present in PG's share/extension dir.
pub fn is_installed(pg_root: &Path) -> bool {
    pg_root.join("share").join("extension").join("vector.control").exists()
}

/// Locate vswhere.exe (ships with VS Installer). Returns None if VS isn't installed.
fn find_vswhere() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
        r"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe",
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() { return Some(p); }
    }
    None
}

/// Locate the latest VS installation that has the VC++ x64 build tools.
/// Returns the installation path (e.g. `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`).
fn find_msvc_install(vswhere: &Path) -> Option<PathBuf> {
    // `-products *` is required because BuildTools is a different product
    // SKU than the full IDE — without it vswhere ignores BuildTools.
    let output = std::process::Command::new(vswhere)
        .args([
            "-latest",
            "-all",
            "-products", "*",
            "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property", "installationPath",
        ])
        .output().ok()?;
    if !output.status.success() { return None; }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if path.is_empty() { None } else { Some(PathBuf::from(path)) }
}

/// Path to `vcvars64.bat` inside an MSVC install.
fn vcvars64(vs_root: &Path) -> PathBuf {
    vs_root.join("VC").join("Auxiliary").join("Build").join("vcvars64.bat")
}

#[cfg(target_os = "windows")]
fn is_running_elevated() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_ELEVATION};
    unsafe {
        let mut tok: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut tok) == 0 {
            return false;
        }
        let mut elev: TOKEN_ELEVATION = std::mem::zeroed();
        let mut sz: u32 = 0;
        let ok = GetTokenInformation(
            tok,
            windows_sys::Win32::Security::TokenElevation,
            &mut elev as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut sz,
        );
        CloseHandle(tok);
        ok != 0 && elev.TokenIsElevated != 0
    }
}

#[cfg(not(target_os = "windows"))]
fn is_running_elevated() -> bool { true }

/// Clone pgvector source at the pinned tag into `<cache>/pgvector-src/`,
/// idempotent: if already cloned at the right tag, returns immediately.
async fn ensure_source(cache_dir: &Path) -> Result<PathBuf> {
    const TAG: &str = "v0.8.0";
    let src = cache_dir.join("pgvector-src");
    if src.join(".git").exists() {
        // Verify it's at the pinned tag — if a previous run got interrupted
        // mid-clone we don't want to silently use a broken checkout.
        let head = std::process::Command::new("git")
            .args(["-C", src.to_str().unwrap_or(""), "describe", "--tags"])
            .output();
        if let Ok(out) = head {
            let described = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if described == TAG { return Ok(src); }
        }
        // mismatched / broken — re-clone fresh
        let _ = std::fs::remove_dir_all(&src);
    }
    std::fs::create_dir_all(cache_dir).context("create cache dir")?;
    let status = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth", "1",
            "--branch", TAG,
            "https://github.com/pgvector/pgvector",
            src.to_str().unwrap_or(""),
        ])
        .status()
        .await
        .context("git clone pgvector")?;
    if !status.success() {
        bail!("git clone of pgvector {} failed (is git on PATH?)", TAG);
    }
    Ok(src)
}

/// Build + install pgvector against the host PG. Idempotent.
///
/// Steps:
/// 1. vswhere → MSVC install path → vcvars64.bat
/// 2. git clone pgvector @ pinned tag into cache
/// 3. Run `vcvars64 && set PGROOT=... && nmake /F Makefile.win && nmake /F Makefile.win install`
///    via a temp `.cmd` file (vcvars64 has too many env mutations to pass through directly).
pub async fn ensure_installed(pg_root: &Path, cache_dir: &Path) -> Result<()> {
    if is_installed(pg_root) {
        return Ok(());
    }

    let pg_root_str = pg_root.to_str().context("pg_root utf8")?;

    let vswhere = find_vswhere().context(
        "MSVC build tools not found. Install with: winget install --id \
         Microsoft.VisualStudio.2022.BuildTools --override \
         \"--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools \
         --add Microsoft.VisualStudio.Component.Windows10SDK.19041\"",
    )?;
    let vs_root = find_msvc_install(&vswhere).context(
        "MSVC is installed but the VC++ x64 build tools workload is not. \
         Open 'Visual Studio Installer', Modify, check 'Desktop development with C++'.",
    )?;
    let vcvars = vcvars64(&vs_root);
    if !vcvars.exists() {
        bail!("vcvars64.bat not found at {} — broken MSVC install?", vcvars.display());
    }

    let src = ensure_source(cache_dir).await?;

    // The install step writes to PG's `lib/` and `share/extension/`. On most
    // Windows hosts PG is in `Program Files` which requires admin. Surface
    // a clear, actionable error rather than failing inside nmake.
    if !is_running_elevated() && pg_root_str.contains("Program Files") {
        bail!(
            "pgvector install needs admin to write into {}. \
             Re-run crush from an elevated terminal (Right-click → Run as administrator), \
             or move PostgreSQL to a non-protected directory.",
            pg_root.display()
        );
    }

    // Build a one-shot .cmd that sets up the MSVC env, points PGROOT at the
    // host PG, runs the build, then the install. vcvars64 sets ~50 env vars
    // — easier to call it once inside a child cmd.exe than to translate.
    let build_script = cache_dir.join("crush_pgvector_build.cmd");
    let script = format!(
        "@echo off\r\n\
         call \"{vcvars}\" || exit /b 1\r\n\
         set \"PGROOT={pg_root}\"\r\n\
         cd /d \"{src}\" || exit /b 1\r\n\
         nmake /F Makefile.win || exit /b 1\r\n\
         nmake /F Makefile.win install || exit /b 1\r\n",
        vcvars = vcvars.display(),
        pg_root = pg_root_str,
        src = src.display(),
    );
    std::fs::write(&build_script, script).context("write build script")?;

    let out = tokio::process::Command::new("cmd")
        .args(["/c", build_script.to_str().unwrap_or("")])
        .output()
        .await
        .context("run pgvector build script")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        bail!(
            "pgvector build failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
            stdout.trim(),
            stderr.trim(),
        );
    }

    if !is_installed(pg_root) {
        bail!(
            "pgvector build reported success but vector.control is missing under {}",
            pg_root.display()
        );
    }
    Ok(())
}
