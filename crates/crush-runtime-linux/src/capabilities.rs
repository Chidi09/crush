use crush_types::{Result, CrushError};
use caps::{CapSet, Capability};

#[derive(Debug, Clone)]
pub struct CapabilitiesManager {
    effective_set: Vec<String>,
}

impl CapabilitiesManager {
    pub fn new() -> Self {
        let defaults = vec![
            "CAP_CHOWN".to_string(),
            "CAP_DAC_OVERRIDE".to_string(),
            "CAP_FOWNER".to_string(),
            "CAP_FSETID".to_string(),
            "CAP_KILL".to_string(),
            "CAP_SETGID".to_string(),
            "CAP_SETUID".to_string(),
            "CAP_SETPCAP".to_string(),
            "CAP_NET_BIND_SERVICE".to_string(),
            "CAP_SYS_CHROOT".to_string(),
            "CAP_MKNOD".to_string(),
            "CAP_AUDIT_WRITE".to_string(),
            "CAP_SETFCAP".to_string(),
        ];

        Self { effective_set: defaults }
    }

    pub fn drop_unnecessary_capabilities(&self) -> Result<()> {
        let caps_to_drop = vec![
            Capability::CAP_NET_RAW,
            Capability::CAP_SYS_ADMIN,
            Capability::CAP_SYS_RAWIO,
            Capability::CAP_SYS_PTRACE,
            Capability::CAP_SYS_MODULE,
            Capability::CAP_SYS_BOOT,
            Capability::CAP_SYS_TIME,
            Capability::CAP_SYS_TTY_CONFIG,
            Capability::CAP_SYSLOG,
            Capability::CAP_NET_ADMIN,
            Capability::CAP_NET_CONTROL,
            Capability::CAP_MAC_ADMIN,
            Capability::CAP_MAC_OVERRIDE,
            Capability::CAP_LINUX_IMMUTABLE,
            Capability::CAP_IPC_LOCK,
            Capability::CAP_IPC_OWNER,
            Capability::CAP_LEASE,
            Capability::CAP_WAKE_ALARM,
            Capability::CAP_BLOCK_SUSPEND,
        ];

        for set in &[CapSet::Bounding, CapSet::Effective, CapSet::Inheritable, CapSet::Permitted] {
            for cap in &caps_to_drop {
                caps::drop(None, *set, *cap)
                    .map_err(|e| CrushError::NamespaceError(format!(
                        "Failed to drop {:?} from {:?}: {}", cap, set, e
                    )))?;
            }
        }

        Ok(())
    }

    pub fn apply_ambient_capabilities(&self) -> Result<()> {
        if let Ok(permitted) = caps::read(None, CapSet::Permitted) {
            for cap in &permitted {
                let cap_str = format!("{:?}", cap);
                if self.effective_set.contains(&cap_str) {
                    caps::set(None, CapSet::Ambient, caps::CapState {
                        effective: true, permitted: true, inheritable: false,
                    }, *cap).ok();
                }
            }
        }
        Ok(())
    }

    pub fn get_effective_set(&self) -> &[String] { &self.effective_set }
}
