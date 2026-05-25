#![allow(non_camel_case_types, dead_code)]

use objc2::rc::Retained;
use objc2::{extern_class, extern_methods, msg_send_id, sel};
use objc2_foundation::{NSObject, NSString, NSURL};

// ── VZVirtualMachine ──────────────────────────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtualMachine;
    unsafe impl ClassType for VZVirtualMachine {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtualMachine {
        #[sel(initWithConfiguration:)]
        pub(crate) fn initWithConfiguration(&self, config: &VZVirtualMachineConfiguration) -> Retained<Self>;

        #[sel(startWithCompletionHandler:)]
        pub(crate) fn startWithCompletionHandler(&self, handler: &block2::Block<dyn Fn(*mut NSObject)>);

        #[sel(stopWithCompletionHandler:)]
        pub(crate) fn stopWithCompletionHandler(&self, handler: &block2::Block<dyn Fn(*mut NSObject)>);

        #[sel(pauseWithCompletionHandler:)]
        pub(crate) fn pauseWithCompletionHandler(&self, handler: &block2::Block<dyn Fn(*mut NSObject)>);

        #[sel(resumeWithCompletionHandler:)]
        pub(crate) fn resumeWithCompletionHandler(&self, handler: &block2::Block<dyn Fn(*mut NSObject)>);

        #[sel(requestStopWithError:)]
        pub(crate) fn requestStopWithError(&self) -> bool;

        #[sel(state)]
        pub(crate) fn state(&self) -> u64;
    }
);

// ── VZVirtualMachineConfiguration ─────────────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtualMachineConfiguration;
    unsafe impl ClassType for VZVirtualMachineConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtualMachineConfiguration {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;

        #[sel(setCPUCount:)]
        pub(crate) fn setCPUCount(&self, count: u64);

        #[sel(setMemorySize:)]
        pub(crate) fn setMemorySize(&self, size: u64);

        #[sel(setBootLoader:)]
        pub(crate) fn setBootLoader(&self, loader: &NSObject);

        #[sel(setMemoryBalloonDevices:)]
        pub(crate) fn setMemoryBalloonDevices(&self, devices: &NSObject);

        #[sel(setNetworkDevices:)]
        pub(crate) fn setNetworkDevices(&self, devices: &NSObject);

        #[sel(setStorageDevices:)]
        pub(crate) fn setStorageDevices(&self, devices: &NSObject);

        #[sel(setDirectorySharingDevices:)]
        pub(crate) fn setDirectorySharingDevices(&self, devices: &NSObject);

        #[sel(setSocketDevices:)]
        pub(crate) fn setSocketDevices(&self, devices: &NSObject);

        #[sel(setConsoleDevices:)]
        pub(crate) fn setConsoleDevices(&self, devices: &NSObject);

        #[sel(validateWithError:)]
        pub(crate) fn validateWithError(&self, error: *mut *mut NSObject) -> bool;
    }
);

// ── VZLinuxBootLoader ─────────────────────────────────────────────────────

