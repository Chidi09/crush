use std::process::Command;
use crush_types::{Result, CrushError};

#[derive(Debug, Default, Clone)]
pub struct ContainerMetrics {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
}

pub enum EbpfAvailability {
    Available,
    NoBtf,
    Unsupported,
}

#[cfg(feature = "ebpf")]
pub struct EbpfManager {
    bpf: aya::Ebpf,
}

#[cfg(not(feature = "ebpf"))]
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

    pub fn check_availability() -> EbpfAvailability {
        if !Self::kernel_supports_ebpf() {
            return EbpfAvailability::Unsupported;
        }
        if !Self::has_btf() {
            return EbpfAvailability::NoBtf;
        }
        EbpfAvailability::Available
    }

    /// Load the compiled eBPF ELF from `/usr/local/lib/crush/crush-ebpf` and
    /// attach the `xdp_router` XDP program and `tc_egress` TC egress program
    /// to `iface`.
    ///
    /// Returns `Err(CrushError::NetworkError)` on non-Linux platforms.
    pub fn load_and_attach(iface: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use aya::{
                programs::{Xdp, XdpFlags, SchedClassifier, TcAttachType},
            };
            use std::process::Command as StdCommand;

            let elf_path = "/usr/local/lib/crush/crush-ebpf";
            let mut bpf = aya::Ebpf::load_file(elf_path)
                .map_err(|e| CrushError::NetworkError(format!("eBPF load_file {}: {}", elf_path, e)))?;

            // --- XDP: xdp_router ---
            let xdp_prog: &mut Xdp = bpf
                .program_mut("xdp_router")
                .ok_or_else(|| CrushError::NetworkError("xdp_router not found in ELF".into()))?
                .try_into()
                .map_err(|e| CrushError::NetworkError(format!("xdp_router cast: {}", e)))?;

            xdp_prog
                .load()
                .map_err(|e| CrushError::NetworkError(format!("xdp_router load: {}", e)))?;
            xdp_prog
                .attach(iface, XdpFlags::default())
                .map_err(|e| CrushError::NetworkError(format!("xdp_router attach {}: {}", iface, e)))?;

            // --- TC Egress: tc_egress ---
            // Ensure the clsact qdisc exists on the interface
            let tc_out = StdCommand::new("tc")
                .args(["qdisc", "add", "dev", iface, "clsact"])
                .output()
                .map_err(|e| CrushError::NetworkError(format!("tc qdisc add: {}", e)))?;
            if !tc_out.status.success() {
                let stderr = String::from_utf8_lossy(&tc_out.stderr);
                if !stderr.contains("File exists") {
                    return Err(CrushError::NetworkError(
                        format!("tc clsact on {}: {}", iface, stderr)
                    ));
                }
            }

            let tc_prog: &mut SchedClassifier = bpf
                .program_mut("tc_egress")
                .ok_or_else(|| CrushError::NetworkError("tc_egress not found in ELF".into()))?
                .try_into()
                .map_err(|e| CrushError::NetworkError(format!("tc_egress cast: {}", e)))?;

            tc_prog
                .load()
                .map_err(|e| CrushError::NetworkError(format!("tc_egress load: {}", e)))?;
            tc_prog
                .attach(iface, TcAttachType::Egress)
                .map_err(|e| CrushError::NetworkError(format!("tc_egress attach {}: {}", iface, e)))?;

            // `bpf` must stay alive — in production this should be stored; for now
            // we intentionally leak it so the programs remain attached for the
            // lifetime of the process.
            std::mem::forget(bpf);

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = iface;
            Err(CrushError::NetworkError(
                "load_and_attach is only supported on Linux".into(),
            ))
        }
    }
}

#[cfg(feature = "ebpf")]
impl EbpfManager {
    pub fn new() -> Result<Self> {
        let elf_bytes: &[u8] = include_bytes!(env!("EBPF_PROG_PATH"));
        let bpf = aya::Ebpf::load(elf_bytes)
            .map_err(|e| CrushError::NetworkError(format!("eBPF load: {}", e)))?;
        Ok(Self { bpf })
    }

