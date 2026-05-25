use crush_types::{Result, CrushError};

pub struct VirtioNetworkConfig {
    vm_id: String,
    interface_name: String,
}

impl VirtioNetworkConfig {
    pub fn new(vm_id: &str) -> Result<Self> {
        let iface = format!("crush_vm_{}", &vm_id[..8.min(vm_id.len())]);
        Ok(Self {
            vm_id: vm_id.to_string(),
            interface_name: iface,
        })
    }

    #[cfg(target_os = "macos")]
    pub fn create_device(&self) -> Result<objc2::rc::Retained<objc2_foundation::NSObject>> {
        use objc2::rc::Retained;
        use objc2::{msg_send_id, sel};
        use objc2_foundation::{NSObject, NSString};
        use crate::bindings::*;

        let mac_str = format!(
            "06:00:{}:{}:{}:{}",
            rand_hex_byte(), rand_hex_byte(),
            rand_hex_byte(), rand_hex_byte(),
        );

        let device: Retained<NSObject> = unsafe {
            let vz_dev = VZVirtioNetworkDeviceConfiguration::init();
            let mac = msg_send_id![VZMACAddress::class(), alloc];

            let attachment = VZNATNetworkDeviceAttachment::init();
            vz_dev.setAttachment(&attachment);
            vz_dev.upcast()
        };

        Ok(device)
    }
}

fn rand_hex_byte() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u8;
    format!("{:02x}", (nanos.wrapping_mul(17) % 256))
}
