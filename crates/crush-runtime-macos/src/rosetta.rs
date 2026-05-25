use crush_types::{Result, CrushError};

pub struct RosettaConfig {
    available: bool,
}

impl RosettaConfig {
    pub fn detect() -> Self {
        let available = Self::check_rosetta_availability();
        if available {
            println!("Rosetta 2: detected and available for x86_64 image translation");
        }
        Self { available }
    }

    fn check_rosetta_availability() -> bool {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("/usr/bin/pgrep")
                .args(["-q", "OAHKit"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
                ||
            std::process::Command::new("/usr/bin/softwareupdate")
                .args(["--list"])
                .output()
                .map(|o| {
                    let out = String::from_utf8_lossy(&o.stdout);
                    out.contains("Rosetta")
                })
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        self.available
    }

    #[cfg(target_os = "macos")]
    pub fn create_rosetta_share(&self) -> Option<objc2::rc::Retained<objc2_foundation::NSObject>> {
        if !self.available {
            return None;
        }

        use objc2::rc::Retained;
        use objc2_foundation::NSObject;
        use crate::bindings::*;

        Some(unsafe { VZLinuxRosettaDirectoryShare::init().upcast() })
    }
}
