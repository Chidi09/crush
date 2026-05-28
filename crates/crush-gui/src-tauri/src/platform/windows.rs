use std::path::Path;

pub fn set_process_priority_class(priority: &str) {
    let class: u32 = match priority {
        "high" => 0x00000080,     // HIGH_PRIORITY_CLASS
        "realtime" => 0x00000100, // REALTIME_PRIORITY_CLASS
        "low" => 0x00004000,      // BELOW_NORMAL_PRIORITY_CLASS
        _ => 0x00000020,          // NORMAL_PRIORITY_CLASS
    };
    unsafe {
        let handle = windows_sys::Win32::System::Threading::GetCurrentProcess();
        windows_sys::Win32::System::Threading::SetPriorityClass(handle, class);
    }
}

pub fn set_console_ctrl_handler() {
    // Console handler setup for Windows — placeholder
}
