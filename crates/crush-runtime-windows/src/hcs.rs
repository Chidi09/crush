use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use libloading::{Library, Symbol};
use crush_types::{Result, CrushError};

type HcsCreateComputeSystemFn = unsafe extern "system" fn(
    id: *const u16, config: *const u16,
    security: *const std::ffi::c_void,
    result: *mut *mut u16, // error result, NOT the handle
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
    create_sys: Symbol<'static, HcsCreateComputeSystemFn>,
    open_sys: Symbol<'static, HcsOpenComputeSystemFn>,
    start_sys: Symbol<'static, HcsStartComputeSystemFn>,
    close_sys: Symbol<'static, HcsCloseComputeSystemFn>,
}

impl HcsManager {
    pub fn load() -> anyhow::Result<Self> {
        unsafe {
            let library = Library::new("vmcompute.dll")?;
            Ok(Self {
                create_sys: library.get(b"HcsCreateComputeSystem")?,
                open_sys: library.get(b"HcsOpenComputeSystem")?,
                start_sys: library.get(b"HcsStartComputeSystem")?,
                close_sys: library.get(b"HcsCloseComputeSystem")?,
                _lib: library,
            })
        }
    }

    pub fn create_compute_system(&self, container_id: &str, config_json: &str) -> Result<*mut std::ffi::c_void> {
        let id_wide: Vec<u16> = OsStr::new(container_id).encode_wide().chain(std::iter::once(0)).collect();
        let config_wide: Vec<u16> = OsStr::new(config_json).encode_wide().chain(std::iter::once(0)).collect();
        let mut error_result: *mut u16 = null_mut();
        // ⚠ FIX: The compute system handle is returned in the return value (HRESULT),
        // we need to open it separately to get a valid handle
        let hresult = unsafe {
            (self.create_sys)(id_wide.as_ptr(), config_wide.as_ptr(), null_mut(), &mut error_result)
        };

        if hresult != 0 {
            if !error_result.is_null() {
                unsafe { windows_sys::Win32::System::SystemServices::LocalFree(error_result as _); }
            }
            return Err(CrushError::NamespaceError(format!(
                "HcsCreateComputeSystem failed: 0x{:x}", hresult
            )));
        }

        // Open the compute system to get a real handle
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
                unsafe { windows_sys::Win32::System::SystemServices::LocalFree(error_result as _); }
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
