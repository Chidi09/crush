use crate::check::{CheckAction, CheckStatus, DoctorCheck};
use async_trait::async_trait;

pub struct BuildToolsCheck;

#[async_trait]
impl DoctorCheck for BuildToolsCheck {
    fn name(&self) -> &'static str {
        "Native Build Tools"
    }

    fn description(&self) -> &'static str {
        "Checks for common native build tools like C/C++ compiler, make, etc."
    }

    async fn check(&self) -> anyhow::Result<CheckStatus> {
        // Mock checking for cc, make, python headers, pkg-config, libpq
        // Usually would shell out to `cc --version`, `make --version`

        #[cfg(target_os = "macos")]
        {
            Ok(CheckStatus::Warning(
                "Missing Xcode Command Line Tools".to_string(),
            ))
        }

        #[cfg(target_os = "linux")]
        {
            Ok(CheckStatus::Warning("Missing build-essential".to_string()))
        }

        #[cfg(target_os = "windows")]
        {
            Ok(CheckStatus::Warning("Missing MSVC Build Tools".to_string()))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Ok(CheckStatus::Pass)
        }
    }

    fn fix(&self) -> Option<CheckAction> {
        #[cfg(target_os = "macos")]
        {
            Some(CheckAction {
                label: "Install Xcode Command Line Tools".to_string(),
                command: "xcode-select".to_string(),
                args: vec!["--install".to_string()],
            })
        }

        #[cfg(target_os = "linux")]
        {
            Some(CheckAction {
                label: "Install build-essential".to_string(),
                command: "sudo".to_string(),
                args: vec![
                    "apt-get".to_string(),
                    "install".to_string(),
                    "-y".to_string(),
                    "build-essential".to_string(),
                ],
            })
        }

        #[cfg(target_os = "windows")]
        {
            Some(CheckAction {
                label: "Install MSVC Build Tools".to_string(),
                command: "winget".to_string(),
                args: vec![
                    "install".to_string(),
                    "--id".to_string(),
                    "Microsoft.VisualStudio.2022.BuildTools".to_string(),
                    "-e".to_string(),
                ],
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}
