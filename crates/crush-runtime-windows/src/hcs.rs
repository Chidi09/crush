use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use libloading::Library;
use windows_sys::Win32::Foundation::LocalFree;
use crush_types::{Result, CrushError};

type HcsCreateComputeSystemFn = unsafe extern "system" fn(
    id: *const u16, config: *const u16,
    security: *const std::ffi::c_void,
    result: *mut *mut u16,
) -> i32;

type HcsOpenComputeSystemFn = unsafe extern "system" fn(
    id: *const u16,
    access: i32,
    system: *mut *mut std::ffi::c_void,
) -> i32;

type HcsStartComputeSystemFn = unsafe extern "system" fn(
    system: *mut std::ffi::c_void,
    options: *const u16,
    result: *mut *mut u16,
) -> i32;

type HcsCloseComputeSystemFn = unsafe extern "system" fn(
    system: *mut std::ffi::c_void,
) -> i32;

pub struct HcsManager {
    _lib: Library,
    create_sys: HcsCreateComputeSystemFn,
    open_sys: HcsOpenComputeSystemFn,
    start_sys: HcsStartComputeSystemFn,
    close_sys: HcsCloseComputeSystemFn,
}

impl HcsManager {
    pub fn load() -> anyhow::Result<Self> {
        unsafe {
            let library = Library::new("vmcompute.dll")?;
            // Dereference Symbol into a raw function pointer (Copy), which outlives `library`
            // because function pointers are just addresses into the loaded DLL. The DLL stays
            // loaded as long as `_lib` is alive in the struct.
            let create_sys: HcsCreateComputeSystemFn =
                *library.get::<HcsCreateComputeSystemFn>(b"HcsCreateComputeSystem\0")?;
            let open_sys: HcsOpenComputeSystemFn =
                *library.get::<HcsOpenComputeSystemFn>(b"HcsOpenComputeSystem\0")?;
            let start_sys: HcsStartComputeSystemFn =
                *library.get::<HcsStartComputeSystemFn>(b"HcsStartComputeSystem\0")?;
            let close_sys: HcsCloseComputeSystemFn =
                *library.get::<HcsCloseComputeSystemFn>(b"HcsCloseComputeSystem\0")?;
            Ok(Self {
                create_sys,
                open_sys,
                start_sys,
                close_sys,
                _lib: library,
            })
        }
    }

    pub fn create_compute_system(&self, container_id: &str, config_json: &str) -> Result<*mut std::ffi::c_void> {
        let id_wide: Vec<u16> = OsStr::new(container_id).encode_wide().chain(std::iter::once(0)).collect();
        let config_wide: Vec<u16> = OsStr::new(config_json).encode_wide().chain(std::iter::once(0)).collect();
        let mut error_result: *mut u16 = null_mut();
        let hresult = unsafe {
            (self.create_sys)(id_wide.as_ptr(), config_wide.as_ptr(), null_mut(), &mut error_result)
        };

        if hresult != 0 {
            if !error_result.is_null() {
                unsafe { LocalFree(error_result as _); }
            }
            return Err(CrushError::NamespaceError(format!(
                "HcsCreateComputeSystem failed: 0x{:x}", hresult
            )));
        }

        let mut system_handle: *mut std::ffi::c_void = null_mut();
        let open_result = unsafe {
            (self.open_sys)(id_wide.as_ptr(), 0xF0000, &mut system_handle)
        };

        if open_result != 0 || system_handle.is_null() {
            return Err(CrushError::NamespaceError(format!(
                "HcsOpenComputeSystem failed: 0x{:x}", open_result
            )));
        }

        Ok(system_handle)
    }

    pub fn start_compute_system(&self, system_handle: *mut std::ffi::c_void) -> Result<()> {
        let mut error_result: *mut u16 = null_mut();
        let hresult = unsafe {
            (self.start_sys)(system_handle, null_mut(), &mut error_result)
        };

        if hresult != 0 {
            if !error_result.is_null() {
                unsafe { LocalFree(error_result as _); }
            }
            return Err(CrushError::NamespaceError(format!(
                "HcsStartComputeSystem failed: 0x{:x}", hresult
            )));
        }

        Ok(())
    }

    pub fn close_compute_system(&self, system_handle: *mut std::ffi::c_void) {
        if !system_handle.is_null() {
            unsafe { (self.close_sys)(system_handle); }
        }
    }
}
