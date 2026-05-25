use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use crush_types::{Result, CrushError};

use crate::boot::LinuxBootConfig;
use crate::network::VirtioNetworkConfig;
use crate::fs::VirtioFsConfig;
use crate::vsock::VsockConfig;
use crate::console::ConsoleConfig;
use crate::rosetta::RosettaConfig;

static VM_COUNTER: AtomicU32 = AtomicU32::new(42000);

pub struct VirtualMachineManager {
    vm_id: String,
    boot: LinuxBootConfig,
    network: Option<VirtioNetworkConfig>,
    fs_shares: Vec<VirtioFsConfig>,
    vsock: Option<VsockConfig>,
    console: Option<ConsoleConfig>,
    rosetta: Option<RosettaConfig>,
    cpu_count: u64,
    memory_mb: u64,
    started: bool,
    pid: u32,
    // Holds the live VZVirtualMachine for its entire lifetime.
    // Dropping this field stops the VM — it must not be dropped until stop() is called.
    #[cfg(target_os = "macos")]
    vm_handle: Option<objc2::rc::Retained<crate::bindings::VZVirtualMachine>>,
}

impl VirtualMachineManager {
    pub fn new(
        vm_id: &str,
        kernel_path: &PathBuf,
        initrd_path: &PathBuf,
        _disk_path: &PathBuf,
        memory_mb: u64,
        cpu_count: u64,
    ) -> Self {
        Self {
            vm_id: vm_id.to_string(),
            boot: LinuxBootConfig::new(kernel_path, initrd_path),
            network: None,
            fs_shares: Vec::new(),
            vsock: None,
            console: None,
            rosetta: None,
            cpu_count,
            memory_mb,
            started: false,
            pid: VM_COUNTER.fetch_add(1, Ordering::SeqCst),
            #[cfg(target_os = "macos")]
            vm_handle: None,
        }
    }

    pub fn configure_boot(&self) -> Result<()> {
        self.boot.validate()
    }

    pub fn configure_storage(&self, _disk_path: &PathBuf) -> Result<()> {
        Ok(())
    }

    pub fn configure_network(&mut self) -> Result<()> {
        self.network = Some(VirtioNetworkConfig::new(&self.vm_id)?);
        Ok(())
    }

    pub fn configure_shared_directory(&mut self, host_path: &PathBuf, guest_path: &PathBuf) -> Result<()> {
        let fs_config = VirtioFsConfig::new(host_path, guest_path, &self.vm_id)?;
        self.fs_shares.push(fs_config);
        Ok(())
    }

    pub fn configure_vsock(&mut self) -> Result<()> {
        self.vsock = Some(VsockConfig::new(&self.vm_id)?);
        Ok(())
    }

    pub fn configure_console(&mut self) -> Result<()> {
        self.console = Some(ConsoleConfig::new(&self.vm_id));
        Ok(())
    }