    pub fn attach_xdp(&mut self, iface: &str) -> Result<()> {
        use aya::programs::{Xdp, XdpFlags};

        let prog: &mut Xdp = self.bpf
            .program_mut("xdp_router")
            .ok_or_else(|| CrushError::NetworkError("XDP program not found in ELF".into()))?
            .try_into()
            .map_err(|e| CrushError::NetworkError(format!("XDP cast: {}", e)))?;

        prog.load()
            .map_err(|e| CrushError::NetworkError(format!("XDP load: {}", e)))?;
        prog.attach(iface, XdpFlags::default())
            .map_err(|e| CrushError::NetworkError(format!("XDP attach {}: {}", iface, e)))?;
        Ok(())
    }

    pub fn attach_tc_egress(&mut self, iface: &str) -> Result<()> {
        use aya::programs::{SchedClassifier, TcAttachType};

        // Ensure clsact qdisc exists before attaching
        let out = Command::new("tc")
            .args(["qdisc", "add", "dev", iface, "clsact"])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("tc qdisc: {}", e)))?;
        if !out.status.success() {
            let s = String::from_utf8_lossy(&out.stderr);
            if !s.contains("File exists") {
                return Err(CrushError::NetworkError(format!("tc clsact on {}: {}", iface, s)));
            }
        }

        let prog: &mut SchedClassifier = self.bpf
            .program_mut("tc_egress")
            .ok_or_else(|| CrushError::NetworkError("TC program not found in ELF".into()))?
            .try_into()
            .map_err(|e| CrushError::NetworkError(format!("TC cast: {}", e)))?;

        prog.load()
            .map_err(|e| CrushError::NetworkError(format!("TC load: {}", e)))?;
        prog.attach(iface, TcAttachType::Egress)
            .map_err(|e| CrushError::NetworkError(format!("TC attach {}: {}", iface, e)))?;
        Ok(())
    }

    pub fn detach_tc_egress(&self, iface: &str) -> Result<()> {
        let out = Command::new("tc")
            .args(["qdisc", "del", "dev", iface, "clsact"])
            .output()
            .map_err(|e| CrushError::NetworkError(format!("tc qdisc del: {}", e)))?;
        if !out.status.success() {
            let s = String::from_utf8_lossy(&out.stderr);
            if !s.contains("No such file") && !s.contains("Cannot find") {
                return Err(CrushError::NetworkError(format!("tc del on {}: {}", iface, s)));
            }
        }
        Ok(())
    }

    pub fn add_container_ip(&mut self, ip_be: u32, ifindex: u32) -> Result<()> {
        use aya::maps::HashMap;

        let mut map: HashMap<_, u32, u32> = HashMap::try_from(
            self.bpf.map_mut("CONTAINER_IPS")
                .ok_or_else(|| CrushError::NetworkError("CONTAINER_IPS map not found".into()))?,
        )
        .map_err(|e| CrushError::NetworkError(format!("CONTAINER_IPS cast: {}", e)))?;

        map.insert(ip_be, ifindex, 0)
            .map_err(|e| CrushError::NetworkError(format!("CONTAINER_IPS insert: {}", e)))?;
        Ok(())
    }

    pub fn remove_container_ip(&mut self, ip_be: u32) -> Result<()> {
        use aya::maps::HashMap;

        let mut map: HashMap<_, u32, u32> = HashMap::try_from(
            self.bpf.map_mut("CONTAINER_IPS")
                .ok_or_else(|| CrushError::NetworkError("CONTAINER_IPS map not found".into()))?,
        )
        .map_err(|e| CrushError::NetworkError(format!("CONTAINER_IPS cast: {}", e)))?;

        map.remove(&ip_be)
            .map_err(|e| CrushError::NetworkError(format!("CONTAINER_IPS remove: {}", e)))?;
        Ok(())
    }

    /// `value` encodes `container_ip_be << 32 | container_port` so the TC
    /// program can rewrite both the destination IP and port in one lookup.
    pub fn add_port_mapping(&mut self, host_port: u16, container_ip_be: u32, container_port: u16) -> Result<()> {
        use aya::maps::HashMap;

        let value: u64 = ((container_ip_be as u64) << 32) | (container_port as u64);

        let mut map: HashMap<_, u32, u64> = HashMap::try_from(
            self.bpf.map_mut("PORT_MAPPINGS")
                .ok_or_else(|| CrushError::NetworkError("PORT_MAPPINGS map not found".into()))?,
        )
        .map_err(|e| CrushError::NetworkError(format!("PORT_MAPPINGS cast: {}", e)))?;

        map.insert(host_port as u32, value, 0)
            .map_err(|e| CrushError::NetworkError(format!("PORT_MAPPINGS insert: {}", e)))?;
        Ok(())
    }

    pub fn remove_port_mapping(&mut self, host_port: u16) -> Result<()> {
        use aya::maps::HashMap;

        let mut map: HashMap<_, u32, u64> = HashMap::try_from(
            self.bpf.map_mut("PORT_MAPPINGS")
                .ok_or_else(|| CrushError::NetworkError("PORT_MAPPINGS map not found".into()))?,
        )
        .map_err(|e| CrushError::NetworkError(format!("PORT_MAPPINGS cast: {}", e)))?;

        map.remove(&(host_port as u32))
            .map_err(|e| CrushError::NetworkError(format!("PORT_MAPPINGS remove: {}", e)))?;
        Ok(())
    }

    pub fn read_container_metrics(&self, cgroup_id: u64) -> ContainerMetrics {
        use aya::maps::HashMap;
        let mut m = ContainerMetrics::default();

        if let Some(map_ref) = self.bpf.map("NET_BYTES_MAP") {
            if let Ok(map) = HashMap::<_, u64, u64>::try_from(map_ref) {
                m.rx_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
            }
        }
        if let Some(map_ref) = self.bpf.map("NET_TX_MAP") {
            if let Ok(map) = HashMap::<_, u64, u64>::try_from(map_ref) {
                m.tx_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
            }
        }
        if let Some(map_ref) = self.bpf.map("BLOCK_READ_MAP") {
            if let Ok(map) = HashMap::<_, u64, u64>::try_from(map_ref) {
                m.block_read_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
            }
        }
        if let Some(map_ref) = self.bpf.map("BLOCK_WRITE_MAP") {
            if let Ok(map) = HashMap::<_, u64, u64>::try_from(map_ref) {
                m.block_write_bytes = map.get(&cgroup_id, 0).unwrap_or(0);
            }
        }
        m
    }

    pub fn attach_metrics(&mut self, container_id: &str) -> Result<()> {
        use aya::programs::{CgroupSkb, CgroupSkbAttachType};

        let cgroup_path = format!("/sys/fs/cgroup/crush/{}", container_id);

        let ingress: &mut CgroupSkb = self.bpf
            .program_mut("crush_net_ingress")
            .ok_or_else(|| CrushError::NetworkError("crush_net_ingress not found".into()))?
            .try_into()
            .map_err(|e| CrushError::NetworkError(format!("ingress cast: {}", e)))?;
        ingress.load()
            .map_err(|e| CrushError::NetworkError(format!("ingress load: {}", e)))?;
        ingress.attach(std::path::Path::new(&cgroup_path), CgroupSkbAttachType::Ingress)
            .map_err(|e| CrushError::NetworkError(format!("ingress attach {}: {}", cgroup_path, e)))?;

        let egress: &mut CgroupSkb = self.bpf
            .program_mut("crush_net_egress")
            .ok_or_else(|| CrushError::NetworkError("crush_net_egress not found".into()))?
            .try_into()
            .map_err(|e| CrushError::NetworkError(format!("egress cast: {}", e)))?;
        egress.load()
            .map_err(|e| CrushError::NetworkError(format!("egress load: {}", e)))?;
        egress.attach(std::path::Path::new(&cgroup_path), CgroupSkbAttachType::Egress)
            .map_err(|e| CrushError::NetworkError(format!("egress attach {}: {}", cgroup_path, e)))?;

        Ok(())
    }
}

#[cfg(not(feature = "ebpf"))]
impl EbpfManager {
    pub fn new() -> Result<Self> {
        Err(CrushError::NetworkError(
            "crush was compiled without the 'ebpf' feature — rebuild with --features ebpf".into(),
        ))
    }
}
