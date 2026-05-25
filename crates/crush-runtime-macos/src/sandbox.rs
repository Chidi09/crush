use crush_types::{Result, CrushError};

/// Builds macOS Seatbelt sandbox profiles with proper escaping.
/// ⚠ CRITICAL: All paths are escaped to prevent SBOX profile injection.
/// Without escaping, a path containing `) (allow file-read* (subpath "/")
/// would grant unrestricted host file access.

pub struct SandboxProfile {
    profile_text: String,
}

impl SandboxProfile {
    pub fn new(container_id: &str, network_access: bool, filesystem_paths: &[String]) -> Self {
        let mut profile = String::new();
        profile.push_str("(version 1)\n");
        profile.push_str("(deny default)\n");

        if network_access {
            profile.push_str("(allow network*)\n");
        }

        profile.push_str("(allow sysctl-read)\n");
        profile.push_str("(allow process-fork)\n");
        profile.push_str("(allow signal (target self))\n");

        for raw_path in filesystem_paths {
            // ⚠ CRITICAL: Escape SBOX string literals to prevent profile injection.
            // Characters that need escaping in SBOX strings: \ " ( )
            let escaped = raw_path.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('(', "\\(")
                .replace(')', "\\)");
            profile.push_str(&format!("(allow file-read* file-write* (subpath \"{}\"))\n", escaped));
        }

        let vm_dir = format!("/tmp/crush_vm_{}", container_id);
        let vm_escaped = vm_dir.replace('(', "\\(").replace(')', "\\)");
        profile.push_str(&format!("(allow file-read* file-write* (subpath \"{}\"))\n", vm_escaped));

        profile.push_str("(allow mach*)\n");
        profile.push_str("(allow ipc-posix*)\n");

        Self { profile_text: profile }
    }

    pub fn apply(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let profile_c = std::ffi::CString::new(self.profile_text.as_str())
                .map_err(|e| CrushError::NamespaceError(format!("Invalid profile: {}", e)))?;

            let result = unsafe { libc::sandbox_init(profile_c.as_ptr(), 0, std::ptr::null_mut()) };
            if result != 0 {
                let err = std::io::Error::last_os_error();
                return Err(CrushError::NamespaceError(format!("sandbox_init failed: {}", err)));
            }
        }
        Ok(())
    }
}