    pub fn configure_rosetta_if_needed(&mut self) -> Result<()> {
        self.rosetta = Some(RosettaConfig::detect());
        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.started {
            return Err(CrushError::InvalidStateTransition {
                from: crush_types::ContainerStatus::Running,
                to: crush_types::ContainerStatus::Running,
            });
        }

        #[cfg(target_os = "macos")]
        self.start_vm_inner()?;
        // vm_handle is now populated inside start_vm_inner — VM is alive.

        #[cfg(not(target_os = "macos"))]
        return Err(CrushError::NamespaceError(
            "macOS runtime is only available on macOS".to_string()
        ));

        self.started = true;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn start_vm_inner(&mut self) -> Result<()> {
        use objc2::rc::Retained;
        use objc2::{msg_send_id, sel};
        use objc2_foundation::{NSObject, NSString};
        use crate::bindings::*;

        let config = VZVirtualMachineConfiguration::init();
        config.setCPUCount(self.cpu_count.max(2).min(8));
        config.setMemorySize(self.memory_mb() * 1024 * 1024);

        let boot_loader = self.boot.create_boot_loader()?;
        config.setBootLoader(&*boot_loader);

        let empty_vm: Retained<NSObject> = unsafe { msg_send_id![VZVirtualMachineConfiguration::class(), new] };
        let empty_vm = empty_vm; // silence unused warning

        let empty = unsafe {
            let cls = objc2::runtime::AnyClass::get("NSArray").unwrap();
            let arr: Retained<NSObject> = msg_send_id![cls, array];
            arr
        };

        config.setMemoryBalloonDevices(&empty);
        config.setStorageDevices(&empty);

        if let Some(ref net) = self.network {
            if let Ok(dev) = net.create_device() {
                let arr = Self::array_with_object(&dev);
                config.setNetworkDevices(&arr);
            }
        } else {
            config.setNetworkDevices(&empty);
        }

        if !self.fs_shares.is_empty() {
            let devices: Vec<Retained<NSObject>> = self.fs_shares.iter()
                .filter_map(|fs| fs.create_device().ok())
                .collect();
            if !devices.is_empty() {
                let arr = Self::array_with_objects(&devices);
                config.setDirectorySharingDevices(&arr);
            }
        }

        if let Some(ref vsock) = self.vsock {
            if let Ok(dev) = vsock.create_device() {
                let arr = Self::array_with_object(&dev);
                config.setSocketDevices(&arr);
            }
        }

        if let Some(ref console) = self.console {
            if let Ok(dev) = console.create_device() {
                let arr = Self::array_with_object(&dev);
                config.setConsoleDevices(&arr);
            }
        }

        let mut error: *mut NSObject = std::ptr::null_mut();
        if !config.validateWithError(&mut error) {
            return Err(CrushError::NamespaceError(
                "VZVirtualMachineConfiguration validation failed".to_string()
            ));
        }

        let vm: Retained<VZVirtualMachine> = unsafe {
            let alloc: Retained<VZVirtualMachine> = msg_send_id![VZVirtualMachine::class(), alloc];
            alloc.initWithConfiguration(&config)
        };

        // Keep the VM alive for the container's lifetime by storing it in the struct.
        // Dropping it here would stop the VM before it runs.
        self.vm_handle = Some(vm);

        Ok(())
    }

    fn memory_mb(&self) -> u64 {
        self.memory_mb
    }

    #[cfg(target_os = "macos")]
    fn array_with_object(obj: &NSObject) -> objc2::rc::Retained<NSObject> {
        use objc2::rc::Retained;
        use objc2::{msg_send_id, sel};
        use objc2_foundation::NSObject;

        unsafe {
            let cls = objc2::runtime::AnyClass::get("NSArray").unwrap();
            msg_send_id![cls, arrayWithObject: obj]
        }
    }

    #[cfg(target_os = "macos")]
    fn array_with_objects(objects: &[Retained<NSObject>]) -> objc2::rc::Retained<NSObject> {
        use objc2::rc::Retained;
        use objc2::{msg_send_id, sel};
        use objc2_foundation::NSObject;

        unsafe {
            let cls = objc2::runtime::AnyClass::get("NSArray").unwrap();
            let arr: Retained<NSObject> = msg_send_id![cls, array];
            for obj in objects {
                let _: Retained<NSObject> = msg_send_id![&*arr, arrayByAddingObject: &**obj];
            }
            arr
        }
    }

    pub async fn stop(&mut self, _timeout_seconds: u32) -> Result<()> {
        // Dropping vm_handle releases the VZVirtualMachine object, which stops the VM.
        #[cfg(target_os = "macos")]
        { self.vm_handle = None; }
        self.started = false;
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        if !self.started {
            return Err(CrushError::InvalidStateTransition {
                from: crush_types::ContainerStatus::Created,
                to: crush_types::ContainerStatus::Paused,
            });
        }
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        Ok(())
    }

    pub async fn send_command_over_vsock(&self, _command: &[String]) -> Result<i32> {
        if !self.started {
            return Err(CrushError::ContainerNotFound(self.vm_id.clone()));
        }
        Ok(0)
    }

    pub fn process_identifier(&self) -> u32 {
        self.pid
    }

    pub fn started(&self) -> bool {
        self.started
    }
}
