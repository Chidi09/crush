use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null;
use windows_sys::Win32::System::Services::{
    StartServiceCtrlDispatcherW, RegisterServiceCtrlHandlerExW,
    SERVICE_TABLE_ENTRYW, SERVICE_STATUS,
    SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STOPPED,
};
use crush_types::{Result, CrushError};

pub struct WindowsService {
    service_name: String,
}

impl WindowsService {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    pub fn start_dispatcher(&self) -> Result<()> {
        println!("SCM: Registering Service Control dispatcher table for service: {}", self.service_name);
        
        let service_name_wide: Vec<u16> = OsStr::new(&self.service_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Register the SCM service main callback function
        let dispatch_table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: service_name_wide.as_ptr() as *mut _,
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW {
                lpServiceName: std::ptr::null_mut(),
                lpServiceProc: None,
            },
        ];

        let success = unsafe {
            StartServiceCtrlDispatcherW(dispatch_table.as_ptr())
        };

        if success == 0 {
            let err = std::io::Error::last_os_error();
            // ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063) is returned when run directly from CLI
            if err.raw_os_error() == Some(1063) {
                println!("SCM: Running in standard terminal console mode instead of SCM daemon");
            } else {
                return Err(CrushError::NamespaceError(format!(
                    "Failed to start service dispatcher: {}",
                    err
                )));
            }
        }

        Ok(())
    }
}

unsafe extern "system" fn service_main(_dw_num_services_args: u32, _lp_service_arg_vectors: *mut *mut u16) {
    println!("SCM: service_main dispatcher activated by the Windows Service Control Manager");
}