extern_class!(
    pub(crate) struct VZLinuxBootLoader;
    unsafe impl ClassType for VZLinuxBootLoader {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZLinuxBootLoader {
        #[sel(initWithKernelURL:)]
        pub(crate) fn initWithKernelURL(&self, url: &NSURL) -> Retained<Self>;

        #[sel(setInitialRamdiskURL:)]
        pub(crate) fn setInitialRamdiskURL(&self, url: Option<&NSURL>);

        #[sel(setCommandLine:)]
        pub(crate) fn setCommandLine(&self, cmdline: &NSString);
    }
);

// ── VZMACAddress ─────────────────────────────────────────────────────────

extern_class!(
    pub(crate) struct VZMACAddress;
    unsafe impl ClassType for VZMACAddress {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZMACAddress {
        #[sel(allocWithZone:)]
        pub(crate) fn alloc() -> Retained<Self>;
    }
);

// ── VZVirtioNetworkDeviceConfiguration ────────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtioNetworkDeviceConfiguration;
    unsafe impl ClassType for VZVirtioNetworkDeviceConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtioNetworkDeviceConfiguration {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;

        #[sel(setAttachment:)]
        pub(crate) fn setAttachment(&self, attachment: &NSObject);

        #[sel(setMACAddress:)]
        pub(crate) fn setMACAddress(&self, mac: &VZMACAddress);
    }
);

// ── VZNATNetworkDeviceAttachment ─────────────────────────────────────────

extern_class!(
    pub(crate) struct VZNATNetworkDeviceAttachment;
    unsafe impl ClassType for VZNATNetworkDeviceAttachment {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZNATNetworkDeviceAttachment {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;
    }
);

// ── VZDiskImageStorageDeviceAttachment ───────────────────────────────────

extern_class!(
    pub(crate) struct VZDiskImageStorageDeviceAttachment;
    unsafe impl ClassType for VZDiskImageStorageDeviceAttachment {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZDiskImageStorageDeviceAttachment {
        #[sel(initWithURL:ofType:options:error:)]
        pub(crate) fn initWithURL(
            &self,
            url: &NSURL,
            of_type: u64,
            options: u64,
            error: *mut *mut NSObject,
        ) -> Option<Retained<Self>>;
    }
);

// ── VZVirtioBlockStorageDeviceConfiguration ──────────────────────────────

extern_class!(
    pub(crate) struct VZVirtioBlockStorageDeviceConfiguration;
    unsafe impl ClassType for VZVirtioBlockStorageDeviceConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtioBlockStorageDeviceConfiguration {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;

        #[sel(setAttachment:)]
        pub(crate) fn setAttachment(&self, attachment: &VZDiskImageStorageDeviceAttachment);
    }
);

// ── VZVirtioFileSystemDeviceConfiguration ────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtioFileSystemDeviceConfiguration;
    unsafe impl ClassType for VZVirtioFileSystemDeviceConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtioFileSystemDeviceConfiguration {
        #[sel(initWithTag:)]
        pub(crate) fn initWithTag(&self, tag: &NSString) -> Retained<Self>;

        #[sel(setDirectoryShare:)]
        pub(crate) fn setDirectoryShare(&self, share: &NSObject);
    }
);

// ── VZSharedDirectory ────────────────────────────────────────────────────

extern_class!(
    pub(crate) struct VZSharedDirectory;
    unsafe impl ClassType for VZSharedDirectory {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZSharedDirectory {
        #[sel(initWithURL:readOnly:)]
        pub(crate) fn initWithURL(&self, url: &NSURL, read_only: bool) -> Retained<Self>;
    }
);

// ── VZSingleDirectoryShare ───────────────────────────────────────────────

extern_class!(
    pub(crate) struct VZSingleDirectoryShare;
    unsafe impl ClassType for VZSingleDirectoryShare {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZSingleDirectoryShare {
        #[sel(initWithDirectory:)]
        pub(crate) fn initWithDirectory(&self, dir: &VZSharedDirectory) -> Retained<Self>;
    }
);

// ── VZVirtioSocketDeviceConfiguration ────────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtioSocketDeviceConfiguration;
    unsafe impl ClassType for VZVirtioSocketDeviceConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtioSocketDeviceConfiguration {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;

        #[sel(setListener:)]
        pub(crate) fn setListener(&self, listener: &NSObject);
    }
);

// ── VZVirtioConsoleDeviceConfiguration ───────────────────────────────────

extern_class!(
    pub(crate) struct VZVirtioConsoleDeviceConfiguration;
    unsafe impl ClassType for VZVirtioConsoleDeviceConfiguration {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZVirtioConsoleDeviceConfiguration {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;
    }
);

// ── VZLinuxRosettaDirectoryShare ─────────────────────────────────────────

extern_class!(
    pub(crate) struct VZLinuxRosettaDirectoryShare;
    unsafe impl ClassType for VZLinuxRosettaDirectoryShare {
        type Super = NSObject;
    }
);

extern_methods!(
    unsafe impl VZLinuxRosettaDirectoryShare {
        #[sel(init)]
        pub(crate) fn init() -> Retained<Self>;
    }
);

// ── Helper functions ──────────────────────────────────────────────────────

pub(crate) fn ns_url_from_path(path: &std::path::Path) -> Retained<NSURL> {
    let path_str = path.to_string_lossy();
    unsafe {
        NSURL::fileURLWithPath(&NSString::from_str(&path_str))
    }
}
