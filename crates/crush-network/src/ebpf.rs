use std::process::Command;
use crush_types::{Result, CrushError};

pub struct EbpfManager;

impl EbpfManager {
    pub fn kernel_supports_ebpf() -> bool {
        let uname = Command::new("uname")
            .args(["-r"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let parts: Vec<&str> = uname.trim().split('.').collect();
        if parts.len() >= 2 {
            let major: u32 = parts[0].parse().unwrap_or(0);
            let minor: u32 = parts[1].parse().unwrap_or(0);
            major > 5 || (major == 5 && minor >= 4)
        } else {
            false
        }
    }

    pub fn has_btf() -> bool {
        std::path::Path::new("/sys/kernel/btf/vmlinux").exists()
    }

    pub fn load_tc_program(bridge_name: &str) -> Result<()> {
        if !Self::kernel_supports_ebpf() {
            return Err(CrushError::NetworkError("eBPF requires kernel 5.4+".to_string()));
        }

        let attach = Command::new("tc")
            .args(["qdisc", "add", "dev", bridge_name, "clsact"])
            .output();

        if let Ok(ref out) = attach {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stderr.contains("File exists") {
                    eprintln!("Warning: tc clsact attach: {}", stderr);
                }
            }
        }

        Ok(())
    }

    pub fn unload_tc_program(bridge_name: &str) -> Result<()> {
        let _ = Command::new("tc")
            .args(["qdisc", "del", "dev", bridge_name, "clsact"])
            .output();
        Ok(())
    }

    pub fn attach_xdp_program(iface: &str) -> Result<()> {
        if !Self::kernel_supports_ebpf() {
            return Err(CrushError::NetworkError("XDP requires kernel 5.4+".to_string()));
        }

        let _ = Command::new("ip")
            .args(["link", "set", "dev", iface, "xdp", "off"])
            .output();

        Ok(())
    }

    pub fn check_availability() -> EbpfAvailability {
        if !Self::kernel_supports_ebpf() {
            return EbpfAvailability::Unsupported;
        }
        if !Self::has_btf() {
            return EbpfAvailability::NoBtf;
        }
        EbpfAvailability::Available
    }
}

pub enum EbpfAvailability {
    Available,
    NoBtf,
    Unsupported,
}
