use std::path::Path;
use std::process::Command;
use crush_types::{Result, CrushError};

pub struct CrossCompiler;

impl CrossCompiler {
    pub fn build_image_key(platform: &str) -> String {
        format!("platform:{}", platform)
    }

    pub fn register_qemu() -> Result<()> {
        let out = Command::new("docker")
            .args(["run", "--privileged", "--rm", "tonistiigi/binfmt", "--install", "all"])
            .output();

        match out {
            Ok(o) if o.status.success() => Ok(()),
            _ => {
                let fallback = Command::new("qemu-binfmt")
                    .args(["-p", "--systemd", "ALL"])
                    .output();
                match fallback {
                    Ok(_) => Ok(()),
                    Err(e) => Err(CrushError::ImageError(format!(
                        "QEMU binfmt registration failed. Install qemu-user-static: {}", e
                    )))
                }
            }
        }
    }

    pub fn target_triple(platform: &str) -> String {
        match platform {
            "linux/amd64" => "x86_64-unknown-linux-gnu",
            "linux/arm64" => "aarch64-unknown-linux-gnu",
            "linux/arm/v7" => "armv7-unknown-linux-gnueabihf",
            "linux/arm/v6" => "arm-unknown-linux-gnueabihf",
            "linux/386" => "i686-unknown-linux-gnu",
            "linux/riscv64" => "riscv64gc-unknown-linux-gnu",
            "linux/s390x" => "s390x-unknown-linux-gnu",
            _ => "x86_64-unknown-linux-gnu",
        }
    }

    pub fn rust_target(platform: &str) -> String {
        match platform {
            "linux/amd64" => "x86_64-unknown-linux-gnu",
            "linux/arm64" => "aarch64-unknown-linux-gnu",
            "linux/arm/v7" => "armv7-unknown-linux-gnueabihf",
            "linux/386" => "i686-unknown-linux-gnu",
            _ => Self::target_triple(platform),
        }
    }

    pub fn rust_cross_command(target: &str) -> String {
        let triple = Self::rust_target(target);
        format!("cargo build --release --target {}", triple)
    }

    pub fn is_native_platform(platform: &str) -> bool {
        let host_arch = std::env::consts::ARCH;
        match (platform, host_arch) {
            ("linux/amd64", "x86_64") => true,
            ("linux/arm64", "aarch64") => true,
            _ => false,
        }
    }

    pub fn base_image_for_platform(platform: &str) -> String {
        let arch = match platform {
            "linux/amd64" => "amd64",
            "linux/arm64" => "arm64",
            "linux/arm/v7" => "arm32v7",
            "linux/arm/v6" => "arm32v6",
            "linux/386" => "i386",
            "linux/riscv64" => "riscv64",
            _ => "amd64",
        };
        format!("{}ubuntu:22.04", if arch == "amd64" { "" } else { &format!("{}/", arch) })
    }

    pub fn parse_platforms(input: &[String]) -> Result<Vec<String>> {
        let mut platforms = Vec::new();
        for p in input {
            let p = p.trim();
            if !p.contains('/') {
                return Err(CrushError::ImageError(format!(
                    "Invalid platform '{}'. Expected format: os/arch (e.g. linux/amd64)", p
                )));
            }
            platforms.push(p.to_string());
        }
        if platforms.is_empty() {
            platforms.push("linux/amd64".to_string());
        }
        Ok(platforms)
    }
}
